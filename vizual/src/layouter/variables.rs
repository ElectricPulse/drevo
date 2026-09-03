use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::Mutex,
};

use super::variable::{SolverVariable, Variable};
use crate::component::debug::ComponentTree;

#[derive(Clone)]
pub struct VariableMetadata {
    pub name: String,
    pub path: String,
    pub component_path: String,
    pub lower: f64,
    pub upper: f64,
    pub is_integer: bool,
}

#[derive(Default)]
struct VariableRegistry {
    variables: Vec<VariableMetadata>,
}

/// Owns the solver variables used by a layout problem.
#[derive(Default)]
pub struct Variables {
    registry: Mutex<VariableRegistry>,
}

impl Variables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make(
        &self,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        self.make_bounded(
            -f64::INFINITY,
            f64::INFINITY,
            false,
            name,
            path,
            component_path,
        )
    }

    pub fn make_bounded(
        &self,
        lower: f64,
        upper: f64,
        is_integer: bool,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let mut registry = self
            .registry
            .lock()
            .expect("layout variable registry poisoned");
        let id = SolverVariable(registry.variables.len());
        registry.variables.push(VariableMetadata {
            name: name.into(),
            path: path.into(),
            component_path: component_path.into(),
            lower,
            upper,
            is_integer,
        });
        Variable::new(id)
    }

    pub fn make_independent(
        &self,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let mut variable = self.make(name, path, component_path);
        variable.make_independent();
        variable
    }

    pub fn make_independent_bounded(
        &self,
        lower: f64,
        upper: f64,
        is_integer: bool,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        let mut variable = self.make_bounded(lower, upper, is_integer, name, path, component_path);
        variable.make_independent();
        variable
    }

    pub(crate) fn metadata(&self, variable: SolverVariable) -> VariableMetadata {
        self.registry
            .lock()
            .expect("layout variable registry poisoned")
            .variables
            .get(variable.0)
            .cloned()
            .unwrap_or_else(|| VariableMetadata {
                name: format!("{variable:?}"),
                path: String::new(),
                component_path: String::new(),
                lower: -f64::INFINITY,
                upper: f64::INFINITY,
                is_integer: false,
            })
    }

    pub(crate) fn name(&self, variable: SolverVariable) -> String {
        self.registry
            .lock()
            .expect("layout variable registry poisoned")
            .variables
            .get(variable.0)
            .map(|metadata| metadata.name.clone())
            .unwrap_or_else(|| format!("{variable:?}"))
    }

    pub(crate) fn component_tree(
        &self,
        variables: &HashSet<SolverVariable>,
        tree: &ComponentTree,
    ) -> Vec<(usize, String, Option<String>)> {
        let registry = self
            .registry
            .lock()
            .expect("layout variable registry poisoned");
        let metadata = variables
            .iter()
            .filter_map(|variable| registry.variables.get(variable.0))
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
