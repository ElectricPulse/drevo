use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::{Child, Children, context::Component_context},
    geometry::Direction,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    widget::{Focus_provider, Widget_trait},
};

/// Shares both this component and its child across the entire parent hitbox.
///
/// This component exists so callers do not have to pass `hitbox` and `problem` manually when
/// calling [`Hitbox::full`] on both layers.
pub struct Full {
    child: Child,
    width: bool,
    height: bool,
}

impl Full {
    pub fn new(child: Child) -> Self {
        Self {
            child,
            width: true,
            height: true,
        }
    }

    pub fn width(child: Child) -> Self {
        Self {
            child,
            width: true,
            height: false,
        }
    }

    pub fn height(child: Child) -> Self {
        Self {
            child,
            width: false,
            height: true,
        }
    }
}

#[async_trait]
impl Widget_trait for Full {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        match (self.width, self.height) {
            (true, true) => {
                hitbox.full(parent, &problem).await?;
                self.child
                    .lock()
                    .await?
                    .hitbox
                    .full(parent, &problem)
                    .await?;
            }
            (width, height) => {
                for (enabled, direction) in [
                    (width, Direction::Horizontal),
                    (height, Direction::Vertical),
                ] {
                    if enabled {
                        hitbox.share_dimension(parent, &problem, direction).await?;
                        self.child
                            .share_dimension(*hitbox, &problem, direction)
                            .await?;
                    }
                }
            }
        }

        Ok(vec![self.child.clone()])
    }
}
