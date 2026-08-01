use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use color_eyre::eyre::{Result, eyre};
use good_lp::{ProblemVariables, Variable as Solver_variable, VariableDefinition};

use super::{screen::SCREEN, variable::Variable};
use crate::geometry::Size;

const FIRST_DYNAMIC_VARIABLE_INDEX: usize = 3;

#[derive(Clone)]
pub struct Variable_definition {
    solver: VariableDefinition,
    name: String,
    path: String,
    component_path: String,
}

enum Resolved_variable {
    Constant(f64),
    Variable(Solver_variable),
}

pub(crate) type Resolved_variables = HashMap<usize, Resolved_variable>;

/// Definitions for stable layout variables shared across relayout-created problems.
#[derive(Default)]
pub struct Variables {
    definitions: Mutex<HashMap<usize, Variable_definition>>,
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
        let mut definitions = self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned");
        let index = (FIRST_DYNAMIC_VARIABLE_INDEX..)
            .find(|index| !definitions.contains_key(index))
            .expect("layout variable index space exhausted");

        let _ = definitions.insert(
            index,
            Variable_definition {
                solver: definition,
                name: name.into(),
                path: path.into(),
                component_path: component_path.into(),
            },
        );
        Variable::new(index)
    }

    pub fn set_static(&self, variable: Variable, value: f64) {
        let mut definitions = self
            .definitions
            .lock()
            .expect("Layout variable definitions poisoned");

        let record = definitions
            .get_mut(&variable.index())
            .expect("Layout variables are broken");
        record.solver = record.solver.clone().clamp(value, value);
    }

    pub fn remove(&self, variable: Variable) {
        if variable.index() < FIRST_DYNAMIC_VARIABLE_INDEX {
            return;
        }

        let _ = self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned")
            .remove(&variable.index());
    }

    pub(crate) fn name(&self, variable: Variable) -> String {
        match variable {
            variable if variable == SCREEN.width => "screen width".to_string(),
            variable if variable == SCREEN.height => "screen height".to_string(),
            variable => self
                .definitions
                .lock()
                .expect("layout variable definitions poisoned")
                .get(&variable.index())
                .map(|definition| definition.name.clone())
                .unwrap_or_else(|| format!("variable {}", variable.index())),
        }
    }

    pub(crate) fn component_paths(
        &self,
        variables: &HashSet<Variable>,
    ) -> impl Iterator<Item = (String, String)> {
        let definitions = self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned");
        definitions
            .iter()
            .filter_map(|(index, definition)| {
                match (
                    definition.component_path.is_empty(),
                    variables.contains(&Variable::new(*index)),
                ) {
                    (false, true) => {
                        Some((definition.component_path.clone(), definition.path.clone()))
                    }
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
    ) -> Result<Resolved_variables> {
        // Dismounting should keep stale definitions out of this registry already. Filtering again
        // is probably unnecessary, but it guarantees that a priority solve only materializes
        // variables referenced by the current symbolic layout.
        let definitions = self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned");

        let mut problem_variables = ProblemVariables::new();
        let mut solver_variables = HashMap::new();
        let mut referenced = referenced.iter().copied().collect::<Vec<_>>();
        referenced.sort_unstable();

        for indexed_variable in referenced {
            let definition = match indexed_variable {
                variable => definitions
                    .get(&variable.index())
                    .map(|definition| definition.solver.clone())
                    .ok_or_else(|| {
                        eyre!(
                            "Layout variable {} is referenced but no longer registered",
                            variable.index()
                        )
                    })?,
            };

            let solver_variable = 'value: {
                let has_lower_bound = definition.get_min().is_finite();
                let has_upper_bound = definition.get_max().is_finite();

                if has_lower_bound && has_upper_bound {
                    break 'value Resolved_variable::Constant(definition.get_min());
                }

                let solver_variable = problem_variables.add(definition);
                Resolved_variable::Variable(solver_variable)
            };

            let _ = solver_variables.insert(indexed_variable.index(), solver_variable);
        }

        Ok(solver_variables)
    }
}
