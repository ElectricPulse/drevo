use async_trait::async_trait;
use color_eyre::Result;

use crate::{
    component::{Child, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    widget::{Control, Focus_provider, Widget_trait, Widget_type},
};

pub struct Alignments {
    pub horizontal: Option<Objective>,
    pub vertical: Option<Objective>,
}

pub struct Align {
    child: Child,
    alignments: Alignments,
}

impl Align {
    pub fn new(child: Child, alignments: Alignments) -> Self {
        Self { child, alignments }
    }

    async fn align(
        problem: &Component_context,
        parent: Hitbox,
        hitbox: Hitbox,
        objective: Objective,
        direction: Direction,
    ) -> Result<()> {
        let priority = 2;
        match objective {
            Objective::Minimize => {
                let start_margin = Expression::from(
                    hitbox.get_start_position(direction) - parent.get_start_position(direction),
                );
                problem.minimize(start_margin, priority).await
            }
            Objective::Maximize => {
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);
                problem.minimize(end_margin, priority).await
            }
            Objective::Minimize_difference => Ok(()),
        }
    }
}

impl Control for Align {}

#[async_trait]
impl Widget_trait for Align {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(constraint!(
                    hitbox.get_start_position(direction) >= parent.get_start_position(direction)
                ))
                .await?;
            problem
                .constrain(constraint!(
                    hitbox.get_end_position(direction) <= parent.get_end_position(direction)
                ))
                .await?;
        }

        if let Some(horizontal) = self.alignments.horizontal {
            Self::align(&problem, parent, hitbox, horizontal, Direction::Horizontal).await?;
        }
        if let Some(vertical) = self.alignments.vertical {
            Self::align(&problem, parent, hitbox, vertical, Direction::Vertical).await?;
        }

        Ok(Widget_type::Virtual(Box::new(self.child.clone())))
    }
}
