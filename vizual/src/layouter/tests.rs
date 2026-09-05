use super::*;

#[test]
fn goals_are_blended_by_priority_weight() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Problem::new(Arc::clone(&variables));
    let x = variables.make_independent_bounded(0.0, 10.0, false, "x", "test", "test");
    let y = variables.make_independent_bounded(0.0, 10.0, false, "y", "test", "test");
    let constraints = vec![constraint!(x.clone() + y.clone() == 10)];
    let objectives = vec![
        Goal {
            priority: 2,
            expression: Expression::from(x.clone()),
        },
        Goal {
            priority: 0,
            expression: Expression::from(y.clone()),
        },
    ];

    let solution = problem.solve_internal(&constraints, &objectives)?;

    assert_eq!(solution.value(&x), 0.0);
    assert_eq!(solution.value(&y), 10.0);
    Ok(())
}

#[tokio::test]
async fn weighted_underconstrained_report_uses_weighted_objective() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Problem::new(Arc::clone(&variables));
    let unbounded = variables.make("unbounded", "test", "test");
    let objective = Goal {
        priority: EXCESS_SPACE,
        expression: Expression::from(unbounded),
    };
    let component_tree = Vec::new();
    let error = problem
        .diagnose_objective(&[], &objective, ObjectiveLabel::Weighted, &component_tree)
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("weighted objective is unbounded"));
    assert!(!error.contains("priority 0 objective"));
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
    problem.minimize(child.get_dimension(Direction::Horizontal), SHRINK_WRAP)?;

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
    let result = problem
        .solve(root, Size::new(800.0, 600.0), &component_tree)
        .await;

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

#[tokio::test]
async fn scrolled_negative_coordinates_solve_successfully() -> Result<()> {
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
        "child".to_string(),
        "test".to_string(),
    );

    // Child is scrolled up by 150px relative to root start at y=0, placing child.start.y at -150
    problem.constrain(constraint!(
        child.get_start_position(Direction::Vertical)
            == root.get_start_position(Direction::Vertical) - 150.0
    ));
    problem.constrain(constraint!(
        child.get_end_position(Direction::Vertical)
            == child.get_start_position(Direction::Vertical) + 500.0
    ));

    let component_tree = Vec::new();
    let solution = problem
        .solve(root, Size::new(800.0, 600.0), &component_tree)
        .await?;

    assert_eq!(
        solution.eval(&child.get_start_position(Direction::Vertical)),
        -150.0
    );
    assert_eq!(
        solution.eval(&child.get_end_position(Direction::Vertical)),
        350.0
    );
    Ok(())
}

#[tokio::test]
async fn root_dimensions_minimizes_extra_root_size_to_window_size() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let mut problem = Problem::new(Arc::clone(&variables));
    let root = Hitbox::new(
        &variables,
        "root".to_string(),
        "root".to_string(),
        "test".to_string(),
    );

    let component_tree = Vec::new();
    let solution = problem
        .solve(root.clone(), Size::new(800.0, 600.0), &component_tree)
        .await?;

    assert_eq!(
        solution.eval(&root.get_dimension(Direction::Horizontal)),
        800.0
    );
    assert_eq!(
        solution.eval(&root.get_dimension(Direction::Vertical)),
        600.0
    );
    Ok(())
}

#[tokio::test]
async fn root_dimensions_allows_expansion_and_minimizes_excess() -> Result<()> {
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
        "child".to_string(),
        "test".to_string(),
    );
    problem.constrain(constraint!(
        child.get_dimension(Direction::Horizontal) == 1200.0
    ));
    problem.constrain(constraint!(
        root.get_dimension(Direction::Horizontal) >= child.get_dimension(Direction::Horizontal)
    ));

    let component_tree = Vec::new();
    let solution = problem
        .solve(root.clone(), Size::new(800.0, 600.0), &component_tree)
        .await?;

    assert_eq!(
        solution.eval(&root.get_dimension(Direction::Horizontal)),
        1200.0
    );
    assert_eq!(
        solution.eval(&root.get_dimension(Direction::Vertical)),
        600.0
    );
    Ok(())
}
