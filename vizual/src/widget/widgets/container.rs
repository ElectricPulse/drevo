use async_trait::async_trait;
use color_eyre::Result;

use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::{Child, Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
};

pub struct Container {
    child: Child,
}

impl Container {
    pub fn new(child: Child) -> Self {
        Self { child }
    }
}

//TODO: Used to create an extra hitbox - currently used in layout
#[async_trait]
impl Widget_trait for Container {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let child_hitbox = self.child.get_hitbox().await?;
        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(constraint!(
                    hitbox.get_start_position(direction)
                        == child_hitbox.get_start_position(direction)
                ))
                .await?;
            problem
                .constrain(constraint!(
                    hitbox.get_end_position(direction) == child_hitbox.get_end_position(direction)
                ))
                .await?;
        }

        Ok(vec![self.child.clone()])
    }
}
