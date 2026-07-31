use crate::{
    component::{Child, Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Focus_provider, Widget_trait, widgets::full::Full},
};
use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

#[derive(Clone, Copy)]
pub enum Position {
    Start,
    Middle,
    End,
}

pub struct Anchors {
    pub horizontal: Option<Position>,
    pub vertical: Option<Position>,
}

impl Anchors {
    pub fn middle() -> Self {
        Self {
            horizontal: Some(Position::Middle),
            vertical: Some(Position::Middle),
        }
    }
}

pub struct Anchor {
    child: Child,
    anchors: Anchors,
}

impl Anchor {
    pub fn new(child: Child, anchors: Anchors) -> Self {
        Self { child, anchors }
    }

    /// Applies the selected anchor while generic layout handles unshared-edge shrink wrapping.
    async fn anchor(
        problem: &Component_context,
        parent: Hitbox,
        hitbox: &mut Hitbox,
        position: Option<Position>,
        direction: Direction,
    ) -> Result<()> {
        match position {
            Some(Position::Start) => {
                hitbox.share_start(parent, problem, direction).await?;
            }
            Some(Position::Middle) => {
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(direction)
                            >= parent.get_start_position(direction)
                    ))
                    .await?;
                problem
                    .constrain(constraint!(
                        hitbox.get_end_position(direction) <= parent.get_end_position(direction)
                    ))
                    .await?;

                let start_margin = Expression::from(
                    hitbox.get_start_position(direction) - parent.get_start_position(direction),
                );
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);
                problem
                    .constrain(constraint!(start_margin.clone() == end_margin))
                    .await?;
                problem.minimize(start_margin, 0).await?;
            }
            Some(Position::End) => {
                hitbox.share_end(parent, problem, direction).await?;
            }
            None => {}
        }

        Ok(())
    }
}

#[async_trait]
impl Widget_trait for Anchor {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        Self::anchor(
            &problem,
            parent,
            hitbox,
            self.anchors.horizontal,
            Direction::Horizontal,
        )
        .await?;

        Self::anchor(
            &problem,
            parent,
            hitbox,
            self.anchors.vertical,
            Direction::Vertical,
        )
        .await?;

        let full = Full::new(self.child.clone());
        Ok(vec![display!(full)])
    }
}
