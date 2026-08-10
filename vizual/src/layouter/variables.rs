use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ops::Mul,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use color_eyre::eyre::{Result, eyre};
use good_lp::{
    Expression as Solver_expression, ProblemVariables, Variable as Solver_variable,
    VariableDefinition,
};

use super::{screen::SCREEN, variable::Variable};

const FIRST_DYNAMIC_VARIABLE_INDEX: usize = 3;

#[derive(Clone)]
pub struct Variable_definition {
    solver: VariableDefinition,
    static_value: Option<f64>,
    name: String,
    path: String,
    component_path: String,
}

#[derive(Clone)]
struct Component_definition {
    path: String,
    component_path: String,
}

#[derive(Clone, Copy)]
pub(crate) enum Resolved_variable {
    Constant(f64),
    Variable(Solver_variable),
}

// Only used in expression.rs to multiply resolved variables by their coefficients.
impl Mul<f64> for Resolved_variable {
    type Output = Solver_expression;

    fn mul(self, rhs: f64) -> Self::Output {
        match self {
            Self::Constant(value) => Solver_expression::from(value * rhs),
            Self::Variable(variable) => variable * rhs,
        }
    }
}

pub(crate) type Resolved_variables = HashMap<usize, Resolved_variable>;

/// Definitions for stable layout variables shared across relayout-created problems.
pub struct Variables {
    definitions: Mutex<HashMap<usize, Variable_definition>>,
    components: Mutex<HashMap<usize, Component_definition>>,
    next_component: AtomicUsize,
}

impl Default for Variables {
    fn default() -> Self {
        let definitions = [
            (SCREEN.width, "screen width"),
            (SCREEN.height, "screen height"),
        ]
        .into_iter()
        .map(|(variable, name)| {
            (
                variable.index(),
                Variable_definition {
                    solver: VariableDefinition::new().min(0).name(name),
                    static_value: None,
                    name: name.to_string(),
                    path: String::new(),
                    component_path: String::new(),
                },
            )
        })
        .collect();

        Self {
            definitions: Mutex::new(definitions),
            components: Mutex::new(HashMap::new()),
            next_component: AtomicUsize::new(0),
        }
    }
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
                static_value: None,
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
        record.static_value = Some(value);
    }

    pub(crate) fn clear_static(&self) {
        for definition in self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned")
            .values_mut()
        {
            definition.static_value = None;
        }
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

    pub(crate) fn static_value(&self, variable: Variable) -> Option<f64> {
        let definitions = self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned");
        let definition = definitions.get(&variable.index())?;

        definition.static_value.or_else(|| {
            (definition.solver.get_min() == definition.solver.get_max())
                .then_some(definition.solver.get_min())
        })
    }

    pub(crate) fn component_tree(
        &self,
        variables: &HashSet<Variable>,
    ) -> Vec<(usize, String, Option<String>)> {
        let definitions = self
            .definitions
            .lock()
            .expect("layout variable definitions poisoned");
        let sources = definitions
            .values()
            .filter(|definition| !definition.component_path.is_empty())
            .map(|definition| (definition.component_path.clone(), definition.path.clone()))
            .collect::<HashMap<_, _>>();
        let mut component_paths = BTreeSet::new();

        for definition in definitions.iter().filter_map(|(index, definition)| {
            (!definition.component_path.is_empty() && variables.contains(&Variable::new(*index)))
                .then_some(definition)
        }) {
            let mut component_path = String::new();
            for component in definition.component_path.split('.') {
                if !component_path.is_empty() {
                    component_path.push('.');
                }
                component_path.push_str(component);
                let _ = component_paths.insert(component_path.clone());
            }
        }

        component_paths
            .into_iter()
            .map(|component_path| {
                let depth = component_path.matches('.').count();
                let component = component_path
                    .rsplit_once('.')
                    .map_or(component_path.as_str(), |(_, component)| component)
                    .to_string();
                let source = sources.get(&component_path).cloned();
                (depth, component, source)
            })
            .collect()
    }

    pub(crate) fn create_solver_variables(
        &self,
        referenced: &HashSet<Variable>,
    ) -> Result<(ProblemVariables, Resolved_variables)> {
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
                variable => definitions.get(&variable.index()).ok_or_else(|| {
                    eyre!(
                        "Layout variable {} is referenced but no longer registered",
                        variable.index()
                    )
                })?,
            };

            let solver_variable = 'value: {
                if let Some(value) = definition.static_value {
                    break 'value Resolved_variable::Constant(value);
                }

                let definition = definition.solver.clone();
                if definition.get_min() == definition.get_max() {
                    break 'value Resolved_variable::Constant(definition.get_min());
                }

                let solver_variable = problem_variables.add(definition);
                Resolved_variable::Variable(solver_variable)
            };

            let _ = solver_variables.insert(indexed_variable.index(), solver_variable);
        }

        Ok((problem_variables, solver_variables))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_values_can_be_cleared_between_solves() {
        let variables = Variables::new();
        let variable = variables.add(VariableDefinition::new().min(0), "test", "", "");

        variables.set_static(variable, 42.0);
        assert_eq!(variables.static_value(variable), Some(42.0));

        variables.clear_static();
        assert_eq!(variables.static_value(variable), None);
    }

    #[test]
    fn component_tree_includes_parents_of_referenced_components() {
        let variables = Variables::new();
        let _parent = variables.add(
            VariableDefinition::new().min(0),
            "parent",
            "parent.rs:1",
            "c2",
        );
        let leaf = variables.add(
            VariableDefinition::new().min(0),
            "leaf",
            "leaf.rs:2",
            "c2.c3.c4",
        );

        assert_eq!(
            variables.component_tree(&HashSet::from([leaf])),
            vec![
                (0, "c2".to_string(), Some("parent.rs:1".to_string())),
                (1, "c3".to_string(), None),
                (2, "c4".to_string(), Some("leaf.rs:2".to_string())),
            ]
        );
    }
}
