use std::hash::{Hash, Hasher};

/// A solver variable identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Solver_variable(pub usize);

/// A hitbox coordinate backed directly by a solver variable.
///
/// Coordinates initially share their corresponding parent coordinate. Positioning widgets make
/// an edge independent when they provide a different equation for it. Sharing is materialized as
/// an equality constraint after the component has finished laying itself out.
#[derive(Clone, Copy, Debug)]
pub struct Variable {
    pub(crate) variable: Solver_variable,
    pub(crate) shared: bool,
}

impl Variable {
    pub(crate) fn new(variable: Solver_variable) -> Self {
        Self {
            variable,
            shared: true,
        }
    }

    /// Stops this coordinate from being constrained to the corresponding parent coordinate.
    pub fn make_independent(&mut self) {
        self.shared = false;
    }

    pub(crate) fn reset_shared(&mut self) {
        self.shared = true;
    }

    pub(crate) fn is_shared(self) -> bool {
        self.shared
    }
}

// `shared` describes how a hitbox edge is finalized; it is not part of the solver variable's
// identity. This also keeps expressions stable when a coordinate is made independent.
impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        self.variable == other.variable
    }
}

impl Eq for Variable {}

impl Hash for Variable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.variable.hash(state);
    }
}
