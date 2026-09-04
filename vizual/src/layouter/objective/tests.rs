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
            .and_then(|objective| objective.coefficients.get(&delta.variable)),
        Some(&1.0)
    );

    problem.minimize_delta(Expression::from(0.0), 1.0, delta, 2)?;
    problem.minimize_delta(Expression::from(0.0), 1.0, delta, 2)?;

    assert_eq!(problem.objectives[2].len(), 3);
    assert!(
        problem.objectives[2]
            .iter()
            .all(|objective| objective.coefficients.get(&delta.variable) == Some(&1.0))
    );
    Ok(())
}
