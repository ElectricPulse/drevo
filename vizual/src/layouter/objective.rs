use color_eyre::eyre::Result;

use super::{expression::Expression, variable::Variable};
use crate::{component::context::Component_context, constraint};

pub type Delta = Option<Variable>;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Objective {
    Maximize,
    Minimize,
    #[default]
    Minimize_difference,
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
                problem.maximize(expression, priority).await
            }
            Self::Minimize => {
                problem
                    .constrain(constraint!(expression.clone() >= target))
                    .await?;
                problem.minimize(expression, priority).await
            }
            Self::Minimize_difference => {
                problem
                    .minimize_difference(expression, target, delta, priority)
                    .await
            }
        }
    }
}
