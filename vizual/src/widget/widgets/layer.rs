use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::super::{Focus_provider, Widget, Widget_trait};
use crate::{
    Render,
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme,
};

/// Assigns a rendering layer to its child relationship.
#[derive(Clone)]
pub struct Layer {
    child: Widget,
    pub layer: usize,
}

impl Layer {
    pub fn new(child: impl Widget_trait, layer: usize) -> Self {
        Self {
            child: Box::new(child),
            layer,
        }
    }
}

#[async_trait]
impl Widget_trait for Layer {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let mut child = display!(self.child.clone());
        child.layer = self.layer;
        Ok(vec![child])
    }
}
