pub mod constraint;
pub mod constraints;
pub mod expression;
pub mod hitbox;
pub mod objective;
pub mod variable;
pub mod variables;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use color_eyre::eyre::{Result, eyre};
use futures::future::BoxFuture;
use good_lp::{
    Solution as Good_lp_solution, SolverModel as _, Variable as Solver_variable,
    highs as highs_solver,
    solvers::{ObjectiveDirection, ResolutionError},
};
use highs::{HighsModelStatus, HighsSolutionStatus, HighsStatus};

use self::{
    constraint::Constraint, expression::Expression, hitbox::Hitbox, variable::Variable,
    variables::Variables,
};
use crate::{
    component::debug::Component_tree,
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

const PRIORITY_LEVELS: usize = 2;
type Priority_objective = Vec<Expression>;
// As of this moment the usage of priorities has crystalized like this:
// Minimum screen size is solved separately before these layout objectives.
// 1 is for gaps, spaces, margins, and paddings.
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
    values: HashMap<Solver_variable, f64>,
}

impl Solution {
    pub fn value(&self, variable: &Variable) -> f64 {
        self.values
            .get(&variable.variable)
            .copied()
            .unwrap_or_default()
    }

    #[cfg(test)]
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

    fn constrain_root_to_screen(constraints: &mut Vec<Constraint>, root: &Hitbox, screen: Size) {
        constraints.push(
            constraint!(root.get_start_position(Direction::Horizontal) == 0)
                .set_name("root_horizontal_start".to_string()),
        );
        constraints.push(
            constraint!(root.get_start_position(Direction::Vertical) == 0)
                .set_name("root_vertical_start".to_string()),
        );
        constraints.push(
            constraint!(root.get_end_position(Direction::Horizontal) == screen.width)
                .set_name("root_horizontal_end".to_string()),
        );
        constraints.push(
            constraint!(root.get_end_position(Direction::Vertical) == screen.height)
                .set_name("root_vertical_end".to_string()),
        );
    }

    async fn solve_objective(
        &self,
        constraints: &[Constraint],
        direction: ObjectiveDirection,
        objective: Expression,
    ) -> std::result::Result<Solution, ResolutionError> {
        let problem_variables = self.variables.problem();
        let solver_objective = objective.into_solver();
        let solver_constraints = constraints
            .iter()
            .map(Constraint::into_solver)
            .collect::<Vec<_>>();
        let variable_count = self.variables.len();

        log_info(
            4,
            format_args!(
                "solver model: {variable_count} variables, {} constraints",
                solver_constraints.len(),
            ),
        );

        let model = log_duration(4, "solver model recreation", || async {
            problem_variables
                .optimise(direction, solver_objective)
                .using(highs_solver)
                .with_all(solver_constraints)
                .set_option("presolve", "on")
                .set_option("parallel", "on")
                .set_option("mip_rel_gap", 0.0)
                .set_option("mip_abs_gap", 0.0)
        })
        .await;

        let solved = log_duration(4, "solver solve", || async { model.solve() }).await?;
        let values = self
            .variables
            .all()
            .into_iter()
            .map(|variable| (variable, solved.value(variable)))
            .collect::<HashMap<_, _>>();

        Ok(Solution { values })
    }

    fn solution_from_highs(
        &self,
        solved: highs::SolvedModel,
    ) -> std::result::Result<Solution, ResolutionError> {
        match solved.status() {
            HighsModelStatus::Infeasible | HighsModelStatus::UnboundedOrInfeasible => {
                return Err(ResolutionError::Infeasible);
            }
            HighsModelStatus::Unbounded => return Err(ResolutionError::Unbounded),
            HighsModelStatus::Optimal
            | HighsModelStatus::ObjectiveBound
            | HighsModelStatus::ObjectiveTarget
            | HighsModelStatus::ReachedTimeLimit
            | HighsModelStatus::ReachedSolutionLimit
            | HighsModelStatus::ReachedInterrupt
            | HighsModelStatus::ReachedIterationLimit
            | HighsModelStatus::ReachedMemoryLimit => {}
            status => {
                return Err(ResolutionError::Str(format!(
                    "HiGHS returned model status {status:?}"
                )));
            }
        }

        if solved.primal_solution_status() != HighsSolutionStatus::Feasible {
            return Err(ResolutionError::Other("NoSolutionFound"));
        }

        let solver_solution = solved.get_solution();
        let variables = self.variables.all();
        if variables.len() != solver_solution.columns().len() {
            return Err(ResolutionError::Str(format!(
                "HiGHS returned {} values for {} layout variables",
                solver_solution.columns().len(),
                variables.len(),
            )));
        }

        let values = variables
            .into_iter()
            .zip(solver_solution.columns().iter().copied())
            .collect::<HashMap<_, _>>();
        Ok(Solution { values })
    }

