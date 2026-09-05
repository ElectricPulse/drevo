use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{LayoutInput, WidgetTrait},
    block::Block,
};
use crate::{
    component::Children, style::Style, theme::Theme, widget::Widget,
    widget::widgets::block::BlockStyle,
};

#[derive(Clone, Copy, PartialEq)]
pub struct PaperStyle {
    pub block: BlockStyle,
}

#[derive(Clone, Style)]
pub struct Paper {
    child: Widget,
    pub style: Style<PaperStyle>,
}

impl Paper {
    pub fn new(child: impl WidgetTrait) -> Self {
        Self {
            child: child.as_any(),
            style: crate::style::Style::default(),
        }
    }
}

impl From<Theme> for PaperStyle {
    fn from(theme: Theme) -> Self {
        theme.specific.paper
    }
}

#[async_trait]
impl WidgetTrait for Paper {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let style = self.style.get(&theme);
        let block = Block::new(self.child.clone(), style.block);
        Ok(vec![display!(block)])
    }
}
