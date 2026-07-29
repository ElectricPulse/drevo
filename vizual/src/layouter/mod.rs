pub mod constraints;
pub mod hitbox;
pub mod screen;
pub mod variable;

use std::{
    collections::{HashMap, HashSet},
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
    panic::Location,
    sync::Arc,
};

use color_eyre::eyre::{Result, eyre};
use futures::future::BoxFuture;
use good_lp::{
    Expression as Solver_expression, Solution as Good_lp_solution, SolverModel as _,
    constraint as solver_constraint, microlp,
    solvers::{ObjectiveDirection, ResolutionError},
};

use self::{
    hitbox::Hitbox,
    screen::SCREEN,
    variable::{Solver_variables, Variable, Variables},
};
use crate::{
    component::context::Component_context,
    geometry::{Direction, Size},
    log::{log_duration, log_info},
};

// This is an async callback for the sake of being generic and allowing for more than x, y, width,
// height setting on child.
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

/// A symbolic affine expression over stable [`Variable`] indices.
#[derive(Clone, Debug, Default)]
pub struct Expression {
    coefficients: HashMap<Variable, f64>,
    constant: f64,
}

impl Expression {
    fn referenced_variables(&self) -> impl Iterator<Item = Variable> + '_ {
        self.coefficients.keys().copied()
    }

    fn eval_with(&self, values: &HashMap<Variable, f64>) -> f64 {
        self.constant
            + self
                .coefficients
                .iter()
                .map(|(variable, coefficient)| {
                    coefficient * values.get(variable).copied().unwrap_or_default()
                })
                .sum::<f64>()
    }

    fn into_solver(
        &self,
        solver_variables: &Solver_variables,
        screen: Option<Size>,
    ) -> Result<Solver_expression> {
        let mut expression = Solver_expression::from(self.constant);

        for (variable, coefficient) in &self.coefficients {
            let constant = match (*variable, screen) {
                (variable, Some(screen)) if variable == SCREEN.width => Some(screen.width),
                (variable, Some(screen)) if variable == SCREEN.height => Some(screen.height),
                _ => None,
            };

            match constant {
                Some(constant) => expression += *coefficient * constant,
                None => {
                    let solver_variable = solver_variables
                        .get(&variable.index())
                        .copied()
                        .ok_or_else(|| {
                            eyre!(
                                "Layout variable {} has no solve-time variable",
                                variable.index()
                            )
                        })?;
                    expression += *coefficient * solver_variable;
                }
            }
        }

        Ok(expression)
    }
}

impl From<Variable> for Expression {
    fn from(variable: Variable) -> Self {
        Self {
            coefficients: HashMap::from([(variable, 1.0)]),
            constant: 0.0,
        }
    }
}

impl From<f64> for Expression {
    fn from(constant: f64) -> Self {
        Self {
            coefficients: HashMap::new(),
            constant,
        }
    }
}

impl From<f32> for Expression {
    fn from(constant: f32) -> Self {
        Self::from(f64::from(constant))
    }
}

impl From<i32> for Expression {
    fn from(constant: i32) -> Self {
        Self::from(f64::from(constant))
    }
}

impl<T: Into<Expression>> Add<T> for Expression {
    type Output = Expression;

    fn add(mut self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        self.constant += rhs.constant;
        for (variable, coefficient) in rhs.coefficients {
            let remove = {
                let stored = self.coefficients.entry(variable).or_default();
                *stored += coefficient;
                *stored == 0.0
            };
            if remove {
                let _ = self.coefficients.remove(&variable);
            }
        }
        self
    }
}

impl<T: Into<Expression>> Sub<T> for Expression {
    type Output = Expression;

    fn sub(self, rhs: T) -> Self::Output {
        self + -rhs.into()
    }
}

impl Mul<f64> for Expression {
    type Output = Expression;

    fn mul(mut self, rhs: f64) -> Self::Output {
        self.constant *= rhs;
        for coefficient in self.coefficients.values_mut() {
            *coefficient *= rhs;
        }
        self
    }
}

impl Div<f64> for Expression {
    type Output = Expression;

    fn div(self, rhs: f64) -> Self::Output {
        self * (1.0 / rhs)
    }
}

impl Neg for Expression {
    type Output = Expression;

    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

impl AddAssign<Expression> for Expression {
    fn add_assign(&mut self, rhs: Expression) {
        *self = self.clone() + rhs;
    }
}

impl<T: Into<Expression>> Add<T> for Variable {
    type Output = Expression;

    fn add(self, rhs: T) -> Self::Output {
        Expression::from(self) + rhs
    }
}

impl<T: Into<Expression>> Sub<T> for Variable {
    type Output = Expression;

    fn sub(self, rhs: T) -> Self::Output {
        Expression::from(self) - rhs
    }
}

impl Mul<f64> for Variable {
    type Output = Expression;

    fn mul(self, rhs: f64) -> Self::Output {
        Expression::from(self) * rhs
    }
}

impl Div<f64> for Variable {
    type Output = Expression;

