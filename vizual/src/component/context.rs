use std::{panic::Location, sync::Arc};

use color_eyre::eyre::{Result, ensure};

use crate::{
    constraint,
    layouter::{Problem, constraint::Constraint, expression::Expression, variable::Variable},
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
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        Ok(self
            .lock()
            .await?
            .add_non_negative_variable(name.into(), path, component_path))
    }

    #[track_caller]
    pub async fn add_binary_variable(&self, name: impl Into<String>) -> Result<Variable> {
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        Ok(self
            .lock()
            .await?
            .add_binary_variable(name.into(), path, component_path))
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
        self.maximize(expression * -1.0, priority).await
    }

    #[track_caller]
    /// Minimizes a normalized difference from the requested target through the problem's shared
    /// `delta`.
    pub async fn minimize_difference(
        &self,
        expression: impl Into<Expression>,
        target: f64,
        proportion: f64,
        priority: usize,
    ) -> Result<()> {
        ensure!(
            proportion > 0.0,
            "minimize-difference proportion must be greater than zero"
        );

        let difference = expression.into() - target;
        let inverse_difference = difference.clone() * -1.0;
        let delta = self.lock().await?.delta;
        let maximum_difference = Expression::from(delta) * proportion;

        self.constrain(constraint!(difference <= maximum_difference.clone()))
            .await?;
        self.constrain(constraint!(inverse_difference <= maximum_difference))
            .await?;
        self.minimize(Expression::from(delta), priority).await
    }
}
