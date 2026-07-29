use good_lp::{Expression, Variable};

use crate::geometry::{Direction, Point, Rect};

use super::{Problem, Solution};

// Extra performance is wasted in layouter for components that have static dimensions
// creating a variable and then assigning a static value is not yet free in the micro_lp solver as it lacks a presolve step
// as there aren't any components with static dimensions yet I don't see reason to implement static dimensions
#[derive(Clone, Copy)]
pub struct Dimensions {
    pub width: Variable,
    pub height: Variable,
}

impl Dimensions {
    pub fn get(self, direction: Direction) -> Variable {
        match direction {
            Direction::Horizontal => self.width,
            Direction::Vertical => self.height,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Hitbox {
    // TODO: add a shape to the hitbox
    //pub shape: Vec<bool>,
    pub x: Variable,
    pub y: Variable,
    pub dimensions: Dimensions,
}

impl Hitbox {
    pub fn new(problem: &mut Problem, name: String, component_path: String, path: String) -> Self {
        Self {
            x: problem.add_non_negative_variable(
                format!("{}.x", name),
                path.clone(),
                component_path.clone(),
            ),
            y: problem.add_non_negative_variable(
                format!("{}.y", name),
                path.clone(),
                component_path.clone(),
            ),
            dimensions: Dimensions {
                width: problem.add_non_negative_variable(
                    format!("{}.width", name),
                    path.clone(),
                    component_path.clone(),
                ),
                height: problem.add_non_negative_variable(
                    format!("{}.height", name),
                    path,
                    component_path,
                ),
            },
        }
    }

    pub fn get_position(self, direction: Direction) -> Variable {
        match direction {
            Direction::Horizontal => self.x,
            Direction::Vertical => self.y,
        }
    }

    pub fn get_dimension(self, direction: Direction) -> Variable {
        self.dimensions.get(direction)
    }

    pub fn get_start_position(self, direction: Direction) -> Variable {
        self.get_position(direction)
    }

    pub fn get_end_position(self, direction: Direction) -> Expression {
        self.get_position(direction) + self.get_dimension(direction)
    }

    pub fn get_resolved(&self, solution: &Solution) -> Rect {
        Rect::new(
            solution.value(self.x),
            solution.value(self.y),
            solution.value(self.dimensions.width),
            solution.value(self.dimensions.height),
        )
    }

    // It is questionable to access solution every time we want to get the value - maybe just rip it out of there
    pub fn hits(&self, solution: &Solution, position: Point) -> bool {
        let hitbox = self.get_resolved(solution);

        hitbox.contains(position)
    }
}
