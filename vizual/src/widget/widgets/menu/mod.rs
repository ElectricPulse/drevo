pub mod boolean;
mod string;

use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use vizual_macros::display;

use super::{
    super::{
        Focus_provider, Widget, Widget_trait,
        custom_widget::{Custom_widget, Custom_widget_trait},
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
    sync::Thread_safe,
    theme::Theme,
    utils::{get_next_index, get_previous_index},
};

// This trait is used as a trait object, which trait aliases do not currently support.
pub trait Menu_item_trait<Choice: Thread_safe>:
    Custom_widget_trait<Payload = bool> + Retrieve_handler<Choice> + dyn_clone::DynClone
{
}
impl<Choice: Thread_safe, Widget> Menu_item_trait<Choice> for Widget where
    Widget: Custom_widget_trait<Payload = bool> + Retrieve_handler<Choice> + Clone
{
}

dyn_clone::clone_trait_object!(<Choice> Menu_item_trait<Choice> where Choice: Thread_safe);

pub type Menu_item<Choice> = Box<dyn Menu_item_trait<Choice>>;

#[derive(Clone)]
struct Menu_item_container<Choice: Thread_safe + Clone> {
    index: usize,
    selected: bool,
    widget: Menu_item<Choice>,
    selected_store: Store<usize>,
    submitted: Store<Choice>,
    button_delta: Variable,
}

impl<Choice: Thread_safe + Clone> Menu_item_container<Choice> {
    async fn submit(&mut self) -> Result<Vizual_msg> {
        *self.selected_store.write().await? = self.index;
        *self.submitted.write().await? = self.widget.on_retrieve().await?;
        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[async_trait]
impl<Choice: Thread_safe + Clone> Widget_trait for Menu_item_container<Choice> {
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
        _root: &crate::component::Shared_component,
    ) -> Result<Children> {
        let content = Custom_widget::new(self.widget.clone(), self.selected);
        let mut button = Button::around(content);
        button.highlighted = self.selected;
        button.focusable = true;
        button.delta = Some(self.button_delta.clone());
        let button = Anchor::left(button);

        Ok(vec![display!(button)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        self.submit().await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        if matches!(key.code, Key_code::Enter) {
            return self.submit().await;
        }

        Vizual_msg::none()
    }
}

#[derive(Clone)]
pub struct Menu<Choice: Thread_safe> {
    items: Vec<Menu_item<Choice>>,
    // Note: For now there is no reordering or filtering of the items, but even if that were needed
    // a separate list of indices could be created for that. What is important for now and into the future:
    // there is no need for a menu where you want the items to persist yet change the underlying widget
    // (i.e. change items at runtime) — in that case an extra wrapper around the items would have to be passed
    // identifying their ID.
    pub selected: Store<usize>,
    pub submitted: Store<Choice>,
}

impl<Choice: Thread_safe + Clone> Menu<Choice> {
    pub async fn new(mut items: Vec<Menu_item<Choice>>, default_item: usize) -> Result<Self> {
        let item = items
            .get_mut(default_item)
            .ok_or_else(|| eyre!("Default menu item index {default_item} is out of range"))?;
        let default_choice = item.on_retrieve().await?;

        Ok(Self {
            items,
            selected: Store::new(default_item),
            submitted: Store::new(default_choice),
        })
    }

    pub async fn set_index(&mut self, index: usize) -> Result<()> {
        let item = self
            .items
            .get_mut(index)
            .ok_or_else(|| eyre!("Menu item index {index} is out of range"))?;
        let value = item.on_retrieve().await?;
        *self.selected.write().await? = index;
        *self.submitted.write().await? = value;
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
        _root: &crate::component::Shared_component,
    ) -> Result<Children> {
        let selected = *self.selected.affect(render).await?;
        let mut rows: Vec<Widget> = Vec::with_capacity(self.items.len());
        let button_delta = problem.add_delta("menu-item-button-delta", 1).await?;

        for (index, item) in self.items.iter().enumerate() {
            let row = Menu_item_container {
                index,
                selected: index == selected,
                widget: item.clone(),
                selected_store: self.selected.clone(),
                submitted: self.submitted.clone(),
                button_delta: button_delta.clone(),
            };
            rows.push(Box::new(row));
        }

        Ok(vec![display!(Axis::new(Direction::Vertical, rows))])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        match key.code {
            Key_code::Arrow_up | Key_code::Arrow_down => {
                let index = *self.selected.read().await?;
                let next_index = match key.code {
                    Key_code::Arrow_up => get_previous_index(self.items.len(), index),
                    _ => get_next_index(self.items.len(), index),
                };
                self.set_index(next_index).await?;
                Vizual_msg::new(Vizual_command::Layout)
            }
            _ => Vizual_msg::none(),
        }
    }
}

#[cfg(test)]
mod tests;
