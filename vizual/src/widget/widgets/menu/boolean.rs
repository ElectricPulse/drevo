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

#[derive(Clone, Copy)]
struct BooleanMenuItem {
    value: bool,
}

impl BooleanMenuItem {
    fn label(&self) -> &'static str {
        if self.value { "Enabled" } else { "Disabled" }
    }
}

#[async_trait]
impl RetrieveHandler<bool> for BooleanMenuItem {
    async fn on_retrieve(&mut self) -> Result<State<bool>> {
        Ok(self.value.into())
    }
}

#[async_trait]
impl CustomWidgetTrait for BooleanMenuItem {
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
        let mut text = Text::new(self.label());
        let mut style = theme.specific.text.button;
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
            .map(|value| -> MenuItem<bool> { Box::new(BooleanMenuItem { value }) })
            .collect::<Vec<_>>();
        let default_item = usize::from(default);

        Self::new(items, default_item).await
    }

    pub async fn set_selected(&mut self, value: bool) -> Result<()> {
        self.set_index(usize::from(value)).await
    }
}
