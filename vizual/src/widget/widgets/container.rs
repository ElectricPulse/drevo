use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

use super::super::{Focus_provider, Widget, Widget_trait};
use crate::{
    component::{Children, context::Component_context},
    geometry::{Direction, Size},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
};

/// Gives a child an independent intermediate hitbox.
#[derive(Clone)]
pub struct Container {
    child: Widget,
    fixed_size: Option<Size>,
}

impl Container {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            child: Box::new(child),
            fixed_size: None,
        }
    }

    pub fn fixed_size(mut self, size: Size) -> Self {
        self.fixed_size = Some(size);
        self
    }
}

#[async_trait]
impl Widget_trait for Container {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
        _logical: &mut bool,
    ) -> Result<Children> {
        if let Some(size) = self.fixed_size {
            hitbox
                .set_static_dimension(&problem, Direction::Horizontal, size.width)
                .await?;
            hitbox
                .set_static_dimension(&problem, Direction::Vertical, size.height)
                .await?;
        }

        let child = display!(self.child.clone());

        Ok(vec![child])
    }
}
