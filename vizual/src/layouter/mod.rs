pub mod constraints;
pub mod hitbox;

use std::{
    collections::{HashMap, HashSet},
    panic::Location,
};

use self::hitbox::{Dimensions, Hitbox};
use crate::{
    component::context::Component_context,
    geometry::{Direction, Size},
    log::{log_duration, log_info},
};
use color_eyre::eyre::{Result, eyre};
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
    pub(crate) delta: Variable,
}

impl Default for Problem {
    fn default() -> Self {
        let mut variable_builder = ProblemVariables::new();

        let width = variable_builder.add(variable().name("screen width"));
        let height = variable_builder.add(variable().name("screen height"));
        let delta = variable_builder.add(variable().min(0).name("delta"));

        // Screen is created so that it can be used by the components in the layout step
        // it's real dimensions will be constrained later
        // this wastes performance as the screen dimensions are in reality static
        // micro_lp doesn't yet make equality constraints free as it lacks a presolve step
        let screen = Dimensions { width, height };

        let path = Component_context::path(Location::caller());
        let mut goals = std::array::from_fn(|_| None);
        goals[1] = Some(Expression::from(delta) * -1.0);

        Self {
            constraints: Vec::new(),
            goals,
            variable_builder,
            variables: vec![
                Problem_variable {
                    variable: width,
                    path: path.clone(),
                    component_path: String::new(),
                },
                Problem_variable {
                    variable: height,
                    path: path.clone(),
                    component_path: String::new(),
                },
                Problem_variable {
                    variable: delta,
                    path,
                    component_path: String::new(),
                },
            ],
            screen,
            delta,
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
            self.constrain(Component_context::name_constraint(
                constraint!(root.get_start_position(direction) == 0),
                match direction {
                    Direction::Horizontal => "root_horizontal_start",
                    Direction::Vertical => "root_vertical_start",
                },
            ));
            self.constrain(Component_context::name_constraint(
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
            Component_context::name_constraint(
                constraint!(self.screen.get(Direction::Horizontal) == screen.width),
                "screen_width",
            ),
            Component_context::name_constraint(
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
                constraints.push(Component_context::name_constraint(
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
