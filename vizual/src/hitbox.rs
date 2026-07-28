use good_lp::{Expression, Variable};

use crate::{
    geometry::{Point, Rect},
    layouter::{Problem, Solution},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Direction {
    pub fn flip(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

// These are gonna be Variables in the solver even though one might
// imagine that even though they should be avoided that atleast static widths, heights will appear in components
// there it's easy to just add a lower/upper constraint
// This simplifies the architecture of the code by about a million percent as if a enum of Constant/Variable was implemented instead
// then the parent would have to point to a Arc<> of this enum as the parent often uses the values before the layout of the child is called
// It probably does waste performance though because of extra constraints
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
