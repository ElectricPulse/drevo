//! Solver-backed layout primitives.
//!
//! Although it is not currently enforced, components should only access layout variables owned
//! by themselves or their descendants. Keeping constraints inside that ownership boundary avoids
//! hidden coupling to parents, ancestors, and siblings. Positioning components are the exception:
//! they may access the external variables needed to position their child relative to its parent.
//! This boundary will become an enforced requirement when state invalidation and relayout are
//! managed per component.

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
use highs::{Col, HighsModelStatus, HighsStatus, Model, RowProblem, Sense};

use self::{
    constraint::Constraint,
    expression::Expression,
    hitbox::Hitbox,
    variable::{Solver_variable, Variable},
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

#[derive(Clone, Debug)]
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
    pub(crate) fn eval(&self, expression: &Expression) -> f64 {
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

    fn build_row_problem(
        &self,
        constraints: &[Constraint],
        objective: Option<(&Expression, Sense)>,
    ) -> (RowProblem, Vec<Col>) {
        let mut problem = RowProblem::default();
        let metadata = self.variables.all_metadata();
        let mut cols = Vec::with_capacity(metadata.len());

        for info in &metadata {
            let col = if info.is_integer {
                problem.add_integer_column(0.0, info.lower..=info.upper)
            } else {
                problem.add_column(0.0, info.lower..=info.upper)
            };
            cols.push(col);
        }

        if let Some((expr, _)) = objective {
            for (var, coeff) in &expr.coefficients {
                if var.0 < cols.len() {
                    problem.change_column_cost(cols[var.0], *coeff);
                }
            }
        }

        for constraint in constraints {
            let row_factors = constraint
                .expression
                .coefficients
                .iter()
                .filter_map(|(var, coeff)| {
                    if var.0 < cols.len() {
                        Some((cols[var.0], *coeff))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let rhs = -constraint.expression.constant;
            if constraint.equality {
                problem.add_row(rhs..=rhs, row_factors);
            } else {
                problem.add_row(..=rhs, row_factors);
            }
        }

        (problem, cols)
    }

    fn build_model(
        &self,
        constraints: &[Constraint],
        objective: Option<(&Expression, Sense)>,
    ) -> Model {
        let sense = objective.map(|(_, s)| s).unwrap_or(Sense::Maximise);
        let (problem, _) = self.build_row_problem(constraints, objective);
        let mut model = problem.optimise(sense);
        model.make_quiet();
        model.set_option("presolve", "on");
        model.set_option("parallel", "on");
        model.set_option("mip_rel_gap", 0.0);
        model.set_option("mip_abs_gap", 0.0);
        model
    }

    fn solution_from_highs(&self, solved: highs::SolvedModel) -> Result<Solution> {
        let solver_solution = solved.get_solution();
        let variables = self.variables.all();
        let cols = solver_solution.columns();
        let mut values = HashMap::with_capacity(variables.len());
        for var in variables {
            if var.0 < cols.len() {
                let _ = values.insert(var, cols[var.0]);
            }
        }
        Ok(Solution { values })
    }

    fn compute_iis(
        highs_ptr: *mut std::ffi::c_void,
        num_cols: usize,
        num_rows: usize,
    ) -> Vec<usize> {
        let mut iis_num_col: highs_sys::HighsInt = 0;
        let mut iis_num_row: highs_sys::HighsInt = 0;
        let ret = unsafe {
            highs_sys::Highs_getIis(
                highs_ptr,
                &mut iis_num_col,
                &mut iis_num_row,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ret != 0 || (iis_num_col == 0 && iis_num_row == 0) {
            unsafe {
                let _ = highs_sys::Highs_setIntOptionValue(
                    highs_ptr,
                    c"iis_strategy".as_ptr(),
                    highs_sys::kHighsIisStrategyFromLpRowPriority,
                );
                let _ = highs_sys::Highs_getIis(
                    highs_ptr,
                    &mut iis_num_col,
                    &mut iis_num_row,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
        }
        if iis_num_row > 0 {
            let mut col_index = vec![0 as highs_sys::HighsInt; iis_num_col as usize];
            let mut row_index = vec![0 as highs_sys::HighsInt; iis_num_row as usize];
            let mut col_bound = vec![0 as highs_sys::HighsInt; iis_num_col as usize];
            let mut row_bound = vec![0 as highs_sys::HighsInt; iis_num_row as usize];
            let mut col_status = vec![0 as highs_sys::HighsInt; num_cols];
            let mut row_status = vec![0 as highs_sys::HighsInt; num_rows];
            unsafe {
                let _ = highs_sys::Highs_getIis(
                    highs_ptr,
                    &mut iis_num_col,
                    &mut iis_num_row,
                    col_index.as_mut_ptr(),
                    row_index.as_mut_ptr(),
                    col_bound.as_mut_ptr(),
                    row_bound.as_mut_ptr(),
                    col_status.as_mut_ptr(),
                    row_status.as_mut_ptr(),
                );
            }
            row_index.into_iter().map(|idx| idx as usize).collect()
        } else {
            Vec::new()
        }
    }

    async fn describe_infeasible(
        &self,
        conflict_indices: Vec<usize>,
        constraints: &[Constraint],
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        let conflicting_constraints = if !conflict_indices.is_empty() {
            conflict_indices
                .into_iter()
                .filter_map(|idx| constraints.get(idx))
                .collect::<Vec<_>>()
        } else {
            constraints.iter().collect::<Vec<_>>()
        };

        let displayed = self.display_constraints(conflicting_constraints.iter().copied())?;
        let conflict = self.with_component_tree(
            displayed,
            conflicting_constraints
                .iter()
                .flat_map(|c| c.expression.referenced_variables()),
            component_tree,
        );

        log::error!("layout conflicting constraints:\n{conflict}");
        Err(eyre!("Layout is overconstrained; conflicting constraints:\n{conflict}"))
    }

    fn is_unbounded(
        &self,
        constraints: &[Constraint],
        variable: Solver_variable,
        maximize: bool,
    ) -> bool {
        let mut expr = Expression::default();
        let _ = expr.coefficients.insert(variable, 1.0);
        let sense = if maximize { Sense::Maximise } else { Sense::Minimise };
        let model = self.build_model(constraints, Some((&expr, sense)));
        match model.try_solve() {
            Ok(solved) => matches!(
                solved.status(),
                HighsModelStatus::Unbounded | HighsModelStatus::UnboundedOrInfeasible
            ),
            Err(_) => true,
        }
    }

    async fn describe_underconstrained(
        &self,
        constraints: &[Constraint],
        objective: &Expression,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
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
            let has_no_upper_bound = self.is_unbounded(constraints, variable, true);
            let has_no_lower_bound = self.is_unbounded(constraints, variable, false);
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
        let msg = self.with_component_tree(details, underconstrained, component_tree);
        Err(eyre!("{msg}"))
    }

    async fn solve_objective_with_diagnostics(
        &self,
        constraints: &[Constraint],
        objective: Expression,
        component_tree: &Component_tree,
    ) -> Result<Solution> {
        let variable_count = self.variables.len();
        log_info(
            4,
            format_args!(
                "solver model: {variable_count} variables, {} constraints",
                constraints.len(),
            ),
        );

        let (status, result) = {
            let model = self.build_model(constraints, Some((&objective, Sense::Maximise)));
            match model.try_solve() {
                Ok(solved) => {
                    let status = solved.status();
                    match status {
                        HighsModelStatus::Optimal
                        | HighsModelStatus::ObjectiveBound
                        | HighsModelStatus::ObjectiveTarget => {
                            (status, Ok(self.solution_from_highs(solved)?))
                        }
                        HighsModelStatus::Infeasible => {
                            let iis = Self::compute_iis(
                                solved.as_ptr() as *mut std::ffi::c_void,
                                self.variables.len(),
                                constraints.len(),
                            );
                            (status, Err(iis))
                        }
                        _ => (status, Err(Vec::new())),
                    }
                }
                Err(error) => return Err(eyre!("HiGHS error while solving model: {error:?}")),
            }
        };

        match status {
            HighsModelStatus::Optimal
            | HighsModelStatus::ObjectiveBound
            | HighsModelStatus::ObjectiveTarget => match result {
                Ok(solution) => Ok(solution),
                Err(_) => Err(eyre!("expected solution")),
            },
            HighsModelStatus::Infeasible => {
                let conflict_indices = match result {
                    Ok(_) => Vec::new(),
                    Err(indices) => indices,
                };
                self.describe_infeasible(conflict_indices, constraints, component_tree).await
            }
            HighsModelStatus::Unbounded | HighsModelStatus::UnboundedOrInfeasible => {
                self.describe_underconstrained(constraints, &objective, component_tree).await
            }
            status => Err(eyre!("HiGHS returned status {status:?}")),
        }
    }

    fn solve_objectives(
        &self,
        constraints: &[Constraint],
        objectives: &[(usize, Expression)],
    ) -> Result<Solution> {
        let variable_count = self.variables.len();
        let weights = vec![-1.0; objectives.len()];
        let offsets = objectives
            .iter()
            .map(|(_, objective)| objective.constant)
            .collect::<Vec<_>>();
        let variables = self.variables.all();
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
                    eyre!("layout priority {priority} does not fit HiGHS' priority type")
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let objective_count = highs_sys::HighsInt::try_from(objectives.len())
            .map_err(|_| eyre!("too many layout priorities for HiGHS"))?;

        log_info(
            2,
            format_args!(
                "lexicographic model: {variable_count} variables, {} constraints, {} priorities",
                constraints.len(),
                objectives.len(),
            ),
        );

        let model_started = Instant::now();
        let mut model = self.build_model(constraints, None);

        if !objectives.is_empty() {
            model.set_option("blend_multi_objectives", false);

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
                    return Err(eyre!(
                        "HiGHS rejected the lexicographic objectives with status {status:?}"
                    ));
                }
                Err(status) => {
                    return Err(eyre!(
                        "HiGHS returned an invalid status while loading lexicographic objectives: {status:?}"
                    ));
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
            eyre!("HiGHS error while solving model: {error:?}")
        })?;
        log_info(
            2,
            format_args!("lexicographic solve took {:?}", solve_started.elapsed()),
        );

        match solved.status() {
            HighsModelStatus::Optimal
            | HighsModelStatus::ObjectiveBound
            | HighsModelStatus::ObjectiveTarget => self.solution_from_highs(solved),
            HighsModelStatus::Infeasible => {
                let conflict_indices = Self::compute_iis(
                    solved.as_ptr() as *mut std::ffi::c_void,
                    self.variables.len(),
                    constraints.len(),
                );
                let conflicting_constraints = conflict_indices
                    .into_iter()
                    .filter_map(|idx| constraints.get(idx))
                    .collect::<Vec<_>>();
                let displayed = self.display_constraints(conflicting_constraints.iter().copied())?;
                let conflict = self.with_component_tree(
                    displayed,
                    conflicting_constraints
                        .iter()
                        .flat_map(|c| c.expression.referenced_variables()),
                    &Vec::new(),
                );
                log::error!("layout conflicting constraints:\n{conflict}");
                Err(eyre!("Layout is overconstrained; conflicting constraints:\n{conflict}"))
            }
            status => {
                Err(eyre!("HiGHS returned status {status:?}"))
            }
        }
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

    fn display_constraints<'a>(
        &self,
        constraints: impl IntoIterator<Item = &'a Constraint>,
    ) -> Result<String> {
        constraints
            .into_iter()
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
                let msg = error.to_string();
                if msg.contains("Layout is overconstrained") {
                    Err(error)
                } else {
                    self.describe_underconstrained(constraints, &objective, component_tree).await
                }
            }
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
mod tests;
