use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::super::Focus_provider, super::text::Text, Menu, Menu_item_trait, Shared_menu_item,
    get_selector,
};
use crate::{
    component::{Child, context::Component_context},
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    sync::Mutex,
    theme::Theme,
};

struct String_menu_item {
    value: String,
    theme: State<Theme>,
}

#[async_trait]
impl Retrieve_handler<String> for String_menu_item {
    async fn on_retrieve(&mut self) -> Result<String> {
        Ok(self.value.clone())
    }
}

#[async_trait]
impl Menu_item_trait<String> for String_menu_item {
    async fn layout(
        &mut self,
        selected: bool,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Child> {
        let text = Text::new(self.value.clone())
            .set_style(self.theme.load().semantic.text.subtitle(selected));

        Ok(display!(text))
    }
}

impl Menu<String> {
    pub fn text(items: Vec<String>, default_item: usize, theme: State<Theme>) -> Self {
        let items = items
            .into_iter()
            .map(|value| {
                Arc::new(Mutex::new(String_menu_item {
                    value,
                    theme: theme.clone(),
                })) as Shared_menu_item<String>
            })
            .collect::<Vec<_>>();
        let default_item = get_selector(
            items
                .get(default_item)
                .expect("Default menu item index must be in range"),
        );

        Self::new(items, default_item, theme)
    }
}
