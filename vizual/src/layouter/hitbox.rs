use color_eyre::eyre::Result;
use good_lp::VariableDefinition;

use super::{Solution, expression::Expression, variable::Variable, variables::Variables};
use crate::{
    component::context::Component_context,
    geometry::{Direction, Point, Rect},
};

#[derive(Clone)]
pub struct Variable_position {
    pub x: Variable,
    pub y: Variable,
}

impl Variable_position {
    fn new(variables: &Variables, name: &str, component_path: &str, path: &str) -> Self {
        Self {
            x: add_variable(
                variables,
                format!("{name}.x"),
                path.to_string(),
                component_path.to_string(),
            ),
            y: add_variable(
                variables,
                format!("{name}.y"),
                path.to_string(),
                component_path.to_string(),
            ),
        }
    }

    fn shared(&self) -> Self {
        Self {
            x: self.x.shared(),
            y: self.y.shared(),
        }
    }

    pub fn get(&self, direction: Direction) -> Variable {
        match direction {
            Direction::Horizontal => self.x.clone(),
            Direction::Vertical => self.y.clone(),
        }
    }

    /// Repoints one stable position handle to the supplied variable definition.
    pub fn point_to_variable(&mut self, direction: Direction, variable: Variable) {
        self.get(direction).point_to(&variable);
    }

    pub(crate) fn point_to(&self, position: &Self) {
        self.x.point_to(&position.x);
        self.y.point_to(&position.y);
    }
}

/// A rectangular layout region stored as independent start and end positions.
///
/// Dimensions are derived from these positions. No blanket `end >= start` constraints are added:
/// widgets add ordering directly or inherit it through their explicit child relationships.
#[derive(Clone)]
pub struct Hitbox {
    // TODO: add a shape to the hitbox
    //pub shape: Vec<bool>,
    pub start: Variable_position,
    pub end: Variable_position,
}

impl Hitbox {
    pub fn new(variables: &Variables, name: String, component_path: String, path: String) -> Self {
        Self {
            start: Variable_position::new(
                variables,
                &format!("{name}.start"),
                &component_path,
                &path,
            ),
            end: Variable_position::new(variables, &format!("{name}.end"), &component_path, &path),
        }
    }

    /// Creates child-owned handles which initially point to the parent's definitions.
    pub(crate) fn shared(parent: &Self) -> Self {
        Self {
            start: parent.start.shared(),
            end: parent.end.shared(),
        }
    }

    /// Resets this hitbox to point to its parent's definitions without invalidating expressions
    /// which already hold its variable handles.
    pub(crate) fn point_to(&self, parent: &Self) {
        self.start.point_to(&parent.start);
        self.end.point_to(&parent.end);
    }

    /// Repoints every stable handle in this hitbox to a fresh solver variable.
    pub fn make_independent(&mut self, problem: &Component_context, name: &str) {
        for (position_name, position) in [("start", &mut self.start), ("end", &mut self.end)] {
            for direction in [Direction::Horizontal, Direction::Vertical] {
                position.point_to_variable(
                    direction,
                    problem.make_independent_variable(format!("{name}.{position_name}")),
                );
            }
        }
    }

    pub(crate) fn make_independent_at(
        &mut self,
        variables: &Variables,
        name: &str,
        component_path: &str,
        path: &str,
    ) {
        let independent = Self::new(
            variables,
            name.to_string(),
            component_path.to_string(),
            path.to_string(),
        );
        self.point_to(&independent);
    }

    /// Constrains the derived dimension to match the parent's dimension on one axis.
    pub async fn share_dimension(
        &mut self,
        parent: &Hitbox,
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

    /// Returns the derived `end - start` dimension for one axis.
    pub fn get_dimension(&self, direction: Direction) -> Expression {
        self.get_end_position(direction) - self.get_start_position(direction)
    }

    pub fn get_start_position(&self, direction: Direction) -> Variable {
        self.start.get(direction)
    }

    /// Returns an expression for the primitive end-position variable.
    pub fn get_end_position(&self, direction: Direction) -> Expression {
        Expression::from(self.end.get(direction))
    }

    pub fn get_resolved(&self, solution: &Solution) -> Rect {
        let x = solution.value(&self.start.x);
        let y = solution.value(&self.start.y);
        Rect::new(
            x,
            y,
            solution.value(&self.end.x) - x,
            solution.value(&self.end.y) - y,
        )
    }

    // It is questionable to access solution every time we want to get the value - maybe just rip it out of there
    pub fn hits(&self, solution: &Solution, position: Point) -> bool {
        self.get_resolved(solution).contains(position)
    }
}

impl Default for Hitbox {
    fn default() -> Self {
        let variables = Variables::new();
        Self::new(
            &variables,
            "empty".to_string(),
            String::new(),
            String::new(),
        )
    }
}

fn add_variable(
    variables: &Variables,
    name: String,
    path: String,
    component_path: String,
) -> Variable {
    variables.make_independent(
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
        let child_variables = [child.start.x.clone(), child.end.x.clone()];
        let context =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        child
            .share_dimension(&parent, &context, Direction::Horizontal)
            .await?;

        assert_eq!(
            [child.start.x.clone(), child.end.x.clone()],
            child_variables
        );
        assert_eq!(context.lock().await?.constraints.len(), 1);
        Ok(())
    }

    #[test]
    fn repointing_updates_existing_expressions() {
        let variables = Variables::new();
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let child = Hitbox::shared(&parent);
        let expression = Expression::from(child.end.x.clone());
        let independent = variables.make_independent(
            VariableDefinition::new().min(0),
            "independent",
            "test",
            "child",
        );

        child.end.x.point_to(&independent);

        let referenced = expression.referenced_variables().next().unwrap();
        assert!(referenced.points_to(&independent));
        assert!(parent.end.x.points_to(&parent.end.x));
        assert!(!parent.end.x.points_to(&independent));
    }
}