    fn div(self, rhs: f64) -> Self::Output {
        Expression::from(self) / rhs
    }
}

macro_rules! implement_number_expression_operations {
    ($number:ty) => {
        impl Add<Variable> for $number {
            type Output = Expression;

            fn add(self, rhs: Variable) -> Self::Output {
                Expression::from(self) + rhs
            }
        }

        impl Sub<Variable> for $number {
            type Output = Expression;

            fn sub(self, rhs: Variable) -> Self::Output {
                Expression::from(self) - rhs
            }
        }

        impl Mul<Variable> for $number {
            type Output = Expression;

            fn mul(self, rhs: Variable) -> Self::Output {
                Expression::from(rhs) * f64::from(self)
            }
        }

        impl Add<Expression> for $number {
            type Output = Expression;

            fn add(self, rhs: Expression) -> Self::Output {
                Expression::from(self) + rhs
            }
        }

        impl Sub<Expression> for $number {
            type Output = Expression;

            fn sub(self, rhs: Expression) -> Self::Output {
                Expression::from(self) - rhs
            }
        }

        impl Mul<Expression> for $number {
            type Output = Expression;

            fn mul(self, rhs: Expression) -> Self::Output {
                rhs * f64::from(self)
            }
        }
    };
}

implement_number_expression_operations!(f64);
implement_number_expression_operations!(f32);
implement_number_expression_operations!(i32);

/// A symbolic equality or inequality over stable layout variables.
#[derive(Clone, Debug)]
pub struct Constraint {
    expression: Expression,
    equality: bool,
    name: Option<String>,
}

impl Constraint {
    pub fn equal(left: impl Into<Expression>, right: impl Into<Expression>) -> Self {
        Self {
            expression: left.into() - right,
            equality: true,
            name: None,
        }
    }

    pub fn less_or_equal(left: impl Into<Expression>, right: impl Into<Expression>) -> Self {
        Self {
            expression: left.into() - right,
            equality: false,
            name: None,
        }
    }

    pub fn greater_or_equal(left: impl Into<Expression>, right: impl Into<Expression>) -> Self {
        Self::less_or_equal(right, left)
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn expression(&self) -> &Expression {
        &self.expression
    }

    pub fn is_equality(&self) -> bool {
        self.equality
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn into_solver(
        &self,
        solver_variables: &Solver_variables,
        screen: Option<Size>,
    ) -> Result<good_lp::Constraint> {
        let expression = self.expression.into_solver(solver_variables, screen)?;
        let constraint = match self.equality {
            true => solver_constraint::eq(expression, 0),
            false => solver_constraint::leq(expression, 0),
        };

        Ok(match &self.name {
            Some(name) => constraint.set_name(name.clone()),
            None => constraint,
        })
    }
}

#[macro_export]
macro_rules! constraint {
    ([$($left:tt)*] <= $($right:tt)*) => {
        $crate::layouter::Constraint::less_or_equal($($left)*, $($right)*)
    };
    ([$($left:tt)*] >= $($right:tt)*) => {
        $crate::layouter::Constraint::greater_or_equal($($left)*, $($right)*)
    };
    ([$($left:tt)*] == $($right:tt)*) => {
        $crate::layouter::Constraint::equal($($left)*, $($right)*)
    };
    ([$($left:tt)*]) => {
        $($left)*
    };
    ([$($left:tt)*] $next:tt $($right:tt)*) => {
        $crate::constraint!([$($left)* $next] $($right)*)
    };
    ($($all:tt)*) => {
        $crate::constraint!([] $($all)*)
    };
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

pub struct Problem {
    constraints: Vec<Constraint>,
    goals: [Option<Expression>; PRIORITY_LEVELS],
    variables: Arc<Variables>,
    owned_variables: Vec<Variable>,
    pub(crate) delta: Variable,
}

impl Problem {
    pub fn new(variables: Arc<Variables>) -> Self {
        let path = Component_context::path(Location::caller());
        let delta = variables.add_non_negative("delta", path, String::new());
        let mut goals = std::array::from_fn(|_| None);
        goals[1] = Some(Expression::from(delta) * -1.0);

        Self {
            constraints: Vec::new(),
            goals,
            variables,
            owned_variables: vec![delta],
            delta,
        }
    }

    pub(crate) fn variables(&self) -> Arc<Variables> {
        Arc::clone(&self.variables)
    }

    pub fn add_non_negative_variable(
        &mut self,
        name: String,
        path: String,
        component_path: String,
    ) -> Variable {
        let variable = self.variables.add_non_negative(name, path, component_path);
        self.owned_variables.push(variable);
        variable
    }

    pub fn add_binary_variable(
        &mut self,
        name: String,
        path: String,
        component_path: String,
    ) -> Variable {
        let variable = self.variables.add_binary(name, path, component_path);
        self.owned_variables.push(variable);
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
                constraint!(
                    root.get_dimension(direction)
                        == match direction {
                            Direction::Horizontal => SCREEN.width,
                            Direction::Vertical => SCREEN.height,
                        }
                ),
                match direction {
                    Direction::Horizontal => "root_width",
                    Direction::Vertical => "root_height",
                },
            ));
        }
    }

