use crate::{
    component::{Child, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Control, Focus_provider, Widget_trait, Widget_type},
};
use async_trait::async_trait;
use color_eyre::Result;

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

    /// Constrains one axis directly so this widget does not need a separate shrink-wrap pass.
    ///
    /// A fixed edge is constrained to its parent. Each free edge is kept within the parent and
    /// optimized inward at priority zero. Middle anchoring instead keeps equal, non-negative
    /// margins and minimizes one of them.
    async fn anchor(
        problem: &Component_context,
        parent: Hitbox,
        hitbox: Hitbox,
        position: Option<Position>,
        direction: Direction,
    ) -> Result<()> {
        if !matches!(position, Some(Position::Middle)) {
            if !matches!(position, Some(Position::Start)) {
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(direction)
                            >= parent.get_start_position(direction)
                    ))
                    .await?;
                problem
                    .maximize(Expression::from(hitbox.get_start_position(direction)), 0)
                    .await?;
            }

            if !matches!(position, Some(Position::End)) {
                problem
                    .constrain(constraint!(
                        hitbox.get_end_position(direction) <= parent.get_end_position(direction)
                    ))
                    .await?;
                problem
                    .minimize(hitbox.get_end_position(direction), 0)
                    .await?;
            }
        }

        match position {
            Some(Position::Start) => {
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(direction)
                            == parent.get_start_position(direction)
                    ))
                    .await?;
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
                problem
                    .constrain(constraint!(
                        hitbox.get_end_position(direction) == parent.get_end_position(direction)
                    ))
                    .await?;
            }
            None => {}
        }

        Ok(())
    }
}

impl Control for Anchor {}

#[async_trait]
impl Widget_trait for Anchor {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
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

        Ok(Widget_type::Virtual(Box::new(self.child.clone())))
    }
}
