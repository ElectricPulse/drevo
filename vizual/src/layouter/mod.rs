pub mod constraint;
pub mod constraints;
pub mod expression;
pub mod hitbox;
pub mod objective;
pub mod screen;
pub mod variable;
pub mod variables;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use color_eyre::eyre::{Result, eyre};
use futures::future::BoxFuture;
use good_lp::{
    Solution as Good_lp_solution, SolverModel as _, microlp,
    solvers::{ObjectiveDirection, ResolutionError},
};

use self::{
    constraint::Constraint,
    expression::Expression,
    hitbox::Hitbox,
    variable::Variable,
    variables::{Variable_type, Variables},
};
use crate::{
    component::{context::Component_context, debug::Component_tree},
    constraint,
    geometry::{Direction, Size},
    log::{log_duration, log_info},
};

// This is an async callback for the sake of being generic and allowing for more than x, y, width,
// height setting on child.
pub trait Setter_callback:
    Fn(f64) -> BoxFuture<'static, ()> + Send + Sync + dyn_clone::DynClone
{
}

impl<Callback> Setter_callback for Callback where
    Callback: Fn(f64) -> BoxFuture<'static, ()> + Send + Sync + Clone + 'static
{
}

dyn_clone::clone_trait_object!(Setter_callback);

pub type Setter = Box<dyn Setter_callback>;

const PRIORITY_LEVELS: usize = 3;
type Priority_objective = Vec<Expression>;
// As of this moment the usage of priorities has crystalized like this:
// 3 is for calculating minimum screen size as that will just minimize root hitbox - but you cannot access it
// 2 is for gaps, spaces, margins, paddings
// 1 is for elements that just want to fill the surrounding space after content looks like it wants to look like and for align
// 0 is for shrink wrap of parents around their children

pub trait Field: Send {
    fn set_from_solver(&mut self, value: f64);
    fn solver_value(&self) -> f64;
}

impl Field for f64 {
    fn set_from_solver(&mut self, value: f64) {
        *self = value.max(0.0);
    }

    fn solver_value(&self) -> f64 {
        *self
    }
}

#[derive(Clone)]
pub struct Solution {
    values: HashMap<usize, f64>,
}

impl Solution {
    pub fn value(&self, variable: &Variable) -> f64 {
        self.values
            .get(&variable.definition_id())
            .copied()
            .unwrap_or_default()
    }

    fn eval(&self, expression: &Expression) -> f64 {
        expression.eval_with(&self.values)
    }
}

pub struct Problem {
    constraints: Vec<Constraint>,
    objectives: [Priority_objective; PRIORITY_LEVELS],
    pub(crate) variables: Arc<Variables>,
}

impl Problem {
    pub fn new(variables: Arc<Variables>) -> Self {
        Self {
            constraints: Vec::new(),
            objectives: std::array::from_fn(|_| Vec::new()),
            variables,
        }
    }

    pub(crate) fn variables(&self) -> Arc<Variables> {
        Arc::clone(&self.variables)
    }

    pub(crate) fn constrain(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub(crate) fn constrain_root_to_screen(&mut self, root: &Hitbox) {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            self.constrain(Component_context::name_constraint(
                constraint!(root.get_start_position(direction) == 0),
                match direction {
                    Direction::Horizontal => "root_horizontal_start",
                    Direction::Vertical => "root_vertical_start",
                },
            ));
            self.constrain(Component_context::name_constraint(
                constraint!(
                    root.get_dimension(direction)
                        == match direction {
                            Direction::Horizontal => self.variables.screen.width.clone(),
                            Direction::Vertical => self.variables.screen.height.clone(),
                        }
                ),
                match direction {
                    Direction::Horizontal => "root_width",
                    Direction::Vertical => "root_height",
                },
            ));
        }
    }

    async fn priority_solve(
        &self,
        constraints: &[Constraint],
        direction: ObjectiveDirection,
        objective: Expression,
    ) -> std::result::Result<Solution, ResolutionError> {
        let referenced = constraints
            .iter()
            .flat_map(|constraint| constraint.expression.referenced_variables())
            .chain(objective.referenced_variables())
            .collect::<HashSet<_>>();

        let (problem_variables, solver_variables) = self
            .variables
            .create_solver_variables(&referenced)
            .map_err(|error| ResolutionError::Str(error.to_string()))?;

        let solver_objective = objective
            .into_solver(&solver_variables)
            .map_err(|error| ResolutionError::Str(error.to_string()))?;

        let solver_constraints = constraints
            .iter()
            // Fully static constraints cannot affect optimization. Dropping them also prevents
            // solver infeasibility caused solely by rounding differences between solved values.
            .filter(|constraint| {
                constraint
                    .expression
                    .referenced_variables()
                    .any(|variable| {
                        matches!(
                            solver_variables.get(&variable),
                            Some(Variable_type::Solver(_))
                        )
                    })
            })
            .map(|constraint| constraint.into_solver(&solver_variables))
            .collect::<Result<Vec<_>>>()
            .map_err(|error| ResolutionError::Str(error.to_string()))?;
        let non_static_variables = solver_variables
            .iter()
            .filter_map(|(variable, variable_type)| {
                matches!(variable_type, Variable_type::Solver(_))
                    .then_some(variable.definition_id())
            })
            .collect::<HashSet<_>>()
            .len();

        log_info(
            4,
            format_args!(
                "priority model: {non_static_variables} non-static variables, {} constraints",
                solver_constraints.len(),
            ),
        );

        let model = log_duration(4, "priority model recreation", || async {
            problem_variables
                .optimise(direction, solver_objective)
                .using(microlp)
                .with_all(solver_constraints)
        })
        .await;

        let solved = log_duration(4, "priority solve", || async { model.solve() }).await?;
        let values = solver_variables
            .iter()
            .map(|(variable, variable_type)| {
                let value = match variable_type {
                    Variable_type::Static(value) => *value,
                    Variable_type::Solver(variable) => solved.value(*variable),
                };
                (variable.definition_id(), value)
            })
            .collect::<HashMap<_, _>>();

        log_info(4, format_args!("stats: {:?}", solved.into_inner().stats()));

        Ok(Solution { values })
    }

