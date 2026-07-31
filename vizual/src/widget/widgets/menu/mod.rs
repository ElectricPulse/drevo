pub mod boolean;
mod string;

use std::sync::{Arc, Weak};

use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use vizual_macros::display;

use super::{
    super::{Control, Focus_provider, Widget_trait},
    button::Button,
    full::Full,
    layout::{Layout, Style as Layout_style},
    space::Space,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::{Child, Children, context::Component_context},
    display::Display,
    event::{Key_code, Key_event, Pointer_event},
    geometry::{Direction, Rect},
    handlers::{Retrieve_handler, Submit_handler},
    layouter::{hitbox::Hitbox, objective::Objective, variable::Variable},
    slot::manager::Slots,
    state::State,
    sync::{Mutex, Thread_safe},
    theme::Theme,
    utils::{get_next_index, get_previous_index},
};

// TODO: Replace this with a custom `Widget_custom_state<State>` that passes `State` into
// `layout(...state: State) render(...state: State)`
// implement this if ever Render is ever wrapped againt with a custom thing
// theme should definetely be passed in like this
#[async_trait]
pub trait Menu_item_trait<Value>: Retrieve_handler<Value> {
    async fn layout(
        &mut self,
        selected: bool,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Child>;

    async fn render(
        &mut self,
        _selected: bool,
        _focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        Ok(None)
    }
}

pub type Shared_menu_item<Value> = Arc<Mutex<dyn Menu_item_trait<Value>>>;
pub type Selector<Value> = Weak<Mutex<dyn Menu_item_trait<Value>>>;

pub fn get_selector<Value>(item: &Shared_menu_item<Value>) -> Selector<Value> {
    Arc::downgrade(item)
}

struct Menu_item<Value> {
    selected: bool,
    widget: Shared_menu_item<Value>,
    menu_selector: State<Selector<Value>>,
    theme: State<Theme>,
    button_delta: Variable,
    submit_state: Option<State<Value>>,
}

#[async_trait]
impl<Value: Thread_safe> Control for Menu_item<Value> {
    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        self.menu_selector.store(get_selector(&self.widget));

        if let Some(submit_state) = &self.submit_state {
            let mut widget = self.widget.lock().await?;
            submit_state.store(widget.on_retrieve().await?);
        }

        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[async_trait]
impl<Value: Thread_safe> Widget_trait for Menu_item<Value> {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let content = self
            .widget
            .lock()
            .await?
            .layout(
                self.selected,
                focus,
                hitbox,
                parent,
                problem.clone(),
                text_context,
                slots,
            )
            .await?;
        let mut button = Button::around(content, self.theme.clone());
        button.highlighted = self.selected;
        button.delta = Some(self.button_delta);
        let full = Full::new(display!(button));

        Ok(vec![display!(full)])
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        self.widget
            .lock()
            .await?
            .render(self.selected, focus, hitbox, display)
            .await
    }
}

pub struct Menu<Value> {
    items: Vec<Shared_menu_item<Value>>,
    pub selected: State<Selector<Value>>,
    default_item: Selector<Value>,
    pub theme: State<Theme>,
    submit_state: Option<State<Value>>,
}

impl<Value: Thread_safe> Menu<Value> {
    pub fn new(
        items: Vec<Shared_menu_item<Value>>,
        default_item: Selector<Value>,
        theme: State<Theme>,
    ) -> Self {
        Self {
            items,
            selected: State::new_with(theme.rerender.clone(), default_item.clone()),
            default_item,
            theme,
            submit_state: None,
        }
    }

    fn get_selected_item(&self) -> Result<Shared_menu_item<Value>> {
        let selected = self
            .selected
            .load()
            .upgrade()
            .ok_or_else(|| eyre!("Selected menu item selector is stale"))?;

        self.items
            .iter()
            .find(|item| Arc::ptr_eq(item, &selected))
            .cloned()
            .ok_or_else(|| eyre!("Selected menu item is not in the menu"))
    }

    fn get_selected_index(&self) -> Result<usize> {
        let selected = self.get_selected_item()?;
        self.items
            .iter()
            .position(|item| Arc::ptr_eq(item, &selected))
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
impl<Value: Thread_safe> Control for Menu<Value> {
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

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Value> for Menu<Value> {
    async fn on_retrieve(&mut self) -> Result<Value> {
        let item = self.get_selected_item()?;
        let value = item.lock().await?.on_retrieve().await?;
        Ok(value)
    }
}

#[async_trait]
impl<Value: Thread_safe + Clone> Widget_trait for Menu<Value> {
    async fn layout(
        &mut self,
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
            .any(|item| Arc::ptr_eq(item, &default_item))
        {
            return Err(eyre!("Default menu item is not in the menu"));
        }

        let selected = self.get_selected_item()?;
        let mut rows = Vec::with_capacity(self.items.len());
        let button_delta = problem.add_delta("menu-item-button-delta", 2).await?;

        for (index, item) in self.items.iter().enumerate() {
            let item = Menu_item {
                selected: Arc::ptr_eq(item, &selected),
                widget: item.clone(),
                menu_selector: self.selected.clone(),
                theme: self.theme.clone(),
                button_delta,
                submit_state: self.submit_state.as_ref().map(|state| state.clone()),
            };
            rows.push(slots.set(index as u64, item).await?);
        }

        let layout = display!(Layout::new(
            Direction::Vertical,
            rows,
            Layout_style::default(self.theme.clone()),
            Objective::default(),
            2,
        ));

        Ok(vec![display!(Full::new(layout))])
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        Ok(None)
    }
}