    fn solve_objectives(
        &self,
        constraints: &[Constraint],
        objectives: &[(usize, Expression)],
    ) -> std::result::Result<Solution, ResolutionError> {
        let problem_variables = self.variables.problem();
        let solver_constraints = constraints
            .iter()
            .map(Constraint::into_solver)
            .collect::<Vec<_>>();
        let variables = self.variables.all();
        let variable_count = variables.len();

        let weights = vec![-1.0; objectives.len()];
        let offsets = objectives
            .iter()
            .map(|(_, objective)| objective.constant)
            .collect::<Vec<_>>();
        let coefficients = objectives
            .iter()
            .flat_map(|(_, objective)| {
                variables.iter().map(|variable| {
                    objective
                        .coefficients
                        .get(variable)
                        .copied()
                        .unwrap_or_default()
                })
            })
            .collect::<Vec<_>>();
        let absolute_tolerances = vec![0.0; objectives.len()];
        let relative_tolerances = vec![-1.0; objectives.len()];
        let priorities = objectives
            .iter()
            .map(|(priority, _)| {
                highs_sys::HighsInt::try_from(*priority).map_err(|_| {
                    ResolutionError::Str(format!(
                        "layout priority {priority} does not fit HiGHS' priority type"
                    ))
                })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let objective_count = highs_sys::HighsInt::try_from(objectives.len()).map_err(|_| {
            ResolutionError::Str("too many layout priorities for HiGHS".to_string())
        })?;

        log_info(
            2,
            format_args!(
                "lexicographic model: {variable_count} variables, {} constraints, {} priorities",
                solver_constraints.len(),
                objectives.len(),
            ),
        );

        let model_started = Instant::now();
        let mut model = problem_variables
            .optimise(ObjectiveDirection::Maximisation, 0)
            .using(highs_solver)
            .with_all(solver_constraints)
            .try_into_inner()?;

        model.set_option("presolve", "on");
        model.set_option("parallel", "on");
        model.set_option("mip_rel_gap", 0.0);
        model.set_option("mip_abs_gap", 0.0);

        if !objectives.is_empty() {
            model.set_option("blend_multi_objectives", false);

            // SAFETY: `model` owns a live HiGHS instance. Every objective vector remains alive
            // for this call, and `coefficients` has exactly objective_count * variable_count
            // entries in the objective-major order required by HiGHS.
            let status = unsafe {
                highs_sys::Highs_passLinearObjectives(
                    model.as_ptr(),
                    objective_count,
                    weights.as_ptr(),
                    offsets.as_ptr(),
                    coefficients.as_ptr(),
                    absolute_tolerances.as_ptr(),
                    relative_tolerances.as_ptr(),
                    priorities.as_ptr(),
                )
            };
            match HighsStatus::try_from(status) {
                Ok(HighsStatus::OK) => {}
                Ok(status) => {
                    return Err(ResolutionError::Str(format!(
                        "HiGHS rejected the lexicographic objectives with status {status:?}"
                    )));
                }
                Err(status) => {
                    return Err(ResolutionError::Str(format!(
                        "HiGHS returned an invalid status while loading lexicographic objectives: {status:?}"
                    )));
                }
            }
        }
        log_info(
            4,
            format_args!(
                "lexicographic model recreation took {:?}",
                model_started.elapsed()
            ),
        );

        let solve_started = Instant::now();
        let solved = model.try_solve().map_err(|error| {
            ResolutionError::Str(format!("HiGHS error while solving model: {error:?}"))
        });
        log_info(
            2,
            format_args!("lexicographic solve took {:?}", solve_started.elapsed()),
        );
        self.solution_from_highs(solved?)
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
                .solve_objective(
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
        coefficients: impl IntoIterator<Item = (Solver_variable, f64)>,
        constant: f64,
    ) -> String {
        let mut terms = coefficients
            .into_iter()
            .map(|(variable, coefficient)| {
                let name = self.variables.name(variable);

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

    fn display_constraint(&self, constraint: &Constraint) -> Result<String> {
        let expression = &constraint.expression;
        let mut left = Vec::new();
        let mut right = Vec::new();

        for (variable, coefficient) in &expression.coefficients {
            match coefficient {
                coefficient if *coefficient > 0.0 => left.push((*variable, *coefficient)),
                coefficient if *coefficient < 0.0 => right.push((*variable, -*coefficient)),
                _ => {}
            }
        }

        let (left_constant, right_constant) = match expression.constant {
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

        Ok(format!("{left} {comparison} {right}"))
    }

    fn display_constraints(&self, constraints: &[Constraint]) -> Result<String> {
        constraints
            .iter()
            .map(|constraint| {
                Ok(format!(
                    "{}: {}",
                    constraint.name().unwrap_or("unknown constraint"),
                    self.display_constraint(constraint)?,
                ))
            })
            .collect::<Result<Vec<_>>>()
            .map(|constraints| constraints.join("\n"))
    }

    fn with_component_tree(
        &self,
        details: String,
        variables: impl IntoIterator<Item = Solver_variable>,
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
            .solve_objective(constraints, ObjectiveDirection::Maximisation, objective)
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
        variables.sort_unstable_by_key(|variable| self.variables.name(*variable));

        let mut underconstrained = Vec::new();
        let mut details = Vec::new();
        for variable in variables {
            let has_no_upper_bound = self
                .is_unbounded(constraints, Expression::from(variable))
                .await?;
            let has_no_lower_bound = self
                .is_unbounded(constraints, Expression::from(variable) * -1.0)
                .await?;
            let range = match (has_no_lower_bound, has_no_upper_bound) {
                (true, true) => "has neither a lower nor an upper bound",
                (true, false) => "has no lower bound",
                (false, true) => "has no upper bound",
                (false, false) => continue,
            };

            details.push(format!("{} {range}", self.variables.name(variable)));
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

    async fn solve_objective_with_diagnostics(
        &self,
        constraints: &[Constraint],
        objective: Expression,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        match self
            .solve_objective(
                constraints,
                ObjectiveDirection::Maximisation,
                objective.clone(),
            )
            .await
        {
            Ok(solution) => Ok(solution),
            Err(error) => {
                self.describe_resolution_error(error, constraints, &objective, component_tree)
                    .await
            }
        }
    }

    async fn solve_objectives_with_diagnostics(
        &self,
        constraints: &[Constraint],
        objectives: &[(usize, Expression)],
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        match self.solve_objectives(constraints, objectives) {
            Ok(solution) => Ok(solution),
            Err(error) => {
                let objective = objectives
                    .iter()
                    .fold(Expression::default(), |sum, (_, objective)| {
                        sum + objective.clone()
                    });
                self.describe_resolution_error(error, constraints, &objective, component_tree)
                    .await
            }
        }
    }

    async fn describe_resolution_error(
        &self,
        error: ResolutionError,
        constraints: &[Constraint],
        objective: &Expression,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        match error {
            ResolutionError::Infeasible => {
                let conflict = self.find_conflicting_constraints(constraints).await?;
                let displayed_constraints = self.display_constraints(&conflict)?;
                let conflict = self.with_component_tree(
                    displayed_constraints,
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
            ResolutionError::Unbounded => Err(eyre!(
                "{}",
                self.describe_underconstrained(constraints, objective, component_tree)
                    .await?
            )),
            error => Err(error.into()),
        }
    }

    async fn full_solve(
        &mut self,
        mut constraints: Vec<Constraint>,
        root: Hitbox,
        screen: Size,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        Self::constrain_root_to_screen(&mut constraints, &root, screen);

        log_duration(0, "layout full solve", || async {
            let objectives = self
                .objectives
                .iter()
                .enumerate()
                .filter_map(|(priority, priority_objectives)| {
                    let objective = priority_objectives
                        .iter()
                        .cloned()
                        .fold(Expression::default(), |sum, expression| sum + expression);
                    (!objective.coefficients.is_empty()).then_some((priority, objective))
                })
                .collect::<Vec<_>>();

            self.solve_objectives_with_diagnostics(&constraints, &objectives, component_tree)
                .await
        })
        .await
    }

    pub(crate) async fn solve(
        &mut self,
        root: Hitbox,
        screen: Size,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        self.full_solve(self.constraints.clone(), root, screen, component_tree)
            .await
    }

    pub(crate) async fn solve_minimum(
        &self,
        root: Hitbox,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        let mut constraints = self.constraints.clone();
        constraints.push(
            constraint!(root.get_start_position(Direction::Horizontal) == 0)
                .set_name("minimum_root_horizontal_start".to_string()),
        );
        constraints.push(
            constraint!(root.get_start_position(Direction::Vertical) == 0)
                .set_name("minimum_root_vertical_start".to_string()),
        );
        let root_size =
            root.get_dimension(Direction::Horizontal) + root.get_dimension(Direction::Vertical);
        self.solve_objective_with_diagnostics(&constraints, root_size * -1.0, component_tree)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouter::objective::minimize;
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
        minimize(&mut problem, child.get_dimension(Direction::Horizontal), 0)?;

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
}
