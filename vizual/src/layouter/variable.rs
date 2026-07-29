use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use color_eyre::eyre::{Result, eyre};
use good_lp::{
    ProblemVariables, Variable as Solver_variable, VariableDefinition,
    variable as solver_variable_definition,
};

use super::screen::SCREEN;
use crate::geometry::Size;

const FIRST_DYNAMIC_VARIABLE_INDEX: usize = 3;

/// A stable index into [`Variables`].
///
/// Index `0` is unused, index `1` is permanently reserved for screen width, and index `2` is
/// permanently reserved for screen height. Dynamic variables always use index `3` or greater.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Variable {
    index: usize,
}

impl Variable {
    pub(crate) const fn new(index: usize) -> Self {
        Self { index }
    }

    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(Clone)]
struct Variable_metadata {
    name: String,
    path: String,
    component_path: String,
}

#[derive(Default)]
struct Variable_store {
    definitions: HashMap<usize, VariableDefinition>,
    metadata: HashMap<usize, Variable_metadata>,
}

pub(crate) type Solver_variables = HashMap<usize, Solver_variable>;

/// Definitions for stable layout variables shared across relayout-created problems.
#[derive(Default)]
pub struct Variables {
    store: Mutex<Variable_store>,
}

impl Variables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &self,
        definition: VariableDefinition,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let mut store = self.store.lock().expect("layout variable store poisoned");
        let index = (FIRST_DYNAMIC_VARIABLE_INDEX..)
            .find(|index| !store.definitions.contains_key(index))
            .expect("layout variable index space exhausted");
        let _ = store.definitions.insert(index, definition);
        let _ = store.metadata.insert(
            index,
            Variable_metadata {
                name: name.into(),
                path: path.into(),
                component_path: component_path.into(),
            },
        );
        Variable::new(index)
    }

    pub fn add_non_negative(
        &self,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let name = name.into();
        self.add(
            solver_variable_definition().min(0).name(name.clone()),
            name,
            path,
            component_path,
        )
    }

    pub fn add_binary(
        &self,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let name = name.into();
        self.add(
            solver_variable_definition().binary().name(name.clone()),
            name,
            path,
            component_path,
        )
    }

    pub fn remove(&self, variable: Variable) {
        if variable.index < FIRST_DYNAMIC_VARIABLE_INDEX {
            return;
        }

        let mut store = self.store.lock().expect("layout variable store poisoned");
        let _ = store.definitions.remove(&variable.index);
        let _ = store.metadata.remove(&variable.index);
    }

    pub(crate) fn name(&self, variable: Variable) -> String {
        match variable {
            variable if variable == SCREEN.width => "screen width".to_string(),
            variable if variable == SCREEN.height => "screen height".to_string(),
            variable => self
                .store
                .lock()
                .expect("layout variable store poisoned")
                .metadata
                .get(&variable.index)
                .map(|metadata| metadata.name.clone())
                .unwrap_or_else(|| format!("variable {}", variable.index)),
        }
    }

    pub(crate) fn component_paths(
        &self,
        variables: &HashSet<Variable>,
    ) -> impl Iterator<Item = (String, String)> {
        let store = self.store.lock().expect("layout variable store poisoned");
        store
            .metadata
            .iter()
            .filter_map(|(index, metadata)| {
                match (
                    metadata.component_path.is_empty(),
                    variables.contains(&Variable::new(*index)),
                ) {
                    (false, true) => Some((metadata.component_path.clone(), metadata.path.clone())),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub(crate) fn create_solver_variables(
        &self,
        referenced: &HashSet<Variable>,
        screen: Option<Size>,
    ) -> Result<(ProblemVariables, Solver_variables)> {
        // Dismounting should keep stale definitions out of this registry already. Filtering again
        // is probably unnecessary, but it guarantees that a priority solve only materializes
        // variables referenced by the current symbolic layout.
        let store = self.store.lock().expect("layout variable store poisoned");
        let mut problem_variables = ProblemVariables::new();
        let mut solver_variables = HashMap::new();
        let mut referenced = referenced.iter().copied().collect::<Vec<_>>();
        referenced.sort_unstable();

        for indexed_variable in referenced {
            let definition = match indexed_variable {
                variable if variable == SCREEN.width && screen.is_some() => continue,
                variable if variable == SCREEN.height && screen.is_some() => continue,
                variable if variable == SCREEN.width => {
                    solver_variable_definition().min(0).name("screen width")
                }
                variable if variable == SCREEN.height => {
                    solver_variable_definition().min(0).name("screen height")
                }
                variable => store
                    .definitions
                    .get(&variable.index)
                    .cloned()
                    .ok_or_else(|| {
                        eyre!(
                            "Layout variable {} is referenced but no longer registered",
                            variable.index
                        )
                    })?,
            };
            let solver_variable = problem_variables.add(definition);
            let _ = solver_variables.insert(indexed_variable.index, solver_variable);
        }

        Ok((problem_variables, solver_variables))
    }
}
