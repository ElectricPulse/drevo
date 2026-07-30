use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Control, Focus_provider, Widget_trait, Widget_type},
    block::Block,
    space::Space,
};
use crate::{
    component::{Child, context::Component_context},
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
};

#[derive(Clone, Copy)]
pub struct Paper_style {
    pub frame_padding: f64,
}

pub struct Paper {
    child: Child,
    theme: State<Theme>,
}

impl Paper {
    pub fn new(child: Child, theme: State<Theme>) -> Self {
        Self { child, theme }
    }
}

impl Control for Paper {}

#[async_trait]
impl Widget_trait for Paper {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let space = Space::uniform(
            self.child.clone(),
            self.theme.load().specific.paper.frame_padding,
            Objective::default(),
            2,
        );

        let block = Block::new(display!(space), self.theme.clone());

        Ok(Widget_type::Virtual(Box::new(block)))
    }
}
