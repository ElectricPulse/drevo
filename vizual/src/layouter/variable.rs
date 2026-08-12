use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use good_lp::VariableDefinition;

use super::variables::Variable_type;

#[derive(Clone)]
pub struct Variable_definition {
    pub(crate) variable_type: Variable_type<VariableDefinition>,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) component_path: String,
}

/// A symbolic variable backed directly by its shared definition.
///
/// These deliberately use `std::sync::Mutex`, not the crate's Tokio mutex: definition lookup and
/// mutation are synchronous operations used while constructing expressions and solver state. No
/// guard may live across an `.await`.
#[derive(Clone)]
pub struct Variable(Arc<Mutex<Variable_definition>>);

impl Variable {
    pub(crate) fn new(definition: Variable_definition) -> Self {
        Self(Arc::new(Mutex::new(definition)))
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

    #[cfg(test)]
    pub(crate) fn points_to(&self, variable: &Self) -> bool {
        Arc::ptr_eq(&self.0, &variable.0)
    }

    pub(crate) fn definition(&self) -> Arc<Mutex<Variable_definition>> {
        Arc::clone(&self.0)
    }

    pub(crate) fn definition_id(&self) -> usize {
        self.id()
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
