use std::{panic::Location, sync::Arc};

use color_eyre::eyre::Result;
use good_lp::VariableDefinition;

use crate::{
    layouter::{
        Problem, constraint::Constraint, expression::Expression, objective::Delta,
        variable::Variable,
    },
    sync::{Mutex, MutexGuard},
};

#[derive(Clone)]
pub struct Component_context {
    pub problem: Arc<Mutex<Problem>>,
    pub component_path: Vec<String>,
}

impl Component_context {
    pub fn new(problem: Arc<Mutex<Problem>>) -> Self {
        Self {
            problem,
            component_path: Vec::new(),
        }
    }

    pub(crate) fn path(location: &'static Location<'static>) -> String {
        format!("{}:{}", location.file(), location.line())
    }

    pub(crate) fn name_constraint(constraint: Constraint, name: impl Into<String>) -> Constraint {
        constraint.set_name(name.into())
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, Problem>> {
        self.problem.lock().await
    }

    #[track_caller]
    pub async fn add_non_negative_variable(&self, name: impl Into<String>) -> Result<Variable> {
        let name = name.into();
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        let problem = self.lock().await?;
        Ok(problem.variables.add(
            VariableDefinition::new().min(0).name(name.clone()),
            name,
            path,
            component_path,
        ))
    }

    #[track_caller]
    pub async fn add_binary_variable(&self, name: impl Into<String>) -> Result<Variable> {
        let name = name.into();
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        let problem = self.lock().await?;
        Ok(problem.variables.add(
            VariableDefinition::new().binary().name(name.clone()),
            name,
            path,
            component_path,
        ))
    }

    #[track_caller]
    pub async fn add_delta(&self, name: impl Into<String>, priority: usize) -> Result<Variable> {
        let name = name.into();
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        self.lock()
            .await?
            .add_delta(name, path, component_path, priority)
    }

    #[track_caller]
    pub async fn constrain(&self, constraint: Constraint) -> Result<()> {
        let constraint = match constraint.name() {
            Some(_) => constraint,
            None => Self::name_constraint(constraint, Self::path(Location::caller())),
        };
        self.lock().await?.constrain(constraint);
        Ok(())
    }

    pub async fn maximize(&self, expression: Expression, priority: usize) -> Result<()> {
        self.lock().await?.maximize(expression, priority)
    }

    pub async fn minimize(&self, expression: Expression, priority: usize) -> Result<()> {
        self.lock().await?.minimize(expression, priority)
    }

    #[track_caller]
    pub async fn minimize_difference(
        &self,
        expression: impl Into<Expression>,
        target: f64,
        delta: Delta,
        priority: usize,
    ) -> Result<()> {
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        let mut problem = self.lock().await?;
        let delta = match delta {
            Some(delta) => delta,
            None => problem.add_delta(
                "minimize-difference-delta".to_string(),
                path,
                component_path,
                priority,
            )?,
        };
        problem.minimize_difference(expression, target, delta, priority)
    }
}
