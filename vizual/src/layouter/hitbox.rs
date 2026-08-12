use std::sync::Arc;

use color_eyre::eyre::Result;
use good_lp::VariableDefinition;

use super::{
    Solution,
    expression::{Expression, Shared_expression},
    variable::Variable,
    variables::Variables,
};
use crate::{
    component::context::Component_context,
    geometry::{Direction, Point, Rect},
};

#[derive(Clone)]
struct Expression_position {
    x: Shared_expression,
    y: Shared_expression,
}

impl Expression_position {
    fn new(variables: &Variables, name: &str, component_path: &str, path: &str) -> Self {
        Self {
            x: Expression::from(add_variable(
                variables,
                format!("{name}.x"),
                path.to_string(),
                component_path.to_string(),
            ))
            .shared(),
            y: Expression::from(add_variable(
                variables,
                format!("{name}.y"),
                path.to_string(),
                component_path.to_string(),
            ))
            .shared(),
        }
    }

    fn shared(&self) -> Self {
        Self {
            x: Expression::from(&self.x).shared(),
            y: Expression::from(&self.y).shared(),
        }
    }

    fn get(&self, direction: Direction) -> Shared_expression {
        match direction {
            Direction::Horizontal => Arc::clone(&self.x),
            Direction::Vertical => Arc::clone(&self.y),
        }
    }

    fn set(&self, direction: Direction, expression: impl Into<Expression>) {
        *self
            .get(direction)
            .lock()
            .expect("layout position expression poisoned") = expression.into();
    }

    pub(crate) fn point_to(&self, position: &Self) {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            let expression = position.get(direction);
            if !Arc::ptr_eq(&self.get(direction), &expression) {
                self.set(direction, expression);
            }
        }
    }

    pub(crate) fn variable(&self, direction: Direction) -> Variable {
        clone_expression(&self.get(direction))
            .single_variable()
            .expect("layout position must contain one variable")
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
    start: Expression_position,
    end: Expression_position,
}

impl Hitbox {
    pub fn new(variables: &Variables, name: String, component_path: String, path: String) -> Self {
        Self {
            start: Expression_position::new(
                variables,
                &format!("{name}.start"),
                &component_path,
                &path,
            ),
            end: Expression_position::new(
                variables,
                &format!("{name}.end"),
                &component_path,
                &path,
            ),
        }
    }

    /// Creates child-owned expressions initialized from the parent's current positions.
    pub(crate) fn shared(parent: &Self) -> Self {
        Self {
            start: parent.start.shared(),
            end: parent.end.shared(),
        }
    }

    /// Replaces this hitbox's position expressions without invalidating expressions which already
    /// reference its shared handles.
    pub(crate) fn point_to(&self, parent: &Self) {
        self.start.point_to(&parent.start);
        self.end.point_to(&parent.end);
    }

    pub fn point_start(&self, direction: Direction, expression: impl Into<Expression>) {
        self.start.set(direction, expression);
    }

    pub fn point_end(&self, direction: Direction, expression: impl Into<Expression>) {
        self.end.set(direction, expression);
    }

    pub(crate) fn start_variable(&self, direction: Direction) -> Variable {
        self.start.variable(direction)
    }

    pub(crate) fn end_variable(&self, direction: Direction) -> Variable {
        self.end.variable(direction)
    }

    pub(crate) fn start_expression(&self, direction: Direction) -> Shared_expression {
        self.start.get(direction)
    }

    pub(crate) fn end_expression(&self, direction: Direction) -> Shared_expression {
        self.end.get(direction)
    }

    /// Replaces every position expression in this hitbox with a fresh solver variable.
    pub fn make_independent(&self, problem: &Component_context, name: &str) {
        for (position_name, position) in [("start", &self.start), ("end", &self.end)] {
            for direction in [Direction::Horizontal, Direction::Vertical] {
                position.set(
                    direction,
                    problem.make_independent_variable(format!("{name}.{position_name}")),
                );
            }
        }
    }

    pub(crate) fn make_independent_at(
        &self,
        variables: &Variables,
        name: &str,
        component_path: &str,
        path: &str,
    ) {
        for (position_name, position) in [("start", &self.start), ("end", &self.end)] {
            for direction in [Direction::Horizontal, Direction::Vertical] {
                let direction_name = match direction {
                    Direction::Horizontal => "x",
                    Direction::Vertical => "y",
                };
                position.set(
                    direction,
                    add_variable(
                        variables,
                        format!("{name}.{position_name}.{direction_name}"),
                        path.to_string(),
                        component_path.to_string(),
                    ),
                );
            }
        }
    }

