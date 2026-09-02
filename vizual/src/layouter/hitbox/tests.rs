use std::sync::Arc;

use super::*;
use crate::{layouter::Formula, sync::Mutex};

#[test]
fn dimensions_are_derived_from_start_and_end() {
    let variables = Variables::new();
    let hitbox = Hitbox::new(
        &variables,
        "hitbox".to_string(),
        "hitbox".to_string(),
        "test".to_string(),
    );

    let width = hitbox.get_dimension(Direction::Horizontal);
    assert_eq!(
        width.coefficients.get(&hitbox.start.x.variable),
        Some(&-1.0)
    );
    assert_eq!(width.coefficients.get(&hitbox.end.x.variable), Some(&1.0));
    assert_eq!(width.coefficients.len(), 2);
}

#[tokio::test]
async fn shared_edges_are_constrained_to_the_parent_after_layout() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let parent = Hitbox::new(
        &variables,
        "parent".to_string(),
        "parent".to_string(),
        "test".to_string(),
    );
    let child = Hitbox::new(
        &variables,
        "child".to_string(),
        "child".to_string(),
        "test".to_string(),
    );
    let context =
        Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(&variables)))));

    child.constrain_shared(&parent, &context).await?;

    let problem = context.lock().await?;
    assert_eq!(problem.constraints.len(), 4);
    let horizontal_start = problem.constraints[0].expression();
    assert_eq!(
        horizontal_start.coefficients.get(&child.start.x.variable),
        Some(&1.0)
    );
    assert_eq!(
        horizontal_start.coefficients.get(&parent.start.x.variable),
        Some(&-1.0)
    );
    Ok(())
}

#[tokio::test]
async fn independent_edges_do_not_receive_parent_equalities() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let parent = Hitbox::new(
        &variables,
        "parent".to_string(),
        "parent".to_string(),
        "test".to_string(),
    );
    let mut child = Hitbox::new(
        &variables,
        "child".to_string(),
        "child".to_string(),
        "test".to_string(),
    );
    child.make_start_independent(Direction::Horizontal);
    let context =
        Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(&variables)))));

    child.constrain_shared(&parent, &context).await?;

    assert_eq!(context.lock().await?.constraints.len(), 3);
    Ok(())
}

#[tokio::test]
async fn static_dimensions_add_a_constraint_over_the_existing_edges() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let hitbox = Hitbox::new(
        &variables,
        "child".to_string(),
        "child".to_string(),
        "test".to_string(),
    );
    let context =
        Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(&variables)))));

    hitbox
        .set_static_dimension(&context, Direction::Horizontal, 42.0)
        .await?;

    let problem = context.lock().await?;
    let constraint = problem.constraints[0].expression();
    assert_eq!(constraint.coefficients.len(), 2);
    assert_eq!(constraint.constant, -42.0);
    Ok(())
}
