use async_trait::async_trait;
use color_eyre::Result;

use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::{Child, Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
};

/// Gives a child an intermediate hitbox without making the child full-sized in the grandparent.
///
/// This is necessary when wrappers such as [`Full`](super::full::Full) appear inside
/// [`Space`](super::space::Space). Without the intermediate hitbox, a full-sized child binds
/// directly to the `Space` hitbox, leaving no room for its margins and effectively ignoring the
/// requested padding. The child fills this container instead, while `Space` keeps its padding
/// outside the container.
pub struct Container {
    child: Child,
}

impl Container {
    pub fn new(child: Child) -> Self {
        Self { child }
    }
}

#[async_trait]
impl Widget_trait for Container {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        Ok(vec![self.child.clone()])
    }
}
