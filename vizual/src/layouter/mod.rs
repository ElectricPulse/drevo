pub mod constraints;

use std::{
    collections::{HashMap, HashSet},
    panic::Location,
    sync::Arc,
};

use crate::{
    backend::graphics::Text_resources,
    geometry::Size,
    hitbox::{Dimensions, Direction, Hitbox},
    log::{log_duration, log_info},
    sync::{Mutex, MutexGuard},
};
use color_eyre::eyre::{Result, ensure, eyre};
use futures::future::BoxFuture;
use good_lp::{
    Constraint, Expression, IntoAffineExpression, ProblemVariables, Solution as Good_lp_solution,
    SolverModel as _, Variable, constraint, microlp,
    solvers::{ObjectiveDirection, ResolutionError},
    variable,
};

// This is an async callback for the sake of being generic and allowing for more than x, y, width, height setting on child
pub type Setter = Box<dyn Fn(f64) -> BoxFuture<'static, ()> + Send + Sync>;

const PRIORITY_LEVELS: usize = 6;

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
    values: HashMap<Variable, f64>,
}

impl Solution {
    pub fn value(&self, variable: Variable) -> f64 {
        self.values.get(&variable).copied().unwrap_or_default()
    }

    fn eval(&self, expression: &Expression) -> f64 {
        expression.eval_with(&self.values)
    }
}

struct Problem_variable {
    variable: Variable,
    path: String,
    component_path: String,
}

pub struct Problem {
    constraints: Vec<Constraint>,
    pub goals: [Option<Expression>; PRIORITY_LEVELS],
    variable_builder: ProblemVariables,
    // Used for later underconstraint checking
    variables: Vec<Problem_variable>,
    pub screen: Dimensions,
}

impl Default for Problem {
    fn default() -> Self {
        let mut variable_builder = ProblemVariables::new();

        let width = variable_builder.add(variable().name("screen width"));
        let height = variable_builder.add(variable().name("screen height"));

        let screen = Dimensions {
            // The variable will be constrained later via Constraint
            width,
            height,
        };

        let path = Problem_context::path(Location::caller());

        Self {
            constraints: Vec::new(),
            goals: std::array::from_fn(|_| None),
            variable_builder,
            variables: vec![
                Problem_variable {
                    variable: width,
                    path: path.clone(),
                    component_path: String::new(),
                },
                Problem_variable {
                    variable: height,
                    path,
                    component_path: String::new(),
                },
            ],
            screen,
        }
    }
}

