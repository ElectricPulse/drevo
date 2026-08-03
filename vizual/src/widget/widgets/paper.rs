use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Widget_trait},
    block::Block,
    full::Full,
    space::Space,
};
use crate::{
    component::{Child, Children, context::Component_context},
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::widgets::block::Block_style,
};

#[derive(Clone, Copy, PartialEq)]
pub struct Paper_style {
    pub padding: f64,
    pub block: Block_style,
}

pub struct Paper {
    child: Child,
    style: State<Paper_style>,
}

impl Paper {
    pub fn new(child: Child, style: State<Paper_style>) -> Self {
        Self { child, style }
    }
}

impl From<&State<Theme>> for State<Paper_style> {
    fn from(theme: &State<Theme>) -> Self {
        theme.project(|theme| &theme.specific.paper)
    }
}

#[async_trait]
impl Widget_trait for Paper {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let space = Space::uniform(
            self.child.clone(),
            self.style.load().padding,
            Objective::default(),
            2,
        );

        let block_style = self.style.project(|style| &style.block);
        let block = Block::new(display!(space), block_style);
        let full = Full::new(display!(block));

        Ok(vec![display!(full)])
    }
}
