pub mod boolean;
mod string;

use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use vizual_macros::display;

use super::{
    super::{
        Focus_provider, Shared_widget, Widget, Widget_trait, custom_widget::Custom_widget_trait,
    },
    button::Button,
    layout::axis::Axis,
    positioning::anchor::Anchor,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    event::{Key_code, Key_event, Pointer_event},
    geometry::Direction,
    handlers::Retrieve_handler,
    layouter::{hitbox::Hitbox, variable::Variable},
    slot::manager::Slots,
    state::{State, Store},
    sync::{Mutex, Thread_safe},
    theme::Theme,
    utils::{get_next_index, get_previous_index},
    widget::custom_widget::Selector,
};

// This trait is used as a trait object, which trait aliases do not currently support.
pub trait Menu_item_trait<Choice: Thread_safe>:
    Custom_widget_trait<Payload = bool> + Retrieve_handler<Choice>
{
}
impl<Choice: Thread_safe, Widget> Menu_item_trait<Choice> for Widget where
    Widget: Custom_widget_trait<Payload = bool> + Retrieve_handler<Choice>
{
}

pub type Menu_item<Choice> = Box<dyn Menu_item_trait<Choice>>;
pub type Menu_item_selector<Choice> = Selector<dyn Menu_item_trait<Choice>>;

impl<Choice, Widget> From<Shared_widget<Widget>> for Menu_item<Choice>
where
    Choice: Thread_safe,
    Widget: Menu_item_trait<Choice>,
{
    fn from(value: Shared_widget<Widget>) -> Self {
        let inner: Arc<Mutex<dyn Menu_item_trait<Choice>>> = value.0;
        Shared_widget(inner)
    }
}

pub fn get_selector<Choice: Thread_safe>(
    item: &Menu_item<Choice>,
) -> Menu_item_selector<Choice> {
    item.as_reference()
}

#[derive(Clone)]
struct Menu_item<Choice: Thread_safe> {
    selected: bool,
    widget: Menu_item<Choice>,
    menu_selector: Store<Menu_item_selector<Choice>>,
    submitted: Store<Choice>,
    button_delta: Variable,
}

