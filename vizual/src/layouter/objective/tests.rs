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
        1,
    )?;

    assert_eq!(
        problem.objectives[1]
            .first()
            .and_then(|objective| objective.coefficients.get(&delta.variable)),
        Some(&-1.0)
    );

    problem.minimize_delta(Expression::from(0.0), 1.0, delta, 1)?;
    problem.minimize_delta(Expression::from(0.0), 1.0, delta, 1)?;

    assert_eq!(problem.objectives[1].len(), 3);
    assert!(
        problem.objectives[1]
            .iter()
            .all(|objective| objective.coefficients.get(&delta.variable) == Some(&-1.0))
    );
    Ok(())
}
