use crate::{
    component::{Child, Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{expression::Expression, hitbox::Hitbox, objective::minimize},
    slot::manager::Slots,
    widget::{Focus_provider, General_shared_widget, Widget_trait},
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

pub struct Anchor {
    child: General_shared_widget,
    anchors: Anchors,
}

impl Anchor {
    pub fn new(child: General_shared_widget, anchors: Anchors) -> Self {
        Self { child, anchors }
    }

    pub fn center(child: General_shared_widget) -> Self {
        Self::new(child, Anchors::middle())
    }

    /// Applies the selected anchor to the child within this hitbox.
    async fn anchor(
        problem: &Component_context,
        hitbox: Hitbox,
        child: &Child,
        position: Option<Position>,
        direction: Direction,
    ) -> Result<()> {
        let child_hitbox = child.get_hitbox().await?;
        let matches_hitbox = child_hitbox.get_start_position(direction)
            == hitbox.get_start_position(direction)
            && child_hitbox.end.get(direction) == hitbox.end.get(direction);

        // This check is needed because even an aligned `Full` should render correctly.
        if matches_hitbox {
            return Ok(());
        }

        match position {
            Some(Position::Start) => {
                child.share_start(hitbox, problem, direction).await?;
            }
            Some(Position::Middle) => {
                problem
                    .constrain(constraint!(
                        child_hitbox.get_start_position(direction)
                            >= hitbox.get_start_position(direction)
                    ))
                    .await?;
                problem
                    .constrain(constraint!(
                        child_hitbox.get_end_position(direction)
                            <= hitbox.get_end_position(direction)
                    ))
                    .await?;

                let start_margin = Expression::from(
                    child_hitbox.get_start_position(direction)
                        - hitbox.get_start_position(direction),
                );
                let end_margin =
                    hitbox.get_end_position(direction) - child_hitbox.get_end_position(direction);
                problem
                    .constrain(constraint!(start_margin.clone() == end_margin))
                    .await?;
                minimize(&mut *problem.lock().await?, start_margin, 0)?;
            }
            Some(Position::End) => {
                child.share_end(hitbox, problem, direction).await?;
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
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let child = slots.set(0, self.child.clone()).await?;

        Self::anchor(
            &problem,
            *hitbox,
            &child,
            self.anchors.horizontal,
            Direction::Horizontal,
        )
        .await?;

        Self::anchor(
            &problem,
            *hitbox,
            &child,
            self.anchors.vertical,
            Direction::Vertical,
        )
        .await?;

        Ok(vec![child])
    }
}
