use color_eyre::eyre::{Result, ensure, eyre};
use good_lp::VariableDefinition;

use super::{PRIORITY_LEVELS, Problem, expression::Expression, variable::Variable};
use crate::{component::context::Component_context, constraint};

pub type Delta = Variable;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Objective {
    Maximize,
    Minimize,
    #[default]
    Minimize_delta,
}

pub(crate) fn minimize(
    problem: &mut Problem,
    expression: Expression,
    priority: usize,
) -> Result<()> {
    problem.maximize(expression * -1.0, priority)
}

impl Objective {
    pub async fn apply(
        self,
        problem: &Component_context,
        expression: Expression,
        target: f64,
        delta: Delta,
        priority: usize,
    ) -> Result<()> {
        match self {
            Self::Maximize => {
                problem
                    .constrain(constraint!(expression.clone() <= target))
                    .await?;
                problem.lock().await?.maximize(expression, priority)
            }
            Self::Minimize => {
                problem
                    .constrain(constraint!(expression.clone() >= target))
                    .await?;
                minimize(&mut *problem.lock().await?, expression, priority)
            }
            Self::Minimize_delta => {
                problem
                    .minimize_delta(expression, target, delta, priority)
                    .await
            }
        }
    }
}

impl Problem {
    pub(crate) fn maximize(&mut self, expression: Expression, priority: usize) -> Result<()> {
        if priority >= PRIORITY_LEVELS {
            return Err(eyre!(
                "Layout objective priority {priority} is outside the supported range 0..{}",
                PRIORITY_LEVELS - 1
            ));
        }

        self.objectives[priority].push(expression);
        Ok(())
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
        self.constrain(constraint!(difference == delta.clone()));

        // Workaround: goals at the same priority are summed together at the end, so minimizing
        // the same delta multiple times increases its weight.
        minimize(self, Expression::from(delta), priority)
    }

    pub fn add_delta(
        &mut self,
        name: String,
        path: String,
        component_path: String,
        priority: usize,
    ) -> Result<Delta> {
        let delta = self.variables.make_independent(
            VariableDefinition::new().min(0).max(1).name(name.clone()),
            name,
            path,
            component_path,
        );

        minimize(self, Expression::from(delta.clone()), priority)?;
        Ok(delta)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::layouter::variables::Variables;

    #[test]
    fn each_delta_use_adds_separate_objective() -> Result<()> {
        let variables = Arc::new(Variables::new());
        let mut problem = Problem::new(Arc::clone(&variables));
        let delta = problem.add_delta(
            "shared-delta".to_string(),
            "test".to_string(),
            "test".to_string(),
            2,
        )?;

        assert_eq!(
            problem.objectives[2]
                .first()
                .and_then(|objective| objective.coefficients.get(&delta)),
            Some(&-1.0)
        );

        problem.minimize_delta(Expression::from(0.0), 1.0, delta.clone(), 2)?;
        problem.minimize_delta(Expression::from(0.0), 1.0, delta.clone(), 2)?;

        assert_eq!(problem.objectives[2].len(), 3);
        assert!(
            problem.objectives[2]
                .iter()
                .all(|objective| objective.coefficients.get(&delta) == Some(&-1.0))
        );
        Ok(())
    }
}
