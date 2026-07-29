use super::super::{Control, Focus_provider, Widget_trait, Widget_type};
use crate::{
    component::{Shared_component, context::Component_context},
    constraint,
    geometry::Direction,
    layouter::{constraints::Objective, expression::Expression, hitbox::Hitbox},
    slot::manager::Slots,
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

pub struct Space {
    child: Shared_component,
    spaces: Spaces,
    objective: Objective,
    // TODO: Keep priority manual until there is a way to set it automatically.
    priority: usize,
}

impl Space {
    pub fn new(
        child: Shared_component,
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
                left,
                right,
                top,
                bottom,
            },
            objective,
            priority,
        }
    }

    pub fn inline(
        child: Shared_component,
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
        child: Shared_component,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, Some(value), None, None, None, objective, priority)
    }

    pub fn right(
        child: Shared_component,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, Some(value), None, None, objective, priority)
    }

    pub fn top(child: Shared_component, value: f64, objective: Objective, priority: usize) -> Self {
        Self::new(child, None, None, Some(value), None, objective, priority)
    }

    pub fn bottom(
        child: Shared_component,
        value: f64,
        objective: Objective,
        priority: usize,
    ) -> Self {
        Self::new(child, None, None, None, Some(value), objective, priority)
    }

    pub fn uniform(
        child: Shared_component,
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

    pub fn full(child: Shared_component, objective: Objective, priority: usize) -> Self {
        Self::new(child, None, None, None, None, objective, priority)
    }

    async fn apply_objective(
        &self,
        problem: &Component_context,
        space: Expression,
        target: Option<f64>,
    ) -> Result<()> {
        problem.constrain(constraint!(space.clone() >= 0)).await?;

        match target {
            Some(target) => {
                // TODO: Using 16 as the proportion for zero-sized space is also a bodge.
                let proportion = match target {
                    0.0 => 16.0,
                    target => target,
                };
                self.objective
                    .apply(problem, space, target, proportion, self.priority)
                    .await
            }
            None => Ok(()),
        }
    }
}

impl Control for Space {}

#[async_trait]
impl Widget_trait for Space {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let child_hitbox = self.child.get_hitbox().await?;
        let spaces = self.spaces;

        for direction in [Direction::Horizontal, Direction::Vertical] {
            let start_space = Expression::from(
                child_hitbox.get_start_position(direction) - hitbox.get_start_position(direction),
            );

            self.apply_objective(&problem, start_space, spaces.start(direction))
                .await?;

            let end_space = Expression::from(
                hitbox.get_end_position(direction) - child_hitbox.get_end_position(direction),
            );

            self.apply_objective(&problem, end_space, spaces.end(direction))
                .await?;
        }

        Ok(Widget_type::Visual(vec![self.child.clone()]))
    }
}