    /// Performs one priority solve with a fresh set of `good_lp` variables and a fresh model.
    async fn priority_solve(
        &self,
        constraints: &[Constraint],
        direction: ObjectiveDirection,
        objective: Expression,
        screen: Option<Size>,
    ) -> std::result::Result<Solution, ResolutionError> {
        let referenced = constraints
            .iter()
            .flat_map(|constraint| constraint.expression.referenced_variables())
            .chain(objective.referenced_variables())
            .collect::<HashSet<_>>();
        let (problem_variables, solver_variables) = self
            .variables
            .create_solver_variables(&referenced, screen)
            .map_err(|error| ResolutionError::Str(error.to_string()))?;
        let solver_objective = objective
            .into_solver(&solver_variables, screen)
            .map_err(|error| ResolutionError::Str(error.to_string()))?;
        let solver_constraints = constraints
            .iter()
            .map(|constraint| constraint.into_solver(&solver_variables, screen))
            .collect::<Result<Vec<_>>>()
            .map_err(|error| ResolutionError::Str(error.to_string()))?;

        log_info(
            4,
            format_args!(
                "priority model: {} referenced variables, {} constraints",
                solver_variables.len(),
                constraints.len(),
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
        let mut values = solver_variables
            .iter()
            .map(|(index, variable)| (Variable::new(*index), solved.value(*variable)))
            .collect::<HashMap<_, _>>();

        if let Some(screen) = screen {
            let _ = values.insert(SCREEN.width, screen.width);
            let _ = values.insert(SCREEN.height, screen.height);
        }

        log_info(4, format_args!("stats: {:?}", solved.into_inner().stats()));

        Ok(Solution { values })
    }

    async fn find_conflicting_constraints(
        &self,
        constraints: &[Constraint],
        screen: Option<Size>,
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
                    screen,
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
                let variable = self.variables.name(variable);

                match coefficient {
                    1.0 => variable,
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

        for (variable, coefficient) in &constraint.expression.coefficients {
            match coefficient {
                coefficient if *coefficient > 0.0 => left.push((*variable, *coefficient)),
                coefficient if *coefficient < 0.0 => right.push((*variable, -*coefficient)),
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

    fn with_component_paths(
        &self,
        details: String,
        variables: impl IntoIterator<Item = Variable>,
    ) -> String {
        let variables = variables.into_iter().collect::<HashSet<_>>();
        let mut components = HashSet::new();
        let paths = self
            .variables
            .component_paths(&variables)
            .filter_map(
                |(component_path, path)| match components.insert(component_path.clone()) {
                    true => Some(format!("{component_path}: {path}")),
                    false => None,
                },
            )
            .collect::<Vec<_>>()
            .join("\n");

        match paths.is_empty() {
            true => details,
            false => format!("{details}\n\n{paths}"),
        }
    }

    async fn priority_solve_with_diagnostics(
        &self,
        constraints: &[Constraint],
        objective: Expression,
        screen: Option<Size>,
    ) -> Result<Solution> {
        match self
            .priority_solve(
                constraints,
                ObjectiveDirection::Maximisation,
                objective,
                screen,
            )
            .await
        {
            Ok(solution) => Ok(solution),
            Err(ResolutionError::Infeasible) => {
                let conflict = self
                    .find_conflicting_constraints(constraints, screen)
                    .await?;
                let constraints = self.display_constraints(&conflict);
                let conflict = self.with_component_paths(
                    constraints,
                    conflict
                        .iter()
                        .flat_map(|constraint| constraint.expression.referenced_variables()),
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

    /// Performs the complete sequence of populated priority solves.
    async fn full_solve(
        &self,
        mut constraints: Vec<Constraint>,
        screen: Option<Size>,
    ) -> Result<Solution> {
        log_duration(0, "layout full solve", || async {
            let mut goals =
                self.goals
                    .iter()
                    .enumerate()
                    .rev()
                    .filter_map(|(priority, objective)| {
                        objective.as_ref().map(|objective| (priority, objective))
                    });
            let Some((mut priority, mut objective)) = goals.next() else {
                log_info(2, "feasibility priority solve");
                return self
                    .priority_solve_with_diagnostics(&constraints, Expression::from(0), screen)
                    .await;
            };

            loop {
                log_info(2, format_args!("priority solve {priority}"));
                let solved = self
                    .priority_solve_with_diagnostics(&constraints, objective.clone(), screen)
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
        self.full_solve(self.constraints.clone(), Some(screen))
            .await
    }

    pub async fn solve_minimum(&self) -> Result<Solution> {
        self.full_solve(self.constraints.clone(), None).await
    }
}

impl Drop for Problem {
    fn drop(&mut self) {
        for variable in self.owned_variables.drain(..) {
            self.variables.remove(variable);
        }
    }
}
