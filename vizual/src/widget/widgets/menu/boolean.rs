use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::super::Focus_provider, super::text::Text, Menu, Menu_item_trait, Shared_menu_item,
    get_selector,
};
use crate::{
    component::Child, handlers::Retrieve_handler, hitbox::Hitbox, layouter::Problem_context,
    slot_manager::Slots, state::State, sync::Mutex, theme::Theme,
};

struct Boolean_menu_item {
    value: bool,
    theme: State<Theme>,
}

impl Boolean_menu_item {
    fn label(&self) -> &'static str {
        if self.value { "Enabled" } else { "Disabled" }
    }
}

#[async_trait]
impl Retrieve_handler<bool> for Boolean_menu_item {
    async fn on_retrieve(&mut self) -> Result<bool> {
        Ok(self.value)
    }
}

#[async_trait]
impl Menu_item_trait<bool> for Boolean_menu_item {
    async fn layout(
        &mut self,
        selected: bool,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Problem_context,
        slots: &mut Slots,
    ) -> Result<Child> {
        let text =
            Text::new(self.label()).set_style(self.theme.load().semantic.text.subtitle(selected));

        Ok(display!(text))
    }
}

impl Menu<bool> {
    pub fn boolean(default: bool, theme: State<Theme>) -> Self {
        let items = [false, true]
            .into_iter()
            .map(|value| {
                Arc::new(Mutex::new(Boolean_menu_item {
                    value,
                    theme: theme.clone(),
                })) as Shared_menu_item<bool>
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(&items[usize::from(default)]);

        Self::new(items, default_item, theme)
    }

    pub(crate) fn set_selected(&self, value: bool) -> Result<()> {
        self.set_index(usize::from(value))
    }
}
