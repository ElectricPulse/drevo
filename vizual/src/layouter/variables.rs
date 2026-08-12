use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ops::Mul,
};

use color_eyre::eyre::Result;
use good_lp::{
    Expression as Solver_expression, ProblemVariables, Variable as Solver_variable,
    VariableDefinition,
};

use super::{screen::Screen, variable::Variable};
use crate::component::debug::Component_tree;

#[derive(Clone)]
pub(crate) enum Variable_type<Variable> {
    Static(f64),
    Solver(Variable),
}

impl Mul<f64> for Variable_type<Solver_variable> {
    type Output = Solver_expression;

    fn mul(self, rhs: f64) -> Self::Output {
        match self {
            Self::Static(value) => Solver_expression::from(value * rhs),
            Self::Solver(variable) => variable * rhs,
        }
    }
}

pub(crate) type Solver_variables = HashMap<Variable, Variable_type<Solver_variable>>;

/// Creates layout variables. Definitions are owned by the returned variables rather than stored
/// in a central registry.
pub struct Variables {
    pub(crate) screen: Screen,
}

impl Default for Variables {
    fn default() -> Self {
        Self {
            screen: Screen::new(),
        }
    }
}

impl Variables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make_independent(
        &self,
        definition: VariableDefinition,
        name: impl Into<String>,
        path: impl Into<String>,
        component_path: impl Into<String>,
    ) -> Variable {
        Variable::solver(definition, name, path, component_path)
    }

    pub(crate) fn set_type(
        &self,
        variable: &Variable,
        variable_type: Variable_type<VariableDefinition>,
    ) {
        variable
            .definition()
            .lock()
            .expect("layout variable definition poisoned")
            .variable_type = variable_type;
    }

    pub(crate) fn get_type(&self, variable: &Variable) -> Variable_type<VariableDefinition> {
        variable
            .definition()
            .lock()
            .expect("layout variable definition poisoned")
            .variable_type
            .clone()
    }

    pub(crate) fn name(&self, variable: &Variable) -> String {
        variable
            .definition()
            .lock()
            .expect("layout variable definition poisoned")
            .name
            .clone()
    }

    pub(crate) fn component_tree(
        &self,
        variables: &HashSet<Variable>,
        tree: &Component_tree,
    ) -> Vec<(usize, String, Option<String>)> {
        let definitions = variables
            .iter()
            .map(Variable::definition)
            .collect::<Vec<_>>();
        let mut component_paths = BTreeSet::new();

        for definition in &definitions {
            let definition = definition
                .lock()
                .expect("layout variable definition poisoned");
            if definition.component_path.is_empty() {
                continue;
            }

            let mut component_path = String::new();
            for component in definition.component_path.split('.') {
                if !component_path.is_empty() {
                    component_path.push('.');
                }
                component_path.push_str(component);
                let _ = component_paths.insert(component_path.clone());
            }
        }

        if tree.is_empty() {
            let sources = definitions
                .iter()
                .filter_map(|definition| {
                    let definition = definition
                        .lock()
                        .expect("layout variable definition poisoned");
                    (!definition.component_path.is_empty())
                        .then(|| (definition.component_path.clone(), definition.path.clone()))
                })
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

    pub(crate) fn create_solver_variables(
        &self,
        referenced: &HashSet<Variable>,
    ) -> Result<(ProblemVariables, Solver_variables)> {
        let mut problem_variables = ProblemVariables::new();
        let mut solver_variables = HashMap::new();
        let mut referenced = referenced.iter().cloned().collect::<Vec<_>>();
        referenced.sort_unstable_by_key(Variable::id);

        for variable in referenced {
            let definition = variable.definition();
            let variable_type = match &definition
                .lock()
                .expect("layout variable definition poisoned")
                .variable_type
            {
                Variable_type::Static(value) => Variable_type::Static(*value),
                Variable_type::Solver(definition) => {
                    Variable_type::Solver(problem_variables.add(definition.clone()))
                }
            };

            let _ = solver_variables.insert(variable, variable_type);
        }

        Ok((problem_variables, solver_variables))
    }
}
