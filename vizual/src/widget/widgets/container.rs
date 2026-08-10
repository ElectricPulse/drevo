use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

use super::super::{Focus_provider, Widget, Widget_trait};
use crate::{
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
};

/// Gives a child an independent intermediate hitbox.
#[derive(Clone)]
pub struct Container {
    child: Widget,
}

impl Container {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            child: Box::new(child),
        }
    }
}

#[async_trait]
impl Widget_trait for Container {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::State<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        Ok(vec![display!(self.child.clone())])
    }
}
