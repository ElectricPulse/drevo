use crate::{
    component::{Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{
        expression::Expression,
        hitbox::Hitbox,
        objective::{Delta, Objective},
    },
    slot::manager::Slots,
    widget::{Focus_provider, Widget, Widget_trait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
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

#[derive(Clone)]
pub struct Space {
    child: Widget,
    spaces: Spaces,
    objective: Objective,
    pub delta: Option<Delta>,
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
        objective: Objective,
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
            objective,
            delta: None,
            priority,
        }
    }

    pub fn inline(
        child: impl Widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(
            child,
            Some(value),
            Some(value),
            None,
            None,
            objective,
            priority,
        )
    }

    pub fn left(
        child: impl Widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, Some(value), None, None, None, objective, priority)
    }

    pub fn right(
        child: impl Widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, Some(value), None, None, objective, priority)
    }

    pub fn top(
        child: impl Widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, None, Some(value), None, objective, priority)
    }

    pub fn bottom(
        child: impl Widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, None, None, Some(value), objective, priority)
    }

    pub fn uniform(
        child: impl Widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(
            child,
            Some(value),
            Some(value),
            Some(value),
            Some(value),
            objective,
            priority,
        )
    }

    pub fn full(child: impl Widget_trait, objective: Objective, priority: usize) -> Self {
        Self::new(child, None, None, None, None, objective, priority)
    }

    async fn apply_objective(
        &self,
        problem: &Component_context,
        space: Expression,
        target: Option<f64>,
        delta: &mut Option<Delta>,
    ) -> Result<()> {
        problem.constrain(constraint!(space.clone() >= 0)).await?;

        match target {
            Some(target) => {
                let delta = match *delta {
                    Some(delta) => delta,
                    None => {
                        let new_delta = problem.add_delta("space-delta", self.priority).await?;
                        *delta = Some(new_delta);
                        new_delta
                    }
                };
                // TODO: Using 16 as the target for zero-sized space is also a workaround.
                let target = match target {
                    0.0 => 16.0,
                    target => target,
                };
                self.objective
                    .apply(problem, space, target, delta, self.priority)
                    .await
            }
            None => Ok(()),
        }
    }
}

#[async_trait]
impl Widget_trait for Space {
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
        let spaces = self.spaces;
        let mut delta = self.delta;

        for direction in [Direction::Horizontal, Direction::Vertical] {
            let start_space = Expression::from(
                hitbox.get_start_position(direction) - parent.get_start_position(direction),
            );

            self.apply_objective(&problem, start_space, spaces.start(direction), &mut delta)
                .await?;

            let end_space = Expression::from(
                parent.get_end_position(direction) - hitbox.get_end_position(direction),
            );

            self.apply_objective(&problem, end_space, spaces.end(direction), &mut delta)
                .await?;
        }

        Ok(vec![display!(self.child.clone())])
    }
}
