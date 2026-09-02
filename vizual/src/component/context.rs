use std::{panic::Location, sync::Arc};

use color_eyre::eyre::Result;

use crate::{
    layouter::{
        Formula, constraint::Constraint, expression::Expression, objective::Delta,
        variable::Variable,
    },
    sync::{Mutex, MutexGuard},
};

#[derive(Clone)]
pub struct Component_context {
    pub formula: Arc<Mutex<Formula>>,
    pub component_path: Vec<String>,
}

impl Component_context {
    pub fn new(formula: Arc<Mutex<Formula>>) -> Self {
        Self {
            formula,
            component_path: Vec::new(),
        }
    }

    pub(crate) fn path(location: &'static Location<'static>) -> String {
        format!("{}:{}", location.file(), location.line())
    }

    pub(crate) fn name_constraint(constraint: Constraint, name: impl Into<String>) -> Constraint {
        constraint.set_name(name.into())
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, Formula>> {
        self.formula.lock().await
    }

    #[track_caller]
    pub async fn add_binary_variable(&self, name: impl Into<String>) -> Result<Variable> {
        let name = name.into();
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        let mut formula = self.lock().await?;
        let variable =
            formula
                .registry
                .make_independent_bounded(0.0, 1.0, true, name, path, component_path);
        formula.variables.push(variable);
        Ok(variable)
    }

    #[track_caller]
    pub(crate) async fn add_nonnegative_variable(
        &self,
        name: impl Into<String>,
    ) -> Result<Variable> {
        let name = name.into();
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        let mut formula = self.lock().await?;
        let variable = formula.registry.make_independent_bounded(
            0.0,
            f64::INFINITY,
            false,
            name,
            path,
            component_path,
        );
        formula.variables.push(variable);
        Ok(variable)
    }

    #[track_caller]
    pub async fn add_delta(&self, name: impl Into<String>, priority: usize) -> Result<Delta> {
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

    #[track_caller]
    pub async fn minimize_delta(
        &self,
        expression: impl Into<Expression>,
        target: f64,
        delta: Delta,
        priority: usize,
    ) -> Result<()> {
        self.lock()
            .await?
            .minimize_delta(expression, target, delta, priority)
    }

    #[track_caller]
    pub async fn minimize(&self, expression: impl Into<Expression>, priority: usize) -> Result<()> {
        self.lock().await?.minimize(expression, priority)
    }

    #[track_caller]
    pub async fn maximize(&self, expression: impl Into<Expression>, priority: usize) -> Result<()> {
        self.lock().await?.maximize(expression, priority)
    }
}
