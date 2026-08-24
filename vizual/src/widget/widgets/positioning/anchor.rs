use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::hitbox::Hitbox,
    widget::{Layout_input, Widget, Widget_trait},
};
use async_trait::async_trait;
use color_eyre::Result;
use crate::macros::display;

#[derive(Clone, Copy)]
pub enum Anchor_position {
    Start,
    Middle,
    End,
}

#[derive(Clone)]
pub struct Anchors {
    pub horizontal: Option<Anchor_position>,
    pub vertical: Option<Anchor_position>,
}

#[derive(Clone)]
pub struct Anchor {
    child: Widget,
    anchors: Anchors,
}

impl Anchor {
    pub fn new(child: impl Widget_trait, anchors: Anchors) -> Self {
        Self {
            child: child.as_any(),
            anchors,
        }
    }

    pub fn left(child: impl Widget_trait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(Anchor_position::Start),
                vertical: None,
            },
        )
    }

    pub fn right(child: impl Widget_trait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: None,
            },
        )
    }

    pub fn top(child: impl Widget_trait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: None,
                vertical: Some(Anchor_position::Start),
            },
        )
    }

    pub fn top_left(child: impl Widget_trait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(Anchor_position::Start),
                vertical: Some(Anchor_position::Start),
            },
        )
    }

    pub fn middle(child: impl Widget_trait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: Some(Anchor_position::Middle),
                vertical: Some(Anchor_position::Middle),
            },
        )
    }

    pub fn v_middle(child: impl Widget_trait) -> Self {
        Self::new(
            child,
            Anchors {
                horizontal: None,
                vertical: Some(Anchor_position::Middle),
            },
        )
    }

    /// Applies the selected anchor to this hitbox within its parent.
    async fn anchor(
        problem: &Component_context,
        parent: &Hitbox,
        hitbox: &mut Hitbox,
        position: Option<Anchor_position>,
        direction: Direction,
    ) -> Result<()> {
        match position {
            Some(Anchor_position::Start) => {
                hitbox.make_end_independent(direction);
                problem
                    .constrain(constraint!(
                        hitbox.get_end_position(direction) <= parent.get_end_position(direction)
                    ))
                    .await?;
            }
            Some(Anchor_position::Middle) => {
                hitbox.make_start_independent(direction);
                hitbox.make_end_independent(direction);

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

                let start_margin =
                    hitbox.get_start_position(direction) - parent.get_start_position(direction);
                let end_margin =
                    parent.get_end_position(direction) - hitbox.get_end_position(direction);
                problem
                    .constrain(constraint!(start_margin.clone() == end_margin))
                    .await?;
                problem.minimize(start_margin, 0).await?;
            }
            Some(Anchor_position::End) => {
                hitbox.make_start_independent(direction);
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(direction)
                            >= parent.get_start_position(direction)
                    ))
                    .await?;
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
        Layout_input {
            hitbox,
            parent,
            problem,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        Self::anchor(
            &problem,
            &parent,
            hitbox,
            self.anchors.horizontal,
            Direction::Horizontal,
        )
        .await?;

        Self::anchor(
            &problem,
            &parent,
            hitbox,
            self.anchors.vertical,
            Direction::Vertical,
        )
        .await?;

        Ok(vec![display!(self.child.clone())])
    }
}