#[derive(Clone)]
struct Menu_item_content<Choice: Thread_safe + Clone> {
    selected: bool,
    widget: Menu_item<Choice>,
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu_item_content<Choice> {
    async fn layout(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let contents = self
            .widget
            .lock()
            .await?
            .layout(
                render,
                theme,
                focus,
                hitbox,
                parent,
                problem,
                text_context,
                slots,
                self.selected,
            )
            .await?;

        // TODO: handle this some other way
        if contents.len() != 1 {
            return Err(eyre!(
                "Menu item layout must return exactly one child, got {}",
                contents.len()
            ));
        }
        Ok(contents)
    }
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu_item<Choice> {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let content = Menu_item_content {
            selected: self.selected,
            widget: self.widget.clone(),
        };
        let mut button = Button::around(content);
        button.highlighted = self.selected;
        button.focusable = true;
        button.delta = Some(self.button_delta.clone());
        let button = Anchor::left(button);

        Ok(vec![display!(button)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        *self.menu_selector.write().await? = get_selector(&self.widget);
        *self.submitted.write().await? = self.widget.lock().await?.on_retrieve().await?;
        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[derive(Clone)]
pub struct Menu<Choice: Thread_safe> {
    items: Vec<Menu_item<Choice>>,
    selected: Store<Menu_item_selector<Choice>>,
    pub submitted: Store<Choice>,
    default_item: Menu_item_selector<Choice>,
}

impl<Choice: Thread_safe + Clone> Menu<Choice> {
    pub async fn new(
        items: Vec<Menu_item<Choice>>,
        default_item: Menu_item_selector<Choice>,
    ) -> Result<Self> {
        let default_choice = default_item
            .upgrade()
            .ok_or_else(|| eyre!("Default menu item selector is stale"))?
            .lock()
            .await?
            .on_retrieve()
            .await?;

        Ok(Self {
            items,
            selected: Store::new(default_item.clone()),
            submitted: Store::new(default_choice),
            default_item,
        })
    }

    fn find_selected_item(
        &self,
        selected: &Menu_item_selector<Choice>,
    ) -> Result<Menu_item<Choice>> {
        let selected = selected
            .upgrade()
            .ok_or_else(|| eyre!("Selected menu item selector is stale"))?;

        self.items
            .iter()
            .find(|item| item.compare_reference(&selected))
            .cloned()
            .ok_or_else(|| eyre!("Selected menu item is not in the menu"))
    }

    async fn get_selected_item(&self) -> Result<Menu_item<Choice>> {
        let selected = self.selected.read().await?;
        self.find_selected_item(&selected)
    }

    async fn get_affected_selected_item(
        &self,
        render: crate::Render,
    ) -> Result<Menu_item<Choice>> {
        let selected = self.selected.affect(render).await?;
        self.find_selected_item(&selected)
    }

    async fn get_selected_index(&self) -> Result<usize> {
        let selected = self.get_selected_item().await?;
        self.items
            .iter()
            .position(|item| item.compare(&selected))
            .ok_or_else(|| eyre!("Selected menu item is not in the menu"))
    }

    async fn set_index(&self, index: usize) -> Result<()> {
        let item = self
            .items
            .get(index)
            .ok_or_else(|| eyre!("Menu item index {index} is out of range"))?;
        *self.selected.write().await? = get_selector(item);
        *self.submitted.write().await? = item.lock().await?.on_retrieve().await?;
        Ok(())
    }
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Retrieve_handler<Choice> for Menu<Choice> {
    async fn on_retrieve(&mut self) -> Result<Choice> {
        Ok(self.submitted.read().await?.clone())
    }
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu<Choice> {
    async fn layout(
        &mut self,
        render: crate::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let default_item = self
            .default_item
            .upgrade()
            .ok_or_else(|| eyre!("Default menu item selector is stale"))?;
        if !self
            .items
            .iter()
            .any(|item| item.compare_reference(&default_item))
        {
            return Err(eyre!("Default menu item is not in the menu"));
        }

        let selected = self.get_affected_selected_item(render).await?;
        let mut rows: Vec<Widget> = Vec::with_capacity(self.items.len());
        let button_delta = problem.add_delta("menu-item-button-delta", 1).await?;

        for item in &self.items {
            let item = Menu_item {
                selected: item.compare(&selected),
                widget: item.clone(),
                menu_selector: self.selected.clone(),
                submitted: self.submitted.clone(),
                button_delta: button_delta.clone(),
            };
            let item = Anchor::left(item);
            rows.push(Box::new(item));
        }

        Ok(vec![display!(Axis::new(Direction::Vertical, rows,))])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up | Key_code::Arrow_down => {
                let index = self.get_selected_index().await?;
                let index = match key.code {
                    Key_code::Arrow_up => get_previous_index(self.items.len(), index),
                    _ => get_next_index(self.items.len(), index),
                };
                self.set_index(index).await?;
                Vizual_msg::new(Vizual_command::Layout)
            }
            _ => Vizual_msg::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Ordinary_menu_item(usize);

    #[async_trait]
    impl Widget_trait for Ordinary_menu_item {
        async fn layout(
            &mut self,
            _render: crate::Render,
            _theme: Store<Theme>,
            _focus: &mut Focus_provider,
            _hitbox: &mut Hitbox,
            _parent: Hitbox,
            _problem: Component_context,
            _text_context: &mut crate::graphics::text::Text_context,
            _slots: &mut Slots,
        ) -> Result<Children> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl Retrieve_handler<usize> for Ordinary_menu_item {
        async fn on_retrieve(&mut self) -> Result<usize> {
            Ok(self.0)
        }
    }

    #[test]
    fn ordinary_widgets_automatically_satisfy_menu_item_trait() {
        fn assert_menu_item<T: Menu_item_trait<usize>>() {}

        assert_menu_item::<Ordinary_menu_item>();
    }

    #[test]
    fn selectors_preserve_menu_item_identity() {
        let item: Menu_item<usize> =
            Custom_widget_trait::into_shared(Ordinary_menu_item(0)).into();
        let selected = get_selector(&item)
            .upgrade()
            .expect("menu item should still be alive");

        assert!(item.compare_reference(&selected));
    }

    #[test]
    fn selectors_become_stale_after_the_menu_item_is_dropped() {
        let selector = {
            let item: Menu_item<usize> =
                Custom_widget_trait::into_shared(Ordinary_menu_item(0)).into();
            get_selector(&item)
        };

        assert!(selector.upgrade().is_none());
    }

    #[tokio::test]
    async fn menu_initializes_and_submits() -> Result<()> {
        let first: Menu_item<usize> =
            Custom_widget_trait::into_shared(Ordinary_menu_item(0)).into();
        let second: Menu_item<usize> =
            Custom_widget_trait::into_shared(Ordinary_menu_item(1)).into();
        let menu = Menu::new(vec![first.clone(), second.clone()], get_selector(&first)).await?;
        assert_eq!(*menu.submitted.read().await?, 0);
        Ok(())
    }
}
