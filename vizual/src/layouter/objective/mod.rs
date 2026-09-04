use color_eyre::eyre::{Result, ensure, eyre};

use super::{Formula, PRIORITY_LEVELS, Problem, expression::Expression, variable::Variable};
use crate::constraint;

#[cfg(test)]
mod tests;

pub type Delta = Variable;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Objective {
    Maximize,
    Minimize,
    #[default]
    MinimizeDelta,
}

impl Objective {
    pub fn apply(
        self,
        formula: &mut Formula,
        expression: Expression,
        target: f64,
        delta: Delta,
        priority: usize,
    ) -> Result<()> {
        match self {
            Self::Maximize => {
                formula.constrain(crate::id!(), constraint!(expression.clone() <= target))?;
                formula.maximize(crate::id!(), expression, priority)
            }
            Self::Minimize => {
                formula.constrain(crate::id!(), constraint!(expression.clone() >= target))?;
                formula.minimize(crate::id!(), expression, priority)
            }
            Self::MinimizeDelta => {
                formula.minimize_delta(crate::id!(), expression, target, delta, priority)
            }
        }
    }
}

macro_rules! formula_objectives {
    ($type:ty) => {
        impl $type {
            pub fn maximize(
                &mut self,
                expression: impl Into<Expression>,
                priority: usize,
            ) -> Result<()> {
                if priority >= PRIORITY_LEVELS {
                    return Err(eyre!(
                        "Layout objective priority {priority} is outside the supported range 0..{}",
                        PRIORITY_LEVELS - 1
                    ));
                }

                self.objectives[priority].push(expression.into());
                Ok(())
            }

            pub fn minimize(
                &mut self,
                expression: impl Into<Expression>,
                priority: usize,
            ) -> Result<()> {
                self.maximize(expression.into() * -1.0, priority)
            }

            pub fn minimize_delta(
                &mut self,
                expression: impl Into<Expression>,
                target: f64,
                delta: Delta,
                priority: usize,
            ) -> Result<()> {
                ensure!(
                    target > 0.0,
                    "minimize-difference target must be greater than zero"
                );

                let expression = expression.into();
                let difference = (Expression::from(target) - expression) / target;
                self.constrain(constraint!(difference == delta));

                // Workaround: goals at the same priority are summed together at the end, so minimizing
                // the same delta multiple times increases its weight.
                self.minimize(delta, priority)
            }
        }
    };
}

formula_objectives!(Problem);

impl Problem {
    pub fn add_delta(
        &mut self,
        name: String,
        path: String,
        component_path: String,
        priority: usize,
    ) -> Result<Delta> {
        let delta =
            self.variables
                .make_independent_bounded(0.0, 1.0, false, name, path, component_path);
        self.minimize(delta, priority)?;
        Ok(delta)
    }
}
