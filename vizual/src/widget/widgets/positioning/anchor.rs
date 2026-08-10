use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox, objective::minimize},
    slot::manager::Slots,
    widget::{Focus_provider, Widget, Widget_trait},
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

#[derive(Clone)]
pub struct Anchors {
    pub horizontal: Option<Position>,
    pub vertical: Option<Position>,
}

impl Anchors {
    pub fn top_left() -> Self {
        Self {
            horizontal: Some(Position::Start),
            vertical: Some(Position::Start),
        }
    }

    pub fn middle() -> Self {
        Self {
            horizontal: Some(Position::Middle),
            vertical: Some(Position::Middle),
        }
    }
}

#[derive(Clone)]
pub struct Anchor {
    child: Widget,
    anchors: Anchors,
}

impl Anchor {
    pub fn new(child: impl Widget_trait, anchors: Anchors) -> Self {
        Self {
            child: Box::new(child),
            anchors,
        }
    }

    pub fn center(child: impl Widget_trait) -> Self {
        Self::new(child, Anchors::middle())
    }

    /// Applies the selected anchor to this hitbox within its parent.
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
                minimize(&mut *problem.lock().await?, start_margin, 0)?;
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
        _render: crate::Render,
        _theme: crate::state::State<crate::theme::Theme>,
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

        Ok(vec![display!(self.child.clone())])
    }
}
