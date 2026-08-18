use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::text::Text,
    Menu, Menu_item,
};
use crate::{
    component::Children,
    handlers::Retrieve_handler,
    state::State,
    widget::{Layout_input, custom_widget::Custom_widget_trait},
};

#[derive(Clone, Copy)]
struct Boolean_menu_item {
    value: bool,
}

impl Boolean_menu_item {
    fn label(&self) -> &'static str {
        if self.value { "Enabled" } else { "Disabled" }
    }
}

#[async_trait]
impl Retrieve_handler<bool> for Boolean_menu_item {
    async fn on_retrieve(&mut self) -> Result<State<bool>> {
        Ok(self.value.into())
    }
}

#[async_trait]
impl Custom_widget_trait for Boolean_menu_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut text = Text::new(self.label());
        let mut style = theme.specific.text.subtitle;
        if !selected {
            style.color = theme.semantic.text.muted;
        }
        text.style.set(style);

        Ok(vec![display!(text)])
    }
}

impl Menu<bool> {
    pub async fn boolean(default: bool) -> Result<Self> {
        let items = [false, true]
            .into_iter()
            .map(|value| -> Menu_item<bool> {
                Box::new(Boolean_menu_item { value })
            })
            .collect::<Vec<_>>();
        let default_item = usize::from(default);

        Self::new(items, default_item).await
    }

    pub async fn set_selected(&mut self, value: bool) -> Result<()> {
        self.set_index(usize::from(value)).await
    }
}
