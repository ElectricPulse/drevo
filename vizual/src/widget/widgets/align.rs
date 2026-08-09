use async_trait::async_trait;
use color_eyre::Result;

use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{
        expression::Expression,
        hitbox::Hitbox,
        objective::{Objective, minimize},
    },
    slot::manager::Slots,
    widget::{Focus_provider, General_shared_widget, Widget_trait},
};

pub struct Alignments {
    pub horizontal: Option<Objective>,
    pub vertical: Option<Objective>,
}

pub struct Align {
    child: General_shared_widget,
    alignments: Alignments,
}

impl Align {
    pub fn new(child: General_shared_widget, alignments: Alignments) -> Self {
        Self { child, alignments }
    }

    async fn align(
        problem: &Component_context,
        hitbox: Hitbox,
        child_hitbox: Hitbox,
        objective: Objective,
        direction: Direction,
    ) -> Result<()> {
        let priority = 0;

        match objective {
            Objective::Minimize => {
                let start_margin = Expression::from(
                    child_hitbox.get_start_position(direction)
                        - hitbox.get_start_position(direction),
                );
                minimize(&mut *problem.lock().await?, start_margin, priority)
            }
            Objective::Maximize => {
                let end_margin =
                    hitbox.get_end_position(direction) - child_hitbox.get_end_position(direction);
                minimize(&mut *problem.lock().await?, end_margin, priority)
            }
            Objective::Minimize_delta => Ok(()),
        }
    }
}

#[async_trait]
impl Widget_trait for Align {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::State<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let child = slots.set(0, self.child.clone()).await?;
        let child_hitbox = child.get_hitbox().await?;

        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(constraint!(
                    child_hitbox.get_start_position(direction)
                        >= hitbox.get_start_position(direction)
                ))
                .await?;
            problem
                .constrain(constraint!(
                    child_hitbox.get_end_position(direction) <= hitbox.get_end_position(direction)
                ))
                .await?;
        }

        if let Some(horizontal) = self.alignments.horizontal {
            Self::align(
                &problem,
                *hitbox,
                child_hitbox,
                horizontal,
                Direction::Horizontal,
            )
            .await?;
        }
        if let Some(vertical) = self.alignments.vertical {
            Self::align(
                &problem,
                *hitbox,
                child_hitbox,
                vertical,
                Direction::Vertical,
            )
            .await?;
        }

        Ok(vec![child])
    }
}