    async fn find_conflicting_constraints(
        &self,
        constraints: &[Constraint],
    ) -> Result<Vec<Constraint>> {
        let mut conflict = constraints.to_vec();
        let mut index = 0;

        while index < conflict.len() {
            let mut candidate = conflict.clone();
            let _ = candidate.remove(index);

            match self
                .priority_solve(
                    &candidate,
                    ObjectiveDirection::Maximisation,
                    Expression::from(0),
                )
                .await
            {
                Err(ResolutionError::Infeasible) => {
                    conflict = candidate;
                }
                Ok(_) | Err(ResolutionError::Unbounded) => {
                    index += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }

        Ok(conflict)
    }

    fn display_constraint_side(
        &self,
        coefficients: impl IntoIterator<Item = (Variable, f64)>,
        constant: f64,
    ) -> String {
        let mut terms = coefficients
            .into_iter()
            .map(|(variable, coefficient)| {
                let mut name = self.variables.name(&variable);
                if let Variable_type::Static(value) = self.variables.get_type(&variable) {
                    name = format!("{name} [static = {value}]");
                }

                match coefficient {
                    1.0 => name,
                    _ => format!("{coefficient} {name}"),
                }
            })
            .collect::<Vec<_>>();

        if constant != 0.0 {
            terms.push(constant.to_string());
        }

        match terms.is_empty() {
            true => "0".to_string(),
            false => terms.join(" + "),
        }
    }

    fn display_constraint(&self, constraint: &Constraint) -> String {
        let mut left = Vec::new();
        let mut right = Vec::new();

        for (variable, coefficient) in &constraint.expression.coefficients {
            match coefficient {
                coefficient if *coefficient > 0.0 => left.push((variable.clone(), *coefficient)),
                coefficient if *coefficient < 0.0 => right.push((variable.clone(), -*coefficient)),
                _ => {}
            }
        }

        let (left_constant, right_constant) = match constraint.expression.constant {
            constant if constant > 0.0 => (constant, 0.0),
            constant if constant < 0.0 => (0.0, -constant),
            _ => (0.0, 0.0),
        };
        let left = self.display_constraint_side(left, left_constant);
        let right = self.display_constraint_side(right, right_constant);
        let comparison = match constraint.is_equality() {
            true => "=",
            false => "<=",
        };

        format!("{left} {comparison} {right}")
    }

    fn display_constraints(&self, constraints: &[Constraint]) -> String {
        constraints
            .iter()
            .map(|constraint| {
                format!(
                    "{}: {}",
                    constraint.name().unwrap_or("unknown constraint"),
                    self.display_constraint(constraint),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn with_component_tree(
        &self,
        details: String,
        variables: impl IntoIterator<Item = Variable>,
        tree: &Component_tree,
    ) -> String {
        let variables = variables.into_iter().collect::<HashSet<_>>();
        let component_tree = self
            .variables
            .component_tree(&variables, tree)
            .into_iter()
            .map(|(depth, component, source)| match source {
                Some(source) => format!("{}{component}: {source}", "  ".repeat(depth)),
                None => format!("{}{component}", "  ".repeat(depth)),
            })
            .collect::<Vec<_>>()
            .join("\n");

        match component_tree.is_empty() {
            true => details,
            false => format!("{details}\n\nComponent tree:\n{component_tree}"),
        }
    }

    async fn is_unbounded(
        &self,
        constraints: &[Constraint],
        objective: Expression,
    ) -> Result<bool> {
        match self
            .priority_solve(constraints, ObjectiveDirection::Maximisation, objective)
            .await
        {
            Err(ResolutionError::Unbounded) => Ok(true),
            Ok(_) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    async fn describe_underconstrained(
        &self,
        constraints: &[Constraint],
        objective: &Expression,
        component_tree: &Component_tree,
    ) -> Result<String> {
        let mut variables = constraints
            .iter()
            .flat_map(|constraint| constraint.expression.referenced_variables())
            .chain(objective.referenced_variables())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        variables.sort_unstable_by_key(Variable::id);

        let mut underconstrained = Vec::new();
        let mut details = Vec::new();
        for variable in variables {
            let has_no_upper_bound = self
                .is_unbounded(constraints, Expression::from(&variable))
                .await?;
            let has_no_lower_bound = self
                .is_unbounded(constraints, Expression::from(&variable) * -1.0)
                .await?;
            let range = match (has_no_lower_bound, has_no_upper_bound) {
                (true, true) => "has neither a lower nor an upper bound",
                (true, false) => "has no lower bound",
                (false, true) => "has no upper bound",
                (false, false) => continue,
            };

            details.push(format!("{} {range}", self.variables.name(&variable)));
            underconstrained.push(variable);
        }

        let details = match details.is_empty() {
            true => "Layout is underconstrained; the objective is unbounded, but no individual unbounded variable range was identified".to_string(),
            false => format!(
                "Layout is underconstrained; unbounded variable ranges:\n{}",
                details.join("\n")
            ),
        };
        Ok(self.with_component_tree(details, underconstrained, component_tree))
    }

    async fn priority_solve_with_diagnostics(
        &self,
        constraints: &[Constraint],
        objective: Expression,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        match self
            .priority_solve(
                constraints,
                ObjectiveDirection::Maximisation,
                objective.clone(),
            )
            .await
        {
            Ok(solution) => Ok(solution),
            Err(ResolutionError::Infeasible) => {
                let conflict = self.find_conflicting_constraints(constraints).await?;
                let constraints = self.display_constraints(&conflict);
                let conflict = self.with_component_tree(
                    constraints,
                    conflict
                        .iter()
                        .flat_map(|constraint| constraint.expression.referenced_variables()),
                    component_tree,
                );

                log::error!("layout conflicting constraints:\n{conflict}");
                Err(eyre!(
                    "Layout is overconstrained; conflicting constraints:\n{conflict}"
                ))
            }
            Err(ResolutionError::Unbounded) => Err(eyre!(
                "{}",
                self.describe_underconstrained(constraints, &objective, component_tree)
                    .await?
            )),
            Err(error) => Err(error.into()),
        }
    }

    async fn full_solve(
        &mut self,
        mut constraints: Vec<Constraint>,
        screen: Size,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        self.variables.set_type(
            &self.variables.screen.width,
            Variable_type::Static(screen.width),
        );
        self.variables.set_type(
            &self.variables.screen.height,
            Variable_type::Static(screen.height),
        );

        log_duration(0, "layout full solve", || async {
            let mut maybe_solution: Option<Solution> = None;

            for (priority, priority_objectives) in
                self.objectives.clone().into_iter().enumerate().rev()
            {
                log_info(2, format_args!("priority solve {priority}"));

                let priority_objective = priority_objectives
                    .clone()
                    .into_iter()
                    .fold(Expression::default(), |sum, expression| sum + expression);

                let solution = self
                    .priority_solve_with_diagnostics(
                        &constraints,
                        priority_objective,
                        component_tree,
                    )
                    .await?;

                for objective in priority_objectives {
                    let objective_solution = solution.eval(&objective);
                    constraints.push(constraint!(objective == objective_solution));
                }

                maybe_solution = Some(solution);
            }

            maybe_solution.ok_or(eyre!("Expected solution"))
        })
        .await
    }

    pub(crate) async fn solve(
        &mut self,
        screen: Size,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        self.full_solve(self.constraints.clone(), screen, component_tree)
            .await
    }

    pub(crate) async fn solve_minimum(
        &self,
        root: Hitbox,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        self.variables.set_type(
            &self.variables.screen.width,
            Variable_type::Solver(
                good_lp::VariableDefinition::new()
                    .min(0)
                    .name("screen width"),
            ),
        );
        self.variables.set_type(
            &self.variables.screen.height,
            Variable_type::Solver(
                good_lp::VariableDefinition::new()
                    .min(0)
                    .name("screen height"),
            ),
        );
        let root_size =
            root.get_dimension(Direction::Horizontal) + root.get_dimension(Direction::Vertical);
        self.priority_solve_with_diagnostics(&self.constraints, root_size * -1.0, component_tree)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouter::objective::minimize;

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
        problem.constrain_root_to_screen(&root);
        problem.constrain(constraint!(child.start.x.clone() == root.start.x.clone()));
        problem.constrain(constraint!(child.end.x.clone() == root.end.x.clone()));
        minimize(&mut problem, child.get_dimension(Direction::Horizontal), 0)?;

        let component_tree = Vec::new();
        let first = problem
            .solve(Size::new(800.0, 600.0), &component_tree)
            .await?;
        assert_eq!(
            first.eval(&child.get_dimension(Direction::Horizontal)),
            800.0
        );

        let second = problem
            .solve(Size::new(801.0, 600.0), &component_tree)
            .await?;
        assert_eq!(
            second.eval(&child.get_dimension(Direction::Horizontal)),
            801.0
        );

        Ok(())
    }
}
