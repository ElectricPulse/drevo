use async_trait::async_trait;
use color_eyre::Result;
use good_lp::constraint;

use crate::{
    component::Child,
    hitbox::{Direction, Hitbox},
    layouter::Problem_context,
    slot_manager::Slots,
    widget::{Control, Focus_provider, Renderable, Widget_type},
};

#[derive(Clone, Copy)]
pub enum Alignment {
    Start,
    Middle,
    End,
}

pub struct Alignments {
    pub horizontal: Alignment,
    pub vertical: Alignment,
}

impl Alignments {
    pub fn middle() -> Self {
        Self {
            horizontal: Alignment::Middle,
            vertical: Alignment::Middle,
        }
    }
}

pub struct Align {
    child: Child,
    alignments: Alignments,
}

impl Align {
    pub fn new(child: Child, alignments: Alignments) -> Self {
        Self { child, alignments }
    }

    async fn constrain(
        problem: &Problem_context,
        hitbox: Hitbox,
        child_hitbox: Hitbox,
        alignment: Alignment,
        direction: Direction,
    ) -> Result<()> {
        match alignment {
            Alignment::Start => {
                problem
                    .constrain(constraint!(
                        child_hitbox.get_start_position(direction)
                            == hitbox.get_start_position(direction)
                    ))
                    .await?;
            }
            Alignment::Middle => {
                let margin = problem.add_non_negative_variable("align-margin").await?;

                problem
                    .constrain(constraint!(
                        hitbox.get_dimension(direction)
                            == child_hitbox.get_dimension(direction) + 2 * margin
                    ))
                    .await?;
                problem
                    .constrain(constraint!(
                        child_hitbox.get_start_position(direction)
                            == hitbox.get_start_position(direction) + margin
                    ))
                    .await?;
            }
            Alignment::End => {
                problem
                    .constrain(constraint!(
                        child_hitbox.get_end_position(direction)
                            == hitbox.get_end_position(direction)
                    ))
                    .await?;
            }
        }

        Ok(())
    }
}

impl Control for Align {}

#[async_trait]
impl Renderable for Align {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let child_hitbox = self.child.get_hitbox().await?;

        Self::constrain(
            &problem,
            hitbox,
            child_hitbox,
            self.alignments.horizontal,
            Direction::Horizontal,
        )
        .await?;

        Self::constrain(
            &problem,
            hitbox,
            child_hitbox,
            self.alignments.vertical,
            Direction::Vertical,
        )
        .await?;

        Ok(Widget_type::Visual(vec![self.child.clone()]))
    }
}
