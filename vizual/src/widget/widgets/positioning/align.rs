use crate::macros::display;
use async_trait::async_trait;
use color_eyre::Result;

use crate::{
    component::{Children, context::ComponentContext},
    constraint,
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective},
    widget::{LayoutInput, Widget, WidgetTrait},
};

#[derive(Clone)]
pub struct Alignments {
    pub horizontal: Option<Objective>,
    pub vertical: Option<Objective>,
}

#[derive(Clone)]
pub struct Align {
    child: Widget,
    alignments: Alignments,
}

impl Align {
    pub fn new(child: impl WidgetTrait, alignments: Alignments) -> Self {
        Self {
            child: child.as_any(),
            alignments,
        }
    }

    async fn align(
        problem: &ComponentContext,
        parent: &Hitbox,
        hitbox: &mut Hitbox,
        objective: Objective,
        direction: Direction,
    ) -> Result<()> {
        let priority = 0;

        match objective {
            Objective::Minimize => {
                let start_margin =
                    hitbox.get_start_position(direction) - parent.get_start_position(direction);
                problem.minimize(start_margin, priority).await
            }
            Objective::Maximize => {
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);
                problem.minimize(end_margin, priority).await
            }
            Objective::MinimizeDelta => Ok(()),
        }
    }
}

#[async_trait]
impl WidgetTrait for Align {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            parent,
            problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        for (direction, objective) in [
            (Direction::Horizontal, self.alignments.horizontal),
            (Direction::Vertical, self.alignments.vertical),
        ] {
            if objective.is_some() {
                hitbox.make_start_independent(direction);
                hitbox.make_end_independent(direction);
            }
        }

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
            Self::align(&problem, &parent, hitbox, horizontal, Direction::Horizontal).await?;
        }
        if let Some(vertical) = self.alignments.vertical {
            Self::align(&problem, &parent, hitbox, vertical, Direction::Vertical).await?;
        }

        Ok(vec![display!(self.child.clone())])
    }
}
