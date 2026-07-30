use crate::{
    component::{Shared_component, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Control, Focus_provider, Widget_trait, Widget_type},
};
use async_trait::async_trait;
use color_eyre::Result;

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
    child: Shared_component,
    alignments: Alignments,
}

impl Align {
    pub fn new(child: Shared_component, alignments: Alignments) -> Self {
        Self { child, alignments }
    }

    async fn constrain(
        problem: &Component_context,
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
                let start_margin = Expression::from(
                    child_hitbox.get_start_position(direction)
                        - hitbox.get_start_position(direction),
                );
                let end_margin =
                    hitbox.get_end_position(direction) - child_hitbox.get_end_position(direction);
                problem
                    .constrain(constraint!(start_margin == end_margin))
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

        Ok(Widget_type::visual_without_shrink_wrap(vec![
            self.child.clone(),
        ]))
    }
}
