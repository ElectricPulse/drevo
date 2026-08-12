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
    positioning::anchor::{Anchor, Anchors},
};
use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    display::Display,
    event::{Key_code, Key_event, Pointer_event},
    geometry::{Direction, Rect},
    handlers::Retrieve_handler,
    layouter::{hitbox::Hitbox, variable::Variable},
    slot::manager::Slots,
    state::State,
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

pub type Shared_menu_item<Choice> = Shared_widget<dyn Menu_item_trait<Choice>>;
pub type Menu_item_selector<Choice> = Selector<dyn Menu_item_trait<Choice>>;

impl<Choice, Widget> From<Shared_widget<Widget>> for Shared_menu_item<Choice>
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
    item: &Shared_menu_item<Choice>,
) -> Menu_item_selector<Choice> {
    item.as_reference()
}

#[derive(Clone)]
struct Menu_item<Choice: Thread_safe> {
    selected: bool,
    widget: Shared_menu_item<Choice>,
    menu_selector: State<Menu_item_selector<Choice>>,
    button_delta: Variable,
    submission: Submission<Choice>,
}

#[derive(Clone)]
struct Menu_item_content<Choice: Thread_safe + Clone> {
    selected: bool,
    widget: Shared_menu_item<Choice>,
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu_item_content<Choice> {
    async fn layout(
        &mut self,
        render: crate::Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
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

#[derive(Clone)]
enum Submission<Choice> {
    None,
    State(State<Choice>),
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu_item<Choice> {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let content = Menu_item_content {
            selected: self.selected,
            widget: self.widget.clone(),
        };
        let mut button = Button::around(Anchor::new(content, Anchors::left()));
        button.highlighted = self.selected;
        button.delta = Some(self.button_delta.clone());
        Ok(vec![display!(button)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        self.menu_selector.store(get_selector(&self.widget));

        if let Submission::State(state) = &self.submission {
            state.set(self.widget.lock().await?.on_retrieve().await?);
        }

        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[derive(Clone)]
pub struct Menu<Choice: Thread_safe> {
    items: Vec<Shared_menu_item<Choice>>,
    pub selected: State<Menu_item_selector<Choice>>,
    default_item: Menu_item_selector<Choice>,
    submission: Submission<Choice>,
}

impl<Choice: Thread_safe> Menu<Choice> {
    pub fn new(
        items: Vec<Shared_menu_item<Choice>>,
        default_item: Menu_item_selector<Choice>,
        render: crate::Render,
    ) -> Self {
        Self {
            items,
            selected: render.new_state(default_item.clone()),
            default_item,
            submission: Submission::None,
        }
    }

    pub fn set_submit_state(&mut self, state: State<Choice>) {
        self.submission = Submission::State(state);
    }

    fn get_selected_item(&self) -> Result<Shared_menu_item<Choice>> {
        let selected = self
            .selected
            .load()
            .upgrade()
            .ok_or_else(|| eyre!("Selected menu item selector is stale"))?;

        self.items
            .iter()
            .find(|item| item.compare_reference(&selected))
            .cloned()
            .ok_or_else(|| eyre!("Selected menu item is not in the menu"))
    }

    fn get_selected_index(&self) -> Result<usize> {
        let selected = self.get_selected_item()?;
        self.items
            .iter()
            .position(|item| item.compare(&selected))
            .ok_or_else(|| eyre!("Selected menu item is not in the menu"))
    }

    fn set_index(&self, index: usize) -> Result<()> {
        let item = self
            .items
            .get(index)
            .ok_or_else(|| eyre!("Menu item index {index} is out of range"))?;
        self.selected.store(get_selector(item));
        Ok(())
    }
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Retrieve_handler<Choice> for Menu<Choice> {
    async fn on_retrieve(&mut self) -> Result<Choice> {
        let item = self.get_selected_item()?;
        let choice = item.lock().await?.on_retrieve().await?;
        Ok(choice)
    }
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu<Choice> {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
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

        let selected = self.get_selected_item()?;
        let mut rows: Vec<Widget> = Vec::with_capacity(self.items.len());
        let button_delta = problem.add_delta("menu-item-button-delta", 2).await?;

        for item in &self.items {
            let item = Menu_item {
                selected: item.compare(&selected),
                widget: item.clone(),
                menu_selector: self.selected.clone(),
                button_delta: button_delta.clone(),
                submission: self.submission.clone(),
            };
            let item = Anchor::new(item, Anchors::left());
            rows.push(Box::new(item));
        }

        Ok(vec![display!(Axis::new(Direction::Vertical, rows,))])
    }

    async fn render(
        &mut self,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        Ok(None)
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up | Key_code::Arrow_down => {
                let index = self.get_selected_index()?;
                let index = match key.code {
                    Key_code::Arrow_up => get_previous_index(self.items.len(), index),
                    _ => get_next_index(self.items.len(), index),
                };
                self.set_index(index)?;
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
    struct Ordinary_menu_item;

    #[async_trait]
    impl Widget_trait for Ordinary_menu_item {
        async fn layout(
            &mut self,
            _render: crate::Render,
            _theme: State<Theme>,
            _focus: &mut Focus_provider,
            _hitbox: &mut Hitbox,
            _parent: Hitbox,
            _problem: Component_context,
            _text_context: &mut crate::text::Text_context,
            _slots: &mut Slots,
        ) -> Result<Children> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl Retrieve_handler<usize> for Ordinary_menu_item {
        async fn on_retrieve(&mut self) -> Result<usize> {
            Ok(0)
        }
    }

    #[test]
    fn ordinary_widgets_automatically_satisfy_menu_item_trait() {
        fn assert_menu_item<T: Menu_item_trait<usize>>() {}

        assert_menu_item::<Ordinary_menu_item>();
    }

    #[test]
    fn selectors_preserve_menu_item_identity() {
        let item: Shared_menu_item<usize> =
            Custom_widget_trait::into_shared(Ordinary_menu_item).into();
        let selected = get_selector(&item)
            .upgrade()
            .expect("menu item should still be alive");

        assert!(item.compare_reference(&selected));
    }

    #[test]
    fn selectors_become_stale_after_the_menu_item_is_dropped() {
        let selector = {
            let item: Shared_menu_item<usize> =
                Custom_widget_trait::into_shared(Ordinary_menu_item).into();
            get_selector(&item)
        };

        assert!(selector.upgrade().is_none());
    }
}
