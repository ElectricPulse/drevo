use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::{display, position};

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
    widget::{Focus_provider, Widget, Widget_trait},
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
    pub fn new(child: impl Widget_trait, alignments: Alignments) -> Self {
        Self {
            child: Box::new(child),
            alignments,
        }
    }

    async fn align(
        problem: &Component_context,
        parent: Hitbox,
        hitbox: Hitbox,
        objective: Objective,
        direction: Direction,
    ) -> Result<()> {
        let priority = 0;

        match objective {
            Objective::Minimize => {
                let start_margin = Expression::from(
                    hitbox.get_start_position(direction) - parent.get_start_position(direction),
                );
                minimize(&mut *problem.lock().await?, start_margin, priority)
            }
            Objective::Maximize => {
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);
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
        parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
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
            Self::align(&problem, parent, *hitbox, horizontal, Direction::Horizontal).await?;
        }
        if let Some(vertical) = self.alignments.vertical {
            Self::align(&problem, parent, *hitbox, vertical, Direction::Vertical).await?;
        }

        Ok(vec![display!(self.child.clone())])
    }
}
