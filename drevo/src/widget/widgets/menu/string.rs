use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{super::text::Text, Menu, MenuItem};
use crate::{
    component::Children,
    handlers::RetrieveHandler,
    state::State,
    widget::{LayoutInput, custom_widget::CustomWidgetTrait},
};

#[derive(Clone)]
struct StringMenuItem {
    value: String,
}

#[async_trait]
impl RetrieveHandler<String> for StringMenuItem {
    async fn on_retrieve(&mut self) -> Result<State<String>> {
        Ok(self.value.clone().into())
    }
}

#[async_trait]
impl CustomWidgetTrait for StringMenuItem {
    type Payload = bool;

    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let mut style = theme.specific.text.button;
        if !selected {
            style.color = theme.semantic.text.muted;
        }

        Ok(vec![display!(Text::new(self.value.clone()).style(style))])
    }
}

impl Menu<String> {
    pub async fn text(items: Vec<String>, default_item: usize) -> Result<Self> {
        let items = items
            .into_iter()
            .map(|value| -> MenuItem<String> { Box::new(StringMenuItem { value }) })
            .collect::<Vec<_>>();

        Self::new(items, default_item).await
    }
}
