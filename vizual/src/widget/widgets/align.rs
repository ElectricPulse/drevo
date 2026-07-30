use async_trait::async_trait;
use color_eyre::Result;

use crate::{
    component::{Child, Component, context::Component_context},
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
    pub async fn new(
        child: Child,
        alignments: Alignments,
        hitbox: Hitbox,
        problem: &Component_context,
    ) -> Result<Widget_type> {
        let horizontal = alignments.horizontal;
        let vertical = alignments.vertical;
        let align = Self { child, alignments };
        let align = Component::new(align, problem.clone()).await?.into_child();
        let align_hitbox = align.get_hitbox().await?;

        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(crate::constraint!(
                    align_hitbox.get_start_position(direction)
                        >= hitbox.get_start_position(direction)
                ))
                .await?;
            problem
                .constrain(crate::constraint!(
                    align_hitbox.get_end_position(direction) <= hitbox.get_end_position(direction)
                ))
                .await?;
        }

        if let Some(horizontal) = horizontal {
            Self::align(problem, align_hitbox, horizontal, Direction::Horizontal).await?;
        }
        if let Some(vertical) = vertical {
            Self::align(problem, align_hitbox, vertical, Direction::Vertical).await?;
        }

        Ok(Widget_type::Visual {
            children: vec![align],
        })
    }

    async fn align(
        problem: &Component_context,
        child_hitbox: Hitbox,
        objective: Objective,
        direction: Direction,
    ) -> Result<()> {
        let priority = 2;
        match objective {
            Objective::Minimize => {
                problem
                    .minimize(
                        Expression::from(child_hitbox.get_start_position(direction)),
                        priority,
                    )
                    .await
            }
            Objective::Maximize => {
                problem
                    .maximize(child_hitbox.get_end_position(direction), priority)
                    .await
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
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let child_hitbox = self.child.get_hitbox().await?;

        if let Some(horizontal) = self.alignments.horizontal {
            Self::align(&problem, child_hitbox, horizontal, Direction::Horizontal).await?;
        }
        if let Some(vertical) = self.alignments.vertical {
            Self::align(&problem, child_hitbox, vertical, Direction::Vertical).await?;
        }

        Widget_type::wrap(vec![self.child.clone()], hitbox, &problem, true, true).await
    }
}