    /// Constrains the derived dimension to match the parent's dimension on one axis.
    pub async fn share_dimension(
        &self,
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

    /// Constrains one derived dimension without replacing either shared edge expression.
    ///
    /// Despite its name, this does not change a position expression to a constant or mark its
    /// variables as static. Hitbox positions are nested shared expressions so positioning widgets,
    /// such as nested [`Space`](crate::widget::widgets::positioning::space::Space) wrappers, can
    /// edit equations in place without allocating an independent solver variable for every layer.
    ///
    /// Consequently, an intrinsically sized leaf inside those wrappers (for example, text) must
    /// express `end - start == value` as a constraint. Replacing its end expression with
    /// `start + value` would sever the inherited expression chain, so the fixed size would no
    /// longer propagate outward through the surrounding spaces and layout widgets.
    pub async fn set_static_dimension(
        &self,
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

    pub fn get_start_position(&self, direction: Direction) -> Expression {
        Expression::from(self.start.get(direction))
    }

    /// Returns an expression which follows the shared end position.
    pub fn get_end_position(&self, direction: Direction) -> Expression {
        Expression::from(self.end.get(direction))
    }

    pub fn get_resolved(&self, solution: &Solution) -> Rect {
        let x = solution.eval(&self.get_start_position(Direction::Horizontal));
        let y = solution.eval(&self.get_start_position(Direction::Vertical));
        Rect::new(
            x,
            y,
            solution.eval(&self.get_end_position(Direction::Horizontal)) - x,
            solution.eval(&self.get_end_position(Direction::Vertical)) - y,
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

fn clone_expression(expression: &Shared_expression) -> Expression {
    expression
        .lock()
        .expect("layout position expression poisoned")
        .clone()
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

        let width = hitbox
            .get_dimension(Direction::Horizontal)
            .resolved()
            .unwrap();
        assert_eq!(
            width
                .coefficients
                .get(&hitbox.start.variable(Direction::Horizontal)),
            Some(&-1.0)
        );
        assert_eq!(
            width
                .coefficients
                .get(&hitbox.end.variable(Direction::Horizontal)),
            Some(&1.0)
        );
        assert_eq!(width.coefficients.len(), 2);
    }

    #[tokio::test]
    async fn static_dimensions_constrain_inherited_expressions() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let hitbox = Hitbox::shared(&parent);
        let problem =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        hitbox
            .set_static_dimension(&problem, Direction::Horizontal, 42.0)
            .await?;

        let constraint = problem.lock().await?.constraints[0]
            .expression()
            .resolved()
            .unwrap();
        assert_eq!(constraint.coefficients.len(), 2);
        assert_eq!(constraint.constant, -42.0);
        assert_eq!(
            constraint
                .coefficients
                .get(&parent.start_variable(Direction::Horizontal)),
            Some(&-1.0)
        );
        assert_eq!(
            constraint
                .coefficients
                .get(&parent.end_variable(Direction::Horizontal)),
            Some(&1.0)
        );
        Ok(())
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
        let child = Hitbox::new(
            &variables,
            "child".to_string(),
            "child".to_string(),
            "test".to_string(),
        );
        let child_positions = [
            child.start.get(Direction::Horizontal),
            child.end.get(Direction::Horizontal),
        ];
        let context =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        child
            .share_dimension(&parent, &context, Direction::Horizontal)
            .await?;

        assert!(Arc::ptr_eq(
            &child.start.get(Direction::Horizontal),
            &child_positions[0]
        ));
        assert!(Arc::ptr_eq(
            &child.end.get(Direction::Horizontal),
            &child_positions[1]
        ));
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
        let expression = Expression::from(child.end.get(Direction::Horizontal));
        let independent = variables.make_independent(
            VariableDefinition::new().min(0),
            "independent",
            "test",
            "child",
        );

        child.end.set(Direction::Horizontal, independent.clone());

        let referenced = expression
            .resolved()
            .unwrap()
            .referenced_variables()
            .next()
            .unwrap();
        assert!(referenced.points_to(&independent));
        assert!(
            !parent
                .end
                .variable(Direction::Horizontal)
                .points_to(&independent)
        );
    }

    #[test]
    fn editing_positions_never_replaces_their_shared_expression() {
        let variables = Variables::new();
        let parent = Hitbox::new(
            &variables,
            "parent".to_string(),
            "parent".to_string(),
            "test".to_string(),
        );
        let child = Hitbox::shared(&parent);
        let child_start = child.start.get(Direction::Horizontal);
        let child_end = child.end.get(Direction::Horizontal);

        child.start.set(
            Direction::Horizontal,
            parent.get_start_position(Direction::Horizontal) + 10.0,
        );
        child.end.point_to(&parent.end);

        assert!(Arc::ptr_eq(
            &child_start,
            &child.start.get(Direction::Horizontal)
        ));
        assert!(Arc::ptr_eq(
            &child_end,
            &child.end.get(Direction::Horizontal)
        ));
    }
}
