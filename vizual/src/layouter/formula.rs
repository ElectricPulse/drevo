use std::sync::Arc;

use super::{
    PRIORITY_LEVELS, constraint::Constraint, expression::Expression, variable::Variable,
    variables::Variables,
};

/// The declarations produced by one component's `layout` call.
///
/// A formula is independent from the transient `Problem` used by a solve. The latter is rebuilt
/// from the live component tree, while this value is retained by its owner.
#[derive(Clone)]
pub struct Formula {
    /// Helper variables declared while this component's layout is evaluated.
    pub(crate) variables: Vec<Variable>,
    pub(crate) constraints: Vec<Constraint>,
    pub(crate) objectives: [Vec<Expression>; PRIORITY_LEVELS],
    /// The shared variable registry used to allocate helper variables during layout.
    ///
    /// Constraints retain only variable identities. The registry owns the metadata that gives
    /// those identities their bounds, integer status, and diagnostic names when the formula is
    /// later added to a solve problem.
    pub(crate) registry: Arc<Variables>,
}

impl Formula {
    pub fn new(registry: Arc<Variables>) -> Self {
        Self {
            variables: Vec::new(),
            constraints: Vec::new(),
            objectives: std::array::from_fn(|_| Vec::new()),
            registry,
        }
    }

    pub(crate) fn constrain(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub(crate) fn registry(&self) -> Arc<Variables> {
        Arc::clone(&self.registry)
    }
}
