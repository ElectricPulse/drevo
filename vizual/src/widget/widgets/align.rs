use async_trait::async_trait;
use color_eyre::Result;

use crate::{
    component::{Shared_component, context::Component_context},
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Control, Focus_provider, Widget_trait, Widget_type},
};

#[derive(Clone, Copy)]
pub enum Alignment {
    Minimize,
    Maximize,
}

pub struct Alignments {
    pub horizontal: Option<Alignment>,
    pub vertical: Option<Alignment>,
}

pub struct Align {
    child: Shared_component,
    alignments: Alignments,
}

impl Align {
    pub fn new(child: Shared_component, alignments: Alignments) -> Self {
        Self { child, alignments }
    }

    async fn align(
        problem: &Component_context,
        child_hitbox: Hitbox,
        alignment: Alignment,
        direction: Direction,
    ) -> Result<()> {
        let priority = 2;
        match alignment {
            Alignment::Minimize => {
                problem
                    .minimize(
                        Expression::from(child_hitbox.get_start_position(direction)),
                        priority,
                    )
                    .await
            }
            Alignment::Maximize => {
                problem
                    .maximize(child_hitbox.get_end_position(direction), priority)
                    .await
            }
        }
    }
}

impl Control for Align {}

#[async_trait]
impl Widget_trait for Align {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
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

        Ok(Widget_type::Visual {
            children: vec![self.child.clone()],
        })
    }
}
