use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{super::text::Text, Menu, Menu_item};
use crate::{
    component::Children,
    handlers::Retrieve_handler,
    state::State,
    widget::{Layout_input, custom_widget::Custom_widget_trait},
};

#[derive(Clone)]
struct String_menu_item {
    value: String,
}

#[async_trait]
impl Retrieve_handler<String> for String_menu_item {
    async fn on_retrieve(&mut self) -> Result<State<String>> {
        Ok(self.value.clone().into())
    }
}

#[async_trait]
impl Custom_widget_trait for String_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        Layout_input {
            relayout,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let mut text = Text::new(self.value.clone());
        let mut style = theme.specific.text.button;
        if !selected {
            style.color = theme.semantic.text.muted;
        }
        text.style.set(style);

        Ok(vec![display!(text)])
    }
}

impl Menu<String> {
    pub async fn text(items: Vec<String>, default_item: usize) -> Result<Self> {
        let items = items
            .into_iter()
            .map(|value| -> Menu_item<String> { Box::new(String_menu_item { value }) })
            .collect::<Vec<_>>();

        Self::new(items, default_item).await
    }
}
