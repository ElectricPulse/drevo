use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Widget_trait},
    block::Block,
    positioning::space::Space,
};
use crate::{
    component::{Children, context::Component_context},
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::Widget,
    widget::widgets::block::Block_style,
};

#[derive(Clone, Copy, PartialEq)]
pub struct Paper_style {
    pub padding: f64,
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
            child: Box::new(child),
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
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let style = self.style.get(&theme);
        let space = Space::uniform(self.child.clone(), style.padding, Objective::default(), 2);

        let mut block = Block::new(space);
        block.style.set(style.block);
        Ok(vec![display!(block)])
    }
}
