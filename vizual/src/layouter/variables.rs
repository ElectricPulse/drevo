use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Mutex,
};

use good_lp::{ProblemVariables, Variable as Solver_variable, VariableDefinition};

use super::variable::Variable;
use crate::component::debug::Component_tree;

#[derive(Clone)]
struct Variable_metadata {
    name: String,
    path: String,
    component_path: String,
}

#[derive(Default)]
struct Variable_registry {
    problem: ProblemVariables,
    metadata: HashMap<Solver_variable, Variable_metadata>,
    order: Vec<Solver_variable>,
}

/// Owns the `good_lp` problem variables used by a layout problem.
///
/// Definitions are handed directly to [`ProblemVariables::add`] and are never retained in a
/// second symbolic-variable representation. The small metadata table exists only for diagnostics.
#[derive(Default)]
pub struct Variables {
    registry: Mutex<Variable_registry>,
}

impl Variables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make(
        &self,
        definition: VariableDefinition,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let mut registry = self
            .registry
            .lock()
            .expect("layout variable registry poisoned");
        let variable = registry.problem.add(definition);
        let _ = registry.metadata.insert(
            variable,
            Variable_metadata {
                name: name.into(),
                path: path.into(),
                component_path: component_path.into(),
            },
        );
        registry.order.push(variable);
        Variable::new(variable)
    }

    pub fn make_independent(
        &self,
        definition: VariableDefinition,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let mut variable = self.make(definition, name, path, component_path);
        variable.make_independent();
        variable
    }

    pub(crate) fn problem(&self) -> ProblemVariables {
        self.registry
            .lock()
            .expect("layout variable registry poisoned")
            .problem
            .clone()
    }

    pub(crate) fn all(&self) -> Vec<Solver_variable> {
        self.registry
            .lock()
            .expect("layout variable registry poisoned")
            .order
            .clone()
    }

    pub(crate) fn len(&self) -> usize {
        self.registry
            .lock()
            .expect("layout variable registry poisoned")
            .problem
            .len()
    }

    pub(crate) fn name(&self, variable: Solver_variable) -> String {
        self.registry
            .lock()
            .expect("layout variable registry poisoned")
            .metadata
            .get(&variable)
            .map(|metadata| metadata.name.clone())
            .unwrap_or_else(|| format!("{variable:?}"))
    }

    pub(crate) fn component_tree(
        &self,
        variables: &HashSet<Solver_variable>,
        tree: &Component_tree,
    ) -> Vec<(usize, String, Option<String>)> {
        let registry = self
            .registry
            .lock()
            .expect("layout variable registry poisoned");
        let metadata = variables
            .iter()
            .filter_map(|variable| registry.metadata.get(variable))
            .collect::<Vec<_>>();
        let mut component_paths = BTreeSet::new();

        for metadata in &metadata {
            if metadata.component_path.is_empty() {
                continue;
            }

            let mut component_path = String::new();
            for component in metadata.component_path.split('.') {
                if !component_path.is_empty() {
                    component_path.push('.');
                }
                component_path.push_str(component);
                let _ = component_paths.insert(component_path.clone());
            }
        }

        if tree.is_empty() {
            let sources = metadata
                .iter()
                .filter(|metadata| !metadata.component_path.is_empty())
                .map(|metadata| (metadata.component_path.clone(), metadata.path.clone()))
                .collect::<HashMap<_, _>>();

            return component_paths
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
                .collect();
        }

        tree.iter()
            .filter(|component| component_paths.contains(&component.component_path))
            .map(|component| {
                (
                    component.depth,
                    component.name.clone(),
                    Some(component.source_path.clone()),
                )
            })
            .collect()
    }
}
