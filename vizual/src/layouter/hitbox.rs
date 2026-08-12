use color_eyre::eyre::Result;
use good_lp::VariableDefinition;

use super::{Solution, expression::Expression, variable::Variable, variables::Variables};
use crate::{
    component::context::Component_context,
    geometry::{Direction, Point, Rect},
};

// A little bit needs to be said about hitbox and the storage of variables
// Some obvious optimizations I had tried doing but ended up removing in favour of just waiting till my solver implements a presolve step for them
// basically first you can share variables via Arc<Mutex<Variable>> where you then have Variable_position contain pointer to those shared variables via Arc<Mutex<Arc<Mutex<Variable>>>
// this makes parent/child variable sharing as easy as copying the Arc<Mutex<Variable>> into the Arc<Mutex<...>>
// Next you can replace Variable with an Expression so that things like nested spaces dont really create new variables instead make the hitboxes be expressions
// All of these things get fixed by a good pre solver and added too much complexity - for example:
// if a text has a static size it should logically set its variable to an expression of that size - but if that text is inside a triply nested space - it would make sense
// for the hitbox of that text to in the end be an expression to optimize that out and not have extra hitboxes - that then means that the static box width/height must
// be an equality constraint
// but if no space exists it would be best to set it as a value
// so if you want performance switch to a competent solver and leave microlp only where WASM capability is needed

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

    pub fn get(&self, direction: Direction) -> Variable {
        match direction {
            Direction::Horizontal => self.x,
            Direction::Vertical => self.y,
        }
    }

    fn get_mut(&mut self, direction: Direction) -> &mut Variable {
        match direction {
            Direction::Horizontal => &mut self.x,
            Direction::Vertical => &mut self.y,
        }
    }

    fn reset_shared(&mut self) {
        self.x.reset_shared();
        self.y.reset_shared();
    }

    fn make_independent(&mut self) {
        self.x.make_independent();
        self.y.make_independent();
    }
}

/// A rectangular layout region whose four coordinates are direct solver variables.
///
/// A new coordinate is shared by default. After its widget has completed layout, a shared
/// coordinate is constrained equal to the corresponding coordinate of the parent. Positioning
/// widgets call [`Variable::make_independent`] on only the coordinates for which they provide
/// another equation.
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

    /// Restores the default sharing state when a retained component begins a new layout.
    pub(crate) fn reset_shared(&mut self) {
        self.start.reset_shared();
        self.end.reset_shared();
    }

    /// Makes all four existing coordinates independent without allocating replacements.
    pub fn make_independent(&mut self) {
        self.start.make_independent();
        self.end.make_independent();
    }

    pub fn make_start_independent(&mut self, direction: Direction) {
        self.start.get_mut(direction).make_independent();
    }

    pub fn make_end_independent(&mut self, direction: Direction) {
        self.end.get_mut(direction).make_independent();
    }

    /// Adds the delayed parent equalities for every coordinate that remains shared.
    pub(crate) async fn constrain_shared(
        &self,
        parent: &Self,
        problem: &Component_context,
    ) -> Result<()> {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            constrain_shared_variable(
                self.start.get(direction),
                parent.start.get(direction),
                problem,
            )
            .await?;
            constrain_shared_variable(self.end.get(direction), parent.end.get(direction), problem)
                .await?;
        }
        Ok(())
    }

    /// Replaces this resolved render-time hitbox with another hitbox.
    pub(crate) fn point_to(&mut self, hitbox: &Self) {
        *self = hitbox.clone();
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

    /// Constrains one derived dimension to a fixed value.
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

async fn constrain_shared_variable(
    variable: Variable,
    parent: Variable,
    problem: &Component_context,
) -> Result<()> {
    if variable.is_shared() && variable != parent {
        problem
            .constrain(crate::constraint!(variable == parent))
            .await?;
    }
    Ok(())
}

fn add_variable(
    variables: &Variables,
    name: String,
    path: String,
    component_path: String,
) -> Variable {
    variables.make(
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
    use crate::{layouter::Problem, sync::Mutex};

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
        assert_eq!(
            width.coefficients.get(&hitbox.start.x.variable),
            Some(&-1.0)
        );
        assert_eq!(width.coefficients.get(&hitbox.end.x.variable), Some(&1.0));
        assert_eq!(width.coefficients.len(), 2);
    }

    #[tokio::test]
    async fn shared_edges_are_constrained_to_the_parent_after_layout() -> Result<()> {
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
        let context =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        child.constrain_shared(&parent, &context).await?;

        let problem = context.lock().await?;
        assert_eq!(problem.constraints.len(), 4);
        let horizontal_start = problem.constraints[0].expression();
        assert_eq!(
            horizontal_start.coefficients.get(&child.start.x.variable),
            Some(&1.0)
        );
        assert_eq!(
            horizontal_start.coefficients.get(&parent.start.x.variable),
            Some(&-1.0)
        );
        Ok(())
    }

    #[tokio::test]
    async fn independent_edges_do_not_receive_parent_equalities() -> Result<()> {
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
        child.make_start_independent(Direction::Horizontal);
        let context =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        child.constrain_shared(&parent, &context).await?;

        assert_eq!(context.lock().await?.constraints.len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn static_dimensions_add_a_constraint_over_the_existing_edges() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let hitbox = Hitbox::new(
            &variables,
            "child".to_string(),
            "child".to_string(),
            "test".to_string(),
        );
        let context =
            Component_context::new(Arc::new(Mutex::new(Problem::new(Arc::clone(&variables)))));

        hitbox
            .set_static_dimension(&context, Direction::Horizontal, 42.0)
            .await?;

        let problem = context.lock().await?;
        let constraint = problem.constraints[0].expression();
        assert_eq!(constraint.coefficients.len(), 2);
        assert_eq!(constraint.constant, -42.0);
        Ok(())
    }
}
