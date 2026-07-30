use color_eyre::eyre::Result;

use super::expression::Expression;
use crate::{component::context::Component_context, constraint};

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
                    .minimize_difference(expression, target, priority)
                    .await
            }
        }
    }
}
