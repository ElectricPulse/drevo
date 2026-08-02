use crate::geometry::{Direction, Point, Rect};
use color_eyre::eyre::Result;
use good_lp::VariableDefinition;

use super::{Solution, expression::Expression, variable::Variable, variables::Variables};
use crate::component::context::Component_context;

#[derive(Clone, Copy)]
pub struct Variable_position {
    pub x: Variable,
    pub y: Variable,
    owned_x: bool,
    owned_y: bool,
}

impl Variable_position {
    pub fn get(self, direction: Direction) -> Variable {
        match direction {
            Direction::Horizontal => self.x,
            Direction::Vertical => self.y,
        }
    }
}

/// A rectangular layout region stored as independent start and end positions.
///
/// Dimensions are derived from these positions. No blanket `end >= start` constraints are added:
/// components that enclose an ordered child inherit its ordering through shrink wrapping.
#[derive(Clone, Copy)]
pub struct Hitbox {
    // TODO: add a shape to the hitbox
    //pub shape: Vec<bool>,
    pub start: Variable_position,
    pub end: Variable_position,
}

impl Hitbox {
    pub fn new(variables: &Variables, name: String, component_path: String, path: String) -> Self {
        Self {
            start: Variable_position {
                x: add_variable(
                    variables,
                    format!("{}.start.x", name),
                    path.clone(),
                    component_path.clone(),
                ),
                y: add_variable(
                    variables,
                    format!("{}.start.y", name),
                    path.clone(),
                    component_path.clone(),
                ),
                owned_x: true,
                owned_y: true,
            },
            end: Variable_position {
                x: add_variable(
                    variables,
                    format!("{}.end.x", name),
                    path.clone(),
                    component_path.clone(),
                ),
                y: add_variable(variables, format!("{}.end.y", name), path, component_path),
                owned_x: true,
                owned_y: true,
            },
        }
    }

    /// Reuses the parent's start-position variable on one axis.
    ///
    /// Existing constraints and objectives that reference the replaced variable are rewritten to
    /// reference the parent variable.
    pub async fn share_start(
        &mut self,
        parent: Hitbox,
        problem: &Component_context,
        direction: Direction,
    ) -> Result<()> {
        let mut problem = problem.lock().await?;
        match direction {
            Direction::Horizontal => {
                share_variable(
                    &mut problem,
                    &mut self.start.x,
                    &mut self.start.owned_x,
                    parent.start.x,
                );
            }
            Direction::Vertical => {
                share_variable(
                    &mut problem,
                    &mut self.start.y,
                    &mut self.start.owned_y,
                    parent.start.y,
                );
            }
        }
        Ok(())
    }

    /// Reuses the parent's end-position variable on one axis.
    pub async fn share_end(
        &mut self,
        parent: Hitbox,
        problem: &Component_context,
        direction: Direction,
    ) -> Result<()> {
        let mut problem = problem.lock().await?;
        match direction {
            Direction::Horizontal => {
                share_variable(
                    &mut problem,
                    &mut self.end.x,
                    &mut self.end.owned_x,
                    parent.end.x,
                );
            }
            Direction::Vertical => {
                share_variable(
                    &mut problem,
                    &mut self.end.y,
                    &mut self.end.owned_y,
                    parent.end.y,
                );
            }
        }
        Ok(())
    }

    /// Constrains the derived dimension to match the parent's dimension on one axis.
    pub async fn share_dimension(
        &mut self,
        parent: Hitbox,
        problem: &Component_context,
        direction: Direction,
    ) -> Result<()> {
        problem
            .constrain(crate::constraint!(
                self.get_dimension(direction) == parent.get_dimension(direction)
            ))
            .await
    }

    /// Constrains one derived dimension to a static value.
    ///
    /// TODO: In the future this could rotate from the start-end pair to either end-width or
    /// start-width, like Cassowary, instead of adding a constraint over the derived dimension.
    pub async fn set_static_dimension(
        &mut self,
        problem: &Component_context,
        direction: Direction,
        value: f64,
    ) -> Result<()> {
        problem
            .constrain(crate::constraint!(self.get_dimension(direction) == value))
            .await
    }

    /// Reuses both start and end variables from the parent on both axes.
    pub async fn full(&mut self, parent: Hitbox, problem: &Component_context) -> Result<()> {
        let mut problem = problem.lock().await?;
        share_variable(
            &mut problem,
            &mut self.start.x,
            &mut self.start.owned_x,
            parent.start.x,
        );
        share_variable(
            &mut problem,
            &mut self.start.y,
            &mut self.start.owned_y,
            parent.start.y,
        );
        share_variable(
            &mut problem,
            &mut self.end.x,
            &mut self.end.owned_x,
            parent.end.x,
        );
        share_variable(
            &mut problem,
            &mut self.end.y,
            &mut self.end.owned_y,
            parent.end.y,
        );
        Ok(())
    }

    /// Returns the derived `end - start` dimension for one axis.
    pub fn get_dimension(self, direction: Direction) -> Expression {
        self.get_end_position(direction) - self.get_start_position(direction)
    }

    pub fn get_start_position(self, direction: Direction) -> Variable {
        self.start.get(direction)
    }

    /// Returns an expression for the primitive end-position variable.
    pub fn get_end_position(self, direction: Direction) -> Expression {
        Expression::from(self.end.get(direction))
    }

