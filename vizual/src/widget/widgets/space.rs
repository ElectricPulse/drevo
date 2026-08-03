use super::{
    super::{Focus_provider, Widget_trait},
    container::Container,
};
use crate::{
    component::{Child, Children, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{
        expression::Expression,
        hitbox::Hitbox,
        objective::{Delta, Objective},
    },
    slot::manager::Slots,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Debug, Default)]
pub struct Spaces {
    pub left: Option<Expression>,
    pub right: Option<Expression>,
    pub top: Option<Expression>,
    pub bottom: Option<Expression>,
}

impl Spaces {
    fn start(&self, direction: Direction) -> Option<Expression> {
        match direction {
            Direction::Horizontal => self.left.clone(),
            Direction::Vertical => self.top.clone(),
        }
    }

    fn end(&self, direction: Direction) -> Option<Expression> {
        match direction {
            Direction::Horizontal => self.right.clone(),
            Direction::Vertical => self.bottom.clone(),
        }
    }

    fn is_empty(&self) -> bool {
        self.left.is_none() && self.right.is_none() && self.top.is_none() && self.bottom.is_none()
    }
}

pub struct Space {
    child: Child,
    spaces: Spaces,
    objective: Objective,
    pub delta: Delta,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Space {
    pub fn new(
        child: Child,
        left: Option<f64>,
        right: Option<f64>,
        top: Option<f64>,
        bottom: Option<f64>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self {
            child,
            spaces: Spaces {
                left: left.map(Expression::from),
                right: right.map(Expression::from),
                top: top.map(Expression::from),
                bottom: bottom.map(Expression::from),
            },
            objective,
            delta: Delta::default(),
            priority,
        }
    }

    pub fn inline(
        child: Child,
        value: impl Into<Expression>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        let value = value.into();
        Self::new_with_spaces(
            child,
            Spaces {
                left: Some(value.clone()),
                right: Some(value),
                ..Spaces::default()
            },
            objective,
            priority,
        )
    }

    pub fn left(
        child: Child,
        value: impl Into<Expression>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new_with_spaces(
            child,
            Spaces {
                left: Some(value.into()),
                ..Spaces::default()
            },
            objective,
            priority,
        )
    }

    pub fn right(
        child: Child,
        value: impl Into<Expression>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new_with_spaces(
            child,
            Spaces {
                right: Some(value.into()),
                ..Spaces::default()
            },
            objective,
            priority,
        )
    }

    pub fn top(
        child: Child,
        value: impl Into<Expression>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new_with_spaces(
            child,
            Spaces {
                top: Some(value.into()),
                ..Spaces::default()
            },
            objective,
            priority,
        )
    }

    pub fn bottom(
        child: Child,
        value: impl Into<Expression>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new_with_spaces(
            child,
            Spaces {
                bottom: Some(value.into()),
                ..Spaces::default()
            },
            objective,
            priority,
        )
    }

    pub fn uniform(
        child: Child,
        value: impl Into<Expression>,
        objective: Objective,
        priority: usize,
    ) -> Self {
        let value = value.into();
        Self::new_with_spaces(
            child,
            Spaces {
                left: Some(value.clone()),
                right: Some(value.clone()),
                top: Some(value.clone()),
                bottom: Some(value),
            },
            objective,
            priority,
        )
    }

    pub fn full(child: Child, objective: Objective, priority: usize) -> Self {
        Self::new(child, None, None, None, None, objective, priority)
    }

    fn new_with_spaces(
        child: Child,
        spaces: Spaces,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self {
            child,
            spaces,
            objective,
            delta: Delta::default(),
            priority,
        }
    }

    async fn apply_objective(
        &self,
        problem: &Component_context,
        space: Expression,
        target: Option<Expression>,
        delta: Delta,
    ) -> Result<()> {
        problem.constrain(constraint!(space.clone() >= 0)).await?;

        match target {
            Some(mut target) => {
                // TODO: Using 16 as the target for zero-sized space is also a workaround.
                if target.is_zero() {
                    target = Expression::from(16.0);
                }
                self.objective
                    .apply(problem, space - target, 0.0, delta, self.priority)
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
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let child = slots.set(0, Container::new(self.child.clone())).await?;
        let child_hitbox = child.get_hitbox().await?;
        let spaces = &self.spaces;
        let delta = match (self.objective, self.delta, spaces.is_empty()) {
            (Objective::Minimize_difference, None, false) => {
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