impl Problem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_non_negative_variable(
        &mut self,
        name: String,
        path: String,
        component_path: String,
    ) -> Variable {
        let variable = self.variable_builder.add(variable().min(0).name(name));
        self.variables.push(Problem_variable {
            variable,
            path,
            component_path,
        });
        variable
    }

    pub fn add_binary_variable(
        &mut self,
        name: String,
        path: String,
        component_path: String,
    ) -> Variable {
        let variable = self.variable_builder.add(variable().binary().name(name));
        self.variables.push(Problem_variable {
            variable,
            path,
            component_path,
        });
        variable
    }

    pub(crate) fn constrain(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub(crate) fn maximize(&mut self, expression: Expression, priority: usize) -> Result<()> {
        if priority >= PRIORITY_LEVELS {
            return Err(eyre!(
                "Layout objective priority {priority} is outside the supported range 0..{}",
                PRIORITY_LEVELS - 1
            ));
        }

        match &mut self.goals[priority] {
            Some(goal) => *goal += expression,
            goal => *goal = Some(expression),
        }
        Ok(())
    }

    pub(crate) fn constrain_root_to_screen(&mut self, root: Hitbox) {
        for direction in [Direction::Horizontal, Direction::Vertical] {
            self.constrain(Problem_context::name_constraint(
                constraint!(root.get_start_position(direction) == 0),
                match direction {
                    Direction::Horizontal => "root_horizontal_start",
                    Direction::Vertical => "root_vertical_start",
                },
            ));
            self.constrain(Problem_context::name_constraint(
                constraint!(root.get_dimension(direction) == self.screen.get(direction)),
                match direction {
                    Direction::Horizontal => "root_width",
                    Direction::Vertical => "root_height",
                },
            ));
        }
    }

    fn screen_constraints(&self, screen: Size) -> Vec<Constraint> {
        vec![
            Problem_context::name_constraint(
                constraint!(self.screen.get(Direction::Horizontal) == screen.width),
                "screen_width",
            ),
            Problem_context::name_constraint(
                constraint!(self.screen.get(Direction::Vertical) == screen.height),
                "screen_height",
            ),
        ]
    }

    fn rebuild_variables(&self) -> ProblemVariables {
        let mut variables = ProblemVariables::new();

        for (expected, definition) in self.variable_builder.iter_variables_with_def() {
            let actual = variables.add(definition.clone());
            debug_assert_eq!(actual, expected);
        }

        variables
    }

    async fn solve_model(
        &self,
        constraints: &[Constraint],
        direction: ObjectiveDirection,
        objective: Expression,
    ) -> std::result::Result<Solution, ResolutionError> {
        log_info(
            4,
            format_args!(
                "model: {} variables, {} constraints",
                self.variables.len(),
                constraints.len(),
            ),
        );

        let model = log_duration(4, "model recreation", || async {
            self.rebuild_variables()
                .optimise(direction, objective)
                .using(microlp)
                .with_all(constraints.iter().cloned())
        })
        .await;
        let solved = log_duration(4, "model solve", || async { model.solve() }).await?;
        let solution = self.solution(&solved);

        log_info(4, format_args!("stats: {:?}", solved.into_inner().stats()));

        Ok(solution)
    }

    fn solution(&self, solved: &impl Good_lp_solution) -> Solution {
        Solution {
            values: self
                .variables
                .iter()
                .map(|variable| (variable.variable, solved.value(variable.variable)))
                .collect(),
        }
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
                .solve_model(
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
                let variable = self.variable_builder.display(&variable);

                match coefficient {
                    1.0 => variable.to_string(),
                    _ => format!("{coefficient} {variable}"),
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

        for (variable, coefficient) in constraint.expression().linear_coefficients() {
            match coefficient {
                coefficient if coefficient > 0.0 => left.push((variable, coefficient)),
                coefficient if coefficient < 0.0 => right.push((variable, -coefficient)),
                _ => {}
            }
        }

        let (left_constant, right_constant) = match constraint.expression().constant() {
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

    fn with_component_paths(
        &self,
        details: String,
        variables: impl IntoIterator<Item = Variable>,
    ) -> String {
        let variables = variables.into_iter().collect::<HashSet<_>>();
        let mut components = HashSet::new();
        let paths = self
            .variables
            .iter()
            .filter_map(|variable| {
                match (
                    variable.component_path.is_empty(),
                    variables.contains(&variable.variable),
                ) {
                    (false, true) if components.insert(variable.component_path.clone()) => {
                        Some(format!("{}: {}", variable.component_path, variable.path))
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        match paths.is_empty() {
            true => details,
            false => format!("{details}\n\n{paths}"),
        }
    }

    async fn solve_model_with_diagnostics(
        &self,
        constraints: &[Constraint],
        objective: Expression,
    ) -> Result<Solution> {
        match self
            .solve_model(constraints, ObjectiveDirection::Maximisation, objective)
            .await
        {
            Ok(solution) => Ok(solution),
            Err(ResolutionError::Infeasible) => {
                let conflict = self.find_conflicting_constraints(constraints).await?;
                let constraints = self.display_constraints(&conflict);
                let conflict = self.with_component_paths(
                    constraints,
                    conflict.iter().flat_map(|constraint| {
                        constraint
                            .expression()
                            .linear_coefficients()
                            .map(|(variable, _)| variable)
                    }),
                );

                log::error!("layout conflicting constraints:\n{conflict}");
                Err(eyre!(
                    "Layout is overconstrained; conflicting constraints:\n{conflict}"
                ))
            }
            Err(ResolutionError::Unbounded) => {
                Err(eyre!("Layout is underconstrained; variable ranges",))
            }
            Err(error) => Err(error.into()),
        }
    }

    async fn solve_constraints(&self, mut constraints: Vec<Constraint>) -> Result<Solution> {
        log_duration(0, "layout solve", || async {
            let mut goals =
                self.goals
                    .iter()
                    .enumerate()
                    .rev()
                    .filter_map(|(priority, objective)| {
                        objective.as_ref().map(|objective| (priority, objective))
                    });
            let Some((mut priority, mut objective)) = goals.next() else {
                log_info(2, "feasibility");
                return self
                    .solve_model_with_diagnostics(&constraints, Expression::from(0))
                    .await;
            };

            loop {
                log_info(2, format_args!("priority {priority}"));
                let solved = self
                    .solve_model_with_diagnostics(&constraints, objective.clone())
                    .await?;
                let Some((next_priority, next_objective)) = goals.next() else {
                    return Ok(solved);
                };
                let optimal_value = solved.eval(objective);
                constraints.push(Problem_context::name_constraint(
                    constraint!(objective.clone() == optimal_value),
                    format!("priority_{priority}_optimum"),
                ));

                priority = next_priority;
                objective = next_objective;
            }
        })
        .await
    }

    pub async fn solve(&self, screen: Size) -> Result<Solution> {
        let mut constraints = self.constraints.clone();
        constraints.extend(self.screen_constraints(screen));
        self.solve_constraints(constraints).await
    }

    pub async fn solve_minimum(&self) -> Result<Solution> {
        self.solve_constraints(self.constraints.clone()).await
    }
}

#[derive(Clone)]
pub struct Problem_context {
    pub problem: Arc<Mutex<Problem>>,
    text_resources: Arc<Mutex<Text_resources>>,
    delta: Variable,
    pub component_path: Vec<String>,
}

// TODO: there are so many excesive bridges here
// delegating would be nice
impl Problem_context {
    fn path(location: &'static Location<'static>) -> String {
        format!("{}:{}", location.file(), location.line())
    }

    pub(crate) fn name_constraint(constraint: Constraint, name: impl Into<String>) -> Constraint {
        constraint.set_name(name.into())
    }

    pub async fn new(
        problem: Arc<Mutex<Problem>>,
        text_resources: Arc<Mutex<Text_resources>>,
    ) -> Result<Self> {
        let delta = {
            let mut problem = problem.lock().await?;
            let delta = problem.add_non_negative_variable(
                "delta".to_string(),
                Self::path(Location::caller()),
                String::new(),
            );

            problem.maximize(Expression::from(delta) * -1.0, 1)?;
            delta
        };

        Ok(Self {
            problem,
            text_resources,
            delta,
            component_path: Vec::new(),
        })
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, Problem>> {
        self.problem.lock().await
    }

    #[track_caller]
    pub async fn add_non_negative_variable(&self, name: impl Into<String>) -> Result<Variable> {
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        Ok(self
            .lock()
            .await?
            .add_non_negative_variable(name.into(), path, component_path))
    }

    #[track_caller]
    pub async fn add_binary_variable(&self, name: impl Into<String>) -> Result<Variable> {
        let path = Self::path(Location::caller());
        let component_path = self.component_path.join(".");
        Ok(self
            .lock()
            .await?
            .add_binary_variable(name.into(), path, component_path))
    }

    #[track_caller]
    pub async fn constrain(&self, constraint: Constraint) -> Result<()> {
        let constraint = match constraint.name() {
            Some(_) => constraint,
            None => Self::name_constraint(constraint, Self::path(Location::caller())),
        };
        let mut problem = self.lock().await?;
        problem.constrain(constraint);
        Ok(())
    }

    pub async fn maximize(&self, expression: Expression, priority: usize) -> Result<()> {
        let mut problem = self.lock().await?;
        problem.maximize(expression, priority)
    }

    pub async fn minimize(&self, expression: Expression, priority: usize) -> Result<()> {
        self.maximize(expression * -1.0, priority).await
    }

    #[track_caller]
    /// Minimizes a normalized difference from the requested target through a shared `delta`.
    ///
    /// Sharing `delta` makes gaps, margins, and padding change together. Independently minimizing
    /// absolute values leaves allocations such as `x + y < screen; maximize x + y` free to
    /// resolve to an extreme like `x = screen` and `y = 0`, giving one button no padding while
    /// another receives all the available padding.
    pub async fn minimize_difference(
        &self,
        expression: impl IntoAffineExpression,
        target: f64,
        proportion: f64,
        priority: usize,
    ) -> Result<()> {
        ensure!(
            proportion > 0.0,
            "minimize-difference proportion must be greater than zero"
        );

        let difference = expression.into_expression() - target;
        let inverse_difference = difference.clone() * -1.0;
        let absolute_difference = self
            .add_non_negative_variable("minimize-difference")
            .await?;

        self.constrain(constraint!(absolute_difference >= difference))
            .await?;
        self.constrain(constraint!(absolute_difference >= inverse_difference))
            .await?;
        self.constrain(constraint!(absolute_difference / proportion == self.delta))
            .await?;
        self.minimize(Expression::from(self.delta), priority).await
    }

    pub async fn measure_text(&self, content: &str, font_size: f32) -> Result<Size> {
        Ok(self
            .text_resources
            .lock()
            .await?
            .measure(content, font_size))
    }
}
