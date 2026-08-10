use super::super::{Focus_provider, Widget_trait};
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
    widget::{General_widget, General_widget_trait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

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
    child: General_widget,
    spaces: Spaces,
    objective: Objective,
    pub delta: Delta,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Space {
    pub fn new(
        child: impl General_widget_trait,
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
            delta: Delta::default(),
            priority,
        }
    }

    pub fn inline(
        child: impl General_widget_trait,
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
        child: impl General_widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, Some(value), None, None, None, objective, priority)
    }

    pub fn right(
        child: impl General_widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, Some(value), None, None, objective, priority)
    }

    pub fn top(
        child: impl General_widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, None, Some(value), None, objective, priority)
    }

    pub fn bottom(
        child: impl General_widget_trait,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, None, None, Some(value), objective, priority)
    }

    pub fn uniform(
        child: impl General_widget_trait,
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

    pub fn full(child: impl General_widget_trait, objective: Objective, priority: usize) -> Self {
        Self::new(child, None, None, None, None, objective, priority)
    }

    async fn apply_objective(
        &self,
        problem: &Component_context,
        space: Expression,
        target: Option<f64>,
        delta: Delta,
    ) -> Result<()> {
        problem.constrain(constraint!(space.clone() >= 0)).await?;

        match target {
            Some(target) => {
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
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let child = slots.set(0, self.child.clone()).await?;
        let child_hitbox = child.get_hitbox().await?;
        let spaces = self.spaces;
        let delta = match (self.objective, self.delta, spaces == Spaces::default()) {
            (Objective::Minimize_delta, None, false) => {
                Some(problem.add_delta("space-delta", self.priority).await?)
            }
            (_, delta, _) => delta,
        };

        for direction in [Direction::Horizontal, Direction::Vertical] {
            let start_space = Expression::from(
                child_hitbox.get_start_position(direction) - hitbox.get_start_position(direction),
            );

            self.apply_objective(&problem, start_space, spaces.start(direction), delta)
                .await?;

            let end_space = Expression::from(
                hitbox.get_end_position(direction) - child_hitbox.get_end_position(direction),
            );

            self.apply_objective(&problem, end_space, spaces.end(direction), delta)
                .await?;
        }

        Ok(vec![child])
    }
}
