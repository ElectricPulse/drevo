use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use good_lp::VariableDefinition;

use super::variables::Variable_type;

pub(crate) type Shared_variable_definition = Arc<Mutex<Variable_definition>>;

#[derive(Clone)]
pub struct Variable_definition {
    pub(crate) variable_type: Variable_type<VariableDefinition>,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) component_path: String,
}

/// A stable symbolic variable whose shared definition can be replaced.
///
/// The outer `Arc<Mutex<_>>` is the stable handle stored in expressions. The inner
/// `Arc<Mutex<Variable_definition>>` lets that handle be repointed while other variables continue
/// to share the old definition. A registry `HashMap` was also a valid design, but it added extra
/// complexity to track whether a `Variable_definition` was still used; direct `Arc` ownership
/// makes that lifetime explicit.
///
/// These deliberately use `std::sync::Mutex`, not the crate's Tokio mutex: definition lookup and
/// repointing are synchronous operations used while constructing expressions and solver state.
/// Each critical section only clones or replaces an `Arc`, and no guard may live across an
/// `.await`. Tokio does not warn about this safe use of a synchronous mutex; it would only become
/// a runtime concern if locking could block an executor thread.
#[derive(Clone)]
pub struct Variable(Arc<Mutex<Shared_variable_definition>>);

impl Variable {
    pub(crate) fn new(definition: Variable_definition) -> Self {
        Self(Arc::new(Mutex::new(Arc::new(Mutex::new(definition)))))
    }

    pub(crate) fn solver(
        definition: VariableDefinition,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Self {
        let variable_type = match definition.get_min() == definition.get_max() {
            true => Variable_type::Static(definition.get_min()),
            false => Variable_type::Solver(definition),
        };

        Self::new(Variable_definition {
            variable_type,
            name: name.into(),
            path: path.into(),
            component_path: component_path.into(),
        })
    }

    pub fn set_static(&self, value: f64) {
        self.definition()
            .lock()
            .expect("layout variable definition poisoned")
            .variable_type = Variable_type::Static(value);
    }

    /// Creates a distinct symbolic handle which initially points to the same definition.
    pub(crate) fn shared(&self) -> Self {
        Self(Arc::new(Mutex::new(self.definition())))
    }

    /// Repoints this symbolic handle and every expression holding it to another definition.
    pub(crate) fn point_to(&self, variable: &Self) {
        let definition = variable.definition();
        *self.0.lock().expect("layout variable handle poisoned") = definition;
    }

    #[cfg(test)]
    pub(crate) fn points_to(&self, variable: &Self) -> bool {
        Arc::ptr_eq(&self.definition(), &variable.definition())
    }

    pub(crate) fn definition(&self) -> Shared_variable_definition {
        Arc::clone(&self.0.lock().expect("layout variable handle poisoned"))
    }

    pub(crate) fn definition_id(&self) -> usize {
        Arc::as_ptr(&self.definition()) as usize
    }

    pub(crate) fn id(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

impl fmt::Debug for Variable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Variable").field(&self.id()).finish()
    }
}

impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Variable {}

impl Hash for Variable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}
