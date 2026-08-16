use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{super::Focus_provider, text::Text},
    Menu, Menu_item,
};
use crate::{
    component::{Children, context::Component_context},
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::custom_widget::Custom_widget_trait,
};

#[derive(Clone)]
struct String_menu_item {
    value: String,
}

#[async_trait]
impl Retrieve_handler<String> for String_menu_item {
    async fn on_retrieve(&mut self) -> Result<String> {
        Ok(self.value.clone())
    }
}

#[async_trait]
impl Custom_widget_trait for String_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
        _logical: &mut bool,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new(self.value.clone());
        text.style.set(match selected {
            true => theme.specific.text.selected_subtitle,
            false => theme.specific.text.subtitle,
        });

        Ok(vec![display!(text)])
    }
}

impl Menu<String> {
    pub async fn text(items: Vec<String>, default_item: usize) -> Result<Self> {
        let items = items
            .into_iter()
            .map(|value| -> Menu_item<String> {
                Box::new(String_menu_item { value })
            })
            .collect::<Vec<_>>();

        Self::new(items, default_item).await
    }
}