    pub fn get_resolved(&self, solution: &Solution) -> Rect {
        let x = solution.value(self.start.x);
        let y = solution.value(self.start.y);
        Rect::new(
            x,
            y,
            solution.value(self.end.x) - x,
            solution.value(self.end.y) - y,
        )
    }

    pub(crate) fn remove_variables(self, variables: &Variables) {
        remove_position_variables(self.start, variables);
        remove_position_variables(self.end, variables);
    }

    pub(crate) fn make_independent(
        &mut self,
        variables: &Variables,
        name: &str,
        component_path: &str,
        path: &str,
    ) {
        make_position_independent(
            &mut self.start,
            variables,
            &format!("{name}.start"),
            component_path,
            path,
        );
        make_position_independent(
            &mut self.end,
            variables,
            &format!("{name}.end"),
            component_path,
            path,
        );
    }

    // It is questionable to access solution every time we want to get the value - maybe just rip it out of there
    pub fn hits(&self, solution: &Solution, position: Point) -> bool {
        let hitbox = self.get_resolved(solution);

        hitbox.contains(position)
    }
}

impl Default for Hitbox {
    fn default() -> Self {
        let empty = Variable::new(0);
        let empty_position = Variable_position {
            x: empty,
            y: empty,
            owned_x: false,
            owned_y: false,
        };
        Self {
            start: empty_position,
            end: empty_position,
        }
    }
}

fn remove_position_variables(position: Variable_position, variables: &Variables) {
    if position.owned_x {
        variables.remove(position.x);
    }
    if position.owned_y {
        variables.remove(position.y);
    }
}

fn make_position_independent(
    position: &mut Variable_position,
    variables: &Variables,
    name: &str,
    component_path: &str,
    path: &str,
) {
    if !position.owned_x {
        position.x = add_variable(
            variables,
            format!("{name}.x"),
            path.to_string(),
            component_path.to_string(),
        );
        position.owned_x = true;
    }
    if !position.owned_y {
        position.y = add_variable(
            variables,
            format!("{name}.y"),
            path.to_string(),
            component_path.to_string(),
        );
        position.owned_y = true;
    }
}

fn share_variable(
    problem: &mut super::Problem,
    variable: &mut Variable,
    owned: &mut bool,
    parent: Variable,
) {
    if *variable == parent {
        return;
    }
    problem.replace_variable(*variable, parent, *owned);
    *variable = parent;
    *owned = false;
}

fn add_variable(
    variables: &Variables,
    name: String,
    path: String,
    component_path: String,
) -> Variable {
    variables.add(
        VariableDefinition::new().min(0).name(name.clone()),
        name,
        path,
        component_path,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        constraint,
        layouter::{Problem, expression::Expression},
        sync::Mutex,
    };

    #[test]
    fn dimensions_are_derived_from_start_and_end() {
        let variables = Variables::new();
        let hitbox = Hitbox::new(
            &variables,
            "hitbox".to_string(),
            "hitbox".to_string(),
            "test".to_string(),
        );

        let width = hitbox.get_dimension(Direction::Horizontal);
        assert_eq!(width.coefficients.get(&hitbox.start.x), Some(&-1.0));
        assert_eq!(width.coefficients.get(&hitbox.end.x), Some(&1.0));
        assert_eq!(width.coefficients.len(), 2);

        let end = hitbox.get_end_position(Direction::Horizontal);
        assert_eq!(end.coefficients.get(&hitbox.end.x), Some(&1.0));
        assert_eq!(end.coefficients.len(), 1);
    }

    #[tokio::test]
    async fn sharing_dimension_does_not_share_positions() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let mut child = Hitbox::new(
            &variables,
            "child".to_string(),
            "child".to_string(),
            "test".to_string(),
        );
        let child_variables = [child.start.x, child.end.x];
        let context =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        child
            .share_dimension(parent, &context, Direction::Horizontal)
            .await?;

        assert_eq!([child.start.x, child.end.x], child_variables);
        assert_eq!(context.lock().await?.constraints.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn full_rewrites_existing_layout_expressions() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let mut child = Hitbox::new(
            &variables,
            "child".to_string(),
            "child".to_string(),
            "test".to_string(),
        );
        let old_variables = [child.start.x, child.start.y, child.end.x, child.end.y];
        let child_expression =
            Expression::from(child.start.x) + child.start.y + child.end.x + child.end.y;
        let mut problem = Problem::new(Arc::clone(&variables));
        problem.constrain(constraint!(child_expression.clone() == 0));
        problem.maximize(child_expression, 0)?;
        let context = Component_context::new(Arc::new(Mutex::new(problem)));

        child.full(parent, &context).await?;

        assert_eq!(child.start.x, parent.start.x);
        assert_eq!(child.start.y, parent.start.y);
        assert_eq!(child.end.x, parent.end.x);
        assert_eq!(child.end.y, parent.end.y);

        let problem = context.lock().await?;
        for old in old_variables {
            assert!(problem.constraints.iter().all(|constraint| {
                !constraint
                    .expression
                    .referenced_variables()
                    .any(|v| v == old)
            }));
            assert!(
                problem
                    .objectives
                    .iter()
                    .flatten()
                    .all(|objective| !objective.referenced_variables().any(|v| v == old))
            );
        }

        Ok(())
    }
}
