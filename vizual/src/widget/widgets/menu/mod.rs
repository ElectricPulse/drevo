pub mod boolean;
mod string;

use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};

use super::{
    super::{
        LayoutInput, Widget, WidgetTrait,
        custom_widget::{CustomWidget, CustomWidgetTrait},
    },
    button::Button,
    layout::axis::Axis,
    positioning::anchor::Anchor,
};
use crate::{
    VizualMsg,
    component::Children,
    event::KeyCode,
    geometry::Direction,
    handlers::RetrieveHandler,
    layouter::variable::Variable,
    state::{State, Store},
    sync::ThreadSafe,
    utils::{get_next_index, get_previous_index},
};

// This trait is used as a trait object, which trait aliases do not currently support.
pub trait MenuItemTrait<Choice: ThreadSafe>:
    CustomWidgetTrait<Payload = bool> + RetrieveHandler<Choice> + dyn_clone::DynClone
{
}
impl<Choice: ThreadSafe, Widget> MenuItemTrait<Choice> for Widget where
    Widget: CustomWidgetTrait<Payload = bool> + RetrieveHandler<Choice> + Clone
{
}

dyn_clone::clone_trait_object!(<Choice> MenuItemTrait<Choice> where Choice: ThreadSafe);

pub type MenuItem<Choice> = Box<dyn MenuItemTrait<Choice>>;

struct MenuItemContainer<Choice: ThreadSafe> {
    index: usize,
    selected: bool,
    widget: MenuItem<Choice>,
    selected_store: Store<usize>,
    submitted: Store<Choice>,
    button_delta: Variable,
    item_block: bool,
}

impl<Choice: ThreadSafe> Clone for MenuItemContainer<Choice> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            selected: self.selected,
            widget: self.widget.clone(),
            selected_store: self.selected_store.clone(),
            submitted: self.submitted.clone(),
            button_delta: self.button_delta.clone(),
            item_block: self.item_block,
        }
    }
}

impl<Choice: ThreadSafe> MenuItemContainer<Choice> {
    async fn submit(&mut self, relayout: crate::Signal) -> Result<VizualMsg> {
        self.selected_store.set(self.index).await?;
        self.submitted.set(self.widget.on_retrieve().await?).await?;
        relayout.send();
        VizualMsg::none()
    }
}

#[async_trait]
impl<Choice: ThreadSafe> WidgetTrait for MenuItemContainer<Choice> {
    async fn layout(&mut self, LayoutInput { slots, .. }: LayoutInput<'_>) -> Result<Children> {
        let content = CustomWidget::new(self.widget.clone(), self.selected);
        let widget: Widget = match self.item_block {
            true => {
                let mut button = Button::around(content);
                button.highlighted = self.selected;
                button.delta = Some(self.button_delta.clone());
                Box::new(Anchor::left(button))
            }
            false => Box::new(Anchor::left(content)),
        };

        Ok(vec![display!(widget)])
    }

    async fn on_mouse_click(
        &mut self,
        input: crate::widget::MouseEvent<'_>,
    ) -> Result<VizualMsg> {
        self.submit(input.relayout).await
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<VizualMsg> {
        let key = input.key;
        if matches!(key.code, KeyCode::Enter) {
            return self.submit(input.relayout).await;
        }

        VizualMsg::none()
    }
}

pub struct Menu<Choice: ThreadSafe> {
    items: Vec<MenuItem<Choice>>,
    pub selected: Store<usize>,
    submitted: Store<Choice>,
    pub item_block: bool,
}

impl<Choice: ThreadSafe> Clone for Menu<Choice> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            selected: self.selected.clone(),
            submitted: self.submitted.clone(),
            item_block: self.item_block,
        }
    }
}

impl<Choice: ThreadSafe> Menu<Choice> {
    pub async fn new(mut items: Vec<MenuItem<Choice>>, default_item: usize) -> Result<Self> {
        let item = items
            .get_mut(default_item)
            .ok_or_else(|| eyre!("Default menu item index {default_item} is out of range"))?;
        let default_choice = item.on_retrieve().await?;

        Ok(Self {
            items,
            selected: Store::new(default_item),
            submitted: Store::new(default_choice),
            item_block: true,
        })
    }

    pub async fn set_index(&mut self, index: usize) -> Result<()> {
        let item = self
            .items
            .get_mut(index)
            .ok_or_else(|| eyre!("Menu item index {index} is out of range"))?;
        let value = item.on_retrieve().await?;
        self.selected.set(index).await?;
        self.submitted.set(value).await?;
        Ok(())
    }

    pub async fn set_submitted(&mut self, store: Store<Choice>) -> Result<()> {
        self.submitted = store;
        Ok(())
    }
}

#[async_trait]
impl<Choice: ThreadSafe> RetrieveHandler<Choice> for Menu<Choice> {
    async fn on_retrieve(&mut self) -> Result<State<Choice>> {
        Ok(self.submitted.clone().into())
    }
}

#[async_trait]
impl<Choice: ThreadSafe> WidgetTrait for Menu<Choice> {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let selected = *self.selected.affect(relayout).await?;
        let mut rows: Vec<Widget> = Vec::with_capacity(self.items.len());
        let button_delta = problem.add_delta("menu-item-button-delta", 1).await?;

        for (index, item) in self.items.iter().enumerate() {
            let row = MenuItemContainer {
                index,
                selected: index == selected,
                widget: item.clone(),
                selected_store: self.selected.clone(),
                submitted: self.submitted.clone(),
                button_delta: button_delta.clone(),
                item_block: self.item_block,
            };
            rows.push(row.as_any());
        }

        Ok(vec![display!(Axis::new(Direction::Vertical, rows))])
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<VizualMsg> {
        let key = input.key;
        let relayout = input.relayout;
        match key.code {
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                let index = *self.selected.read().await?;
                let next_index = match key.code {
                    KeyCode::ArrowUp => get_previous_index(self.items.len(), index),
                    _ => get_next_index(self.items.len(), index),
                };
                self.set_index(next_index).await?;
                relayout.send();
                VizualMsg::none()
            }
            _ => VizualMsg::none(),
        }
    }
}

#[cfg(test)]
mod tests;
