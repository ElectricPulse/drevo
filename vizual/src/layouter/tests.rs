use super::*;
use good_lp::VariableDefinition;

#[test]
fn higher_objective_priority_wins_lexicographically() -> Result<()> {
    let variables = Arc::new(Variables::new());
    let problem = Problem::new(Arc::clone(&variables));
    let x = variables.make_independent(
        VariableDefinition::new().min(0).max(10),
        "x",
        "test",
        "test",
    );
    let y = variables.make_independent(
        VariableDefinition::new().min(0).max(10),
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
