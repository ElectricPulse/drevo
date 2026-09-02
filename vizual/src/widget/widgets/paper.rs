use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{Layout_input, Widget_trait},
    block::Block,
};
use crate::{
    component::Children, theme::Theme, widget::Widget, widget::widgets::block::Block_style,
};

#[derive(Clone, Copy, PartialEq)]
pub struct Paper_style {
    pub block: Block_style,
}

#[derive(Clone)]
pub struct Paper {
    child: Widget,
    pub style: crate::style::Style<Paper_style>,
}

impl Paper {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            child: child.as_any(),
            style: crate::style::Style::default(),
        }
    }
}

impl From<Theme> for Paper_style {
    fn from(theme: Theme) -> Self {
        theme.specific.paper
    }
}

#[async_trait]
impl Widget_trait for Paper {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let style = self.style.get(&theme);
        let block = Block::new(self.child.clone(), style.block);
        Ok(vec![display!(block)])
    }
}
