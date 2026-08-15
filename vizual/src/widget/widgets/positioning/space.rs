use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{
        expression::Expression,
        hitbox::Hitbox,
        objective::Delta,
    },
    slot::manager::Slots,
    widget::{Focus_provider, Widget, Widget_trait},
};
use async_trait::async_trait;
use color_eyre::eyre::{Result, ensure};
use vizual_macros::display;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spaces {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
}

impl Spaces {
    fn start(self, direction: Direction) -> Option<f64> {
        match direction {
            Direction::Horizontal => self.left,
            Direction::Vertical => self.top,
        }
    }

    fn end(self, direction: Direction) -> Option<f64> {
        match direction {
            Direction::Horizontal => self.right,
            Direction::Vertical => self.bottom,
        }
    }
}

/// Adds preferred spacing between a child and its parent.
///
/// TODO: Widget composition sometimes creates nested `Space` wrappers. Consider detecting and
/// combining adjacent spaces so redundant layout variables and component levels are optimized out.
#[derive(Clone)]
pub struct Space {
    child: Widget,
    spaces: Spaces,
    pub delta: Option<Delta>,
    pub minimum: f64,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Space {
    pub fn new(
        child: impl Widget_trait,
        left: Option<f64>,
        right: Option<f64>,
        top: Option<f64>,
        bottom: Option<f64>,
        priority: usize,
    ) -> Self {
        Self {
            child: Box::new(child),
            spaces: Spaces {
                left,
                right,
                top,
                bottom,
            },
            delta: None,
            minimum: 0.0,
            priority,
        }
    }

    pub fn inline(child: impl Widget_trait, value: f64, priority: usize) -> Self {
        Self::new(child, Some(value), Some(value), None, None, priority)
    }

    pub fn left(child: impl Widget_trait, value: f64, priority: usize) -> Self {
        Self::new(child, Some(value), None, None, None, priority)
    }

    pub fn right(child: impl Widget_trait, value: f64, priority: usize) -> Self {
        Self::new(child, None, Some(value), None, None, priority)
    }

    pub fn top(child: impl Widget_trait, value: f64, priority: usize) -> Self {
        Self::new(child, None, None, Some(value), None, priority)
    }

    pub fn bottom(child: impl Widget_trait, value: f64, priority: usize) -> Self {
        Self::new(child, None, None, None, Some(value), priority)
    }

    pub fn uniform(child: impl Widget_trait, value: f64, priority: usize) -> Self {
        Self::new(
            child,
            Some(value),
            Some(value),
            Some(value),
            Some(value),
            priority,
        )
    }

    pub fn full(child: impl Widget_trait, priority: usize) -> Self {
        Self::new(child, None, None, None, None, priority)
    }

    async fn expression(
        &self,
        problem: &Component_context,
        target: f64,
        delta: &mut Option<Delta>,
    ) -> Result<Expression> {
        ensure!(target > 0.0, "space target must be greater than zero");
        let delta = match delta.as_ref() {
            Some(delta) => delta.clone(),
            None => {
                let new_delta = problem.add_delta("space-delta", self.priority).await?;
                *delta = Some(new_delta.clone());
                new_delta
            }
        };
        let space = target * (1 - delta.clone());

        if self.minimum > 0.0 {
            problem
                .constrain(constraint!(space.clone() >= self.minimum))
                .await?;
        }
        problem.minimize(delta, self.priority).await?;

        Ok(space)
    }
}

#[async_trait]
impl Widget_trait for Space {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let spaces = self.spaces;
        let mut delta = self.delta.clone();

        for direction in [Direction::Horizontal, Direction::Vertical] {
            if let Some(space) = spaces.start(direction)
                && space != 0.0
            {
                let space = self.expression(&problem, space, &mut delta).await?;
                hitbox.make_start_independent(direction);
                problem
                    .constrain(constraint!(
                        hitbox.get_start_position(direction)
                            == parent.get_start_position(direction) + space
                    ))
                    .await?;
            }

            if let Some(space) = spaces.end(direction)
                && space != 0.0
            {
                let space = self.expression(&problem, space, &mut delta).await?;
                hitbox.make_end_independent(direction);
                problem
                    .constrain(constraint!(
                        hitbox.get_end_position(direction)
                            == parent.get_end_position(direction) - space
                    ))
                    .await?;
            }
        }

        Ok(vec![display!(self.child.clone())])
    }
}
