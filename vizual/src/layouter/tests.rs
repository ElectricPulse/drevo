use super::*;

#[test]
fn higher_objective_priority_wins_lexicographically() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Problem::new(Arc::clone(&variables));
    let x = variables.make_independent_bounded(
        0.0,
        10.0,
        false,
        "x",
        "test",
        "test",
    );
    let y = variables.make_independent_bounded(
        0.0,
        10.0,
        false,
        "y",
        "test",
        "test",
    );
    let constraints = vec![constraint!(x.clone() + y.clone() <= 10)];
    let objectives = vec![
        (1, Expression::from(x.clone())),
        (0, Expression::from(y.clone())),
    ];

    let solution = problem.solve_objectives(&constraints, &objectives)?;

    assert_eq!(solution.value(&x), 10.0);
    assert_eq!(solution.value(&y), 0.0);
    Ok(())
}

#[tokio::test]
async fn priority_results_do_not_persist_between_solves() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let mut problem = Problem::new(Arc::clone(&variables));
    let root = Hitbox::new(
        &variables,
        "root".to_string(),
        "root".to_string(),
        "test".to_string(),
    );
    let child = Hitbox::new(
        &variables,
        "child".to_string(),
        "root.child".to_string(),
        "test".to_string(),
    );
    problem.constrain(constraint!(
        child.get_start_position(Direction::Horizontal)
            == root.get_start_position(Direction::Horizontal)
    ));
    problem.constrain(constraint!(
        child.get_end_position(Direction::Horizontal)
            == root.get_end_position(Direction::Horizontal)
    ));
    problem.minimize(child.get_dimension(Direction::Horizontal), 0)?;

    let component_tree = Vec::new();
    let first = problem
        .solve(root.clone(), Size::new(800.0, 600.0), &component_tree)
        .await?;
    assert_eq!(
        first.eval(&child.get_dimension(Direction::Horizontal)),
        800.0
    );

    let second = problem
        .solve(root, Size::new(801.0, 600.0), &component_tree)
        .await?;
    assert_eq!(
        second.eval(&child.get_dimension(Direction::Horizontal)),
        801.0
    );

    Ok(())
}

#[tokio::test]
async fn overconstrained_problem_isolates_conflicts_via_iis() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let mut problem = Problem::new(Arc::clone(&variables));
    let x = variables.make_independent_bounded(0.0, 100.0, false, "x", "test", "test");
    
    // Infeasible contradictory constraints: x >= 20 and x <= 10
    problem.constrain(constraint!(x.clone() >= 20).set_name("x_min_20".to_string()));
    problem.constrain(constraint!(x.clone() <= 10).set_name("x_max_10".to_string()));

    let component_tree = Vec::new();
    let root = Hitbox::new(
        &variables,
        "root".to_string(),
        "root".to_string(),
        "test".to_string(),
    );
    let result = problem.solve(root, Size::new(800.0, 600.0), &component_tree).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Layout is overconstrained"),
        "expected overconstrained error, got: {error_msg}"
    );
    assert!(
        error_msg.contains("x_min_20") && error_msg.contains("x_max_10"),
        "expected conflicting constraint names in error message, got: {error_msg}"
    );
    Ok(())
}
