use color_eyre::eyre::Result;

use super::{Solution, expression::Expression, variable::Variable, variables::Variables};
use crate::{
    constraint,
    geometry::{Direction, Point, Rect},
    id,
    layouter::Formula,
};

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct VariablePosition {
    pub x: Variable,
    pub y: Variable,
}

impl VariablePosition {
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
    pub start: VariablePosition,
    pub end: VariablePosition,
}

impl Hitbox {
    pub fn new(variables: &Variables, name: String, component_path: String, path: String) -> Self {
        Self {
            start: VariablePosition::new(
                variables,
                &format!("{name}.start"),
                &component_path,
                &path,
            ),
            end: VariablePosition::new(variables, &format!("{name}.end"), &component_path, &path),
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
        formula: &mut Formula,
    ) -> Result<()> {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            constrain_shared_variable(
                self.start.get(direction),
                parent.start.get(direction),
                formula,
            )?;
            constrain_shared_variable(self.end.get(direction), parent.end.get(direction), formula)?;
        }
        Ok(())
    }

    /// Constrains the derived dimension to match the parent's dimension on one axis.
    pub async fn share_dimension(
        &self,
        parent: &Hitbox,
        formula: &mut Formula,
        direction: Direction,
    ) -> Result<()> {
        formula.constrain(
            id!(),
            constraint!(self.get_dimension(direction) == parent.get_dimension(direction)),
        )
    }

    /// Constrains one derived dimension to a fixed value.
    pub async fn set_static_dimension(
        &self,
        formula: &mut Formula,
        direction: Direction,
        value: f64,
    ) -> Result<()> {
        formula.constrain(
            id!(),
            constraint!(self.get_dimension(direction) == value),
        )
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

fn constrain_shared_variable(
    variable: Variable,
    parent: Variable,
    formula: &mut Formula,
) -> Result<()> {
    if variable.is_shared() && variable != parent {
        formula.constrain(id!(), constraint!(variable == parent))?;
    }
    Ok(())
}

fn add_variable(
    variables: &Variables,
    name: String,
    path: String,
    component_path: String,
) -> Variable {
    variables.make(name, path, component_path)
}
