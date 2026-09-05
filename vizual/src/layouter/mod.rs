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
pub mod formula;
pub mod hitbox;
pub mod objective;
pub mod priorities;
pub mod variable;
pub mod variables;

pub use formula::Formula;
pub use priorities::*;

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
    formula::WarmStart,
    hitbox::Hitbox,
    variable::{SolverVariable, Variable},
    variables::Variables,
};
use crate::{
    component::debug::ComponentTree,
    config::{
        BLENDED_GOAL_WEIGHT, COPY_SOLUTION_TO_FORMULA, MODEL_DEBUG, PRIORITIES, Priorities,
    },
    constraint,
    geometry::{Direction, Size},
    log::{log_duration, log_info},
};

// This is an async callback for the sake of being generic and allowing for more than x, y, width,
// height setting on child.
pub trait SetterCallback:
    Fn(f64) -> BoxFuture<'static, ()> + Send + Sync + dyn_clone::DynClone
{
}

impl<Callback> SetterCallback for Callback where
    Callback: Fn(f64) -> BoxFuture<'static, ()> + Send + Sync + Clone + 'static
{
}

dyn_clone::clone_trait_object!(SetterCallback);

pub type Setter = Box<dyn SetterCallback>;

type PriorityObjective = Vec<Expression>;
#[derive(Clone)]
struct Goal {
    priority: usize,
    expression: Expression,
}

#[derive(Clone, Copy)]
enum ObjectiveLabel {
    Priority(usize),
    Weighted,
}

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
    values: HashMap<SolverVariable, f64>,
    warm_variables: HashMap<SolverVariable, WarmStart>,
    warm_constraints: HashMap<String, WarmStart>,
}

impl Solution {
    pub fn value(&self, variable: &Variable) -> f64 {
        self.values
            .get(&variable.variable)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn warm_start_for_variable(&self, variable: Variable) -> Option<WarmStart> {
        self.warm_variables.get(&variable.variable).copied()
    }

    pub(crate) fn warm_start_for_constraint(&self, name: &str) -> Option<WarmStart> {
        self.warm_constraints.get(name).copied()
    }

    #[cfg(test)]
    pub(crate) fn eval(&self, expression: &Expression) -> f64 {
        expression.eval_with(&self.values)
    }
}

pub struct Problem {
    constraints: Vec<Constraint>,
    objectives: [PriorityObjective; PRIORITY_LEVELS],
    pub(crate) variables: Arc<Variables>,
    declared_variables: HashSet<SolverVariable>,
    warm_variables: HashMap<SolverVariable, WarmStart>,
    warm_constraints: HashMap<String, WarmStart>,
}

impl Problem {
    pub fn new(variables: Arc<Variables>) -> Self {
        Self {
            constraints: Vec::new(),
            objectives: std::array::from_fn(|_| Vec::new()),
            variables,
            declared_variables: HashSet::new(),
            warm_variables: HashMap::new(),
            warm_constraints: HashMap::new(),
        }
    }

    pub(crate) fn constrain(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Adds a cached component formula to this one-shot solve problem.
    pub(crate) fn add_formula(&mut self, formula: &Formula) {
        self.declared_variables
            .extend(formula.variables.iter().map(|variable| variable.variable));
        
        if COPY_SOLUTION_TO_FORMULA {
            for variable in &formula.variables {
                if let Some(warm_start) = formula.variable_warm_start(*variable) {
                    let _ = self.warm_variables.insert(variable.variable, warm_start);
                }
            }
            for constraint in &formula.constraints {
                if let Some(name) = constraint.name() {
                    if let Some(warm_start) = formula.constraint_warm_start(name) {
                        let _ = self.warm_constraints.insert(name.to_string(), warm_start);
                    }
                }
            }
        }

        self.constraints.extend(formula.constraints.iter().cloned());

        for (target, source) in self.objectives.iter_mut().zip(&formula.objectives) {
            target.extend(source.iter().cloned());
        }
    }

    pub(crate) fn warm_start_counts(&self) -> (usize, usize) {
        (self.warm_variables.len(), self.warm_constraints.len())
    }

    fn live_variables<'a>(
        &self,
        constraints: &[Constraint],
        objectives: impl IntoIterator<Item = &'a Expression>,
    ) -> Vec<SolverVariable> {
        let mut variables = self.declared_variables.clone();
        variables.extend(
            constraints
                .iter()
                .flat_map(|constraint| constraint.expression.referenced_variables()),
        );
        variables.extend(
            objectives
                .into_iter()
                .flat_map(Expression::referenced_variables),
        );
        let mut variables = variables.into_iter().collect::<Vec<_>>();
        variables.sort_unstable();
        variables
    }

    fn constrain_root_to_screen(
        constraints: &mut Vec<Constraint>,
        objectives: &mut [PriorityObjective; PRIORITY_LEVELS],
        variables: &Arc<Variables>,
        root: &Hitbox,
        screen: Size,
    ) {
        constraints.push(
            constraint!(root.get_start_position(Direction::Horizontal) == 0)
                .set_name("root_horizontal_start".to_string()),
        );
        constraints.push(
            constraint!(root.get_start_position(Direction::Vertical) == 0)
                .set_name("root_vertical_start".to_string()),
        );
        constraints.push(
            constraint!(root.get_dimension(Direction::Horizontal) >= screen.width)
                .set_name("root_horizontal_min".to_string()),
        );
        constraints.push(
            constraint!(root.get_dimension(Direction::Vertical) >= screen.height)
                .set_name("root_vertical_min".to_string()),
        );

        for direction in [Direction::Horizontal, Direction::Vertical] {
            let actual_window_size = match direction {
                Direction::Horizontal => screen.width,
                Direction::Vertical => screen.height,
            };
            let root_dim = root.get_dimension(direction);
            let extra_root_size = variables.make(
                format!("extra_root_size.{direction:?}"),
                "src/layouter/mod.rs",
                "root",
            );
            constraints.push(
                constraint!(extra_root_size >= 0.0)
                    .set_name(format!("extra_root_size.{direction:?}:ge_0")),
            );
            constraints.push(
                constraint!(extra_root_size >= root_dim - actual_window_size)
                    .set_name(format!("extra_root_size.{direction:?}:ge_root_sub_window")),
            );
            objectives[ROOT_DIMENSIONS].push(extra_root_size.into());
        }
    }

    fn build_row_problem(
        &self,
        constraints: &[Constraint],
        objective: Option<(&Expression, Sense)>,
    ) -> (
        RowProblem,
        Vec<SolverVariable>,
        HashMap<SolverVariable, Col>,
    ) {
        let mut problem = RowProblem::default();
        let variables = self.live_variables(constraints, objective.iter().map(|(expr, _)| *expr));
        let mut cols = HashMap::with_capacity(variables.len());

        for variable in &variables {
            let info = self.variables.metadata(*variable);
            let col = if info.is_integer {
                problem.add_integer_column(0.0, info.lower..=info.upper)
            } else {
                problem.add_column(0.0, info.lower..=info.upper)
            };
            let _ = cols.insert(*variable, col);
        }

        if let Some((expr, _)) = objective {
            for (var, coeff) in &expr.coefficients {
                if let Some(col) = cols.get(var) {
                    problem.change_column_cost(*col, *coeff);
                }
            }
        }

        for constraint in constraints {
            let row_factors = constraint
                .expression
                .coefficients
                .iter()
                .filter_map(|(var, coeff)| cols.get(var).map(|col| (*col, *coeff)))
                .collect::<Vec<_>>();
            let rhs = -constraint.expression.constant;
            if constraint.equality {
                problem.add_row(rhs..=rhs, row_factors);
            } else {
                problem.add_row(..=rhs, row_factors);
            }
        }

        (problem, variables, cols)
    }

    fn build_model(
        &self,
        constraints: &[Constraint],
        objective: Option<(&Expression, Sense)>,
    ) -> Model {
        let sense = objective.map(|(_, s)| s).unwrap_or(Sense::Maximise);
        let (problem, _, _) = self.build_row_problem(constraints, objective);
        let mut model = problem.optimise(sense);
        if MODEL_DEBUG {
            model.set_option("output_flag", true);
            model.set_option("log_to_console", true);
        }
        model.set_option("presolve", "on");
        model.set_option("parallel", "on");
        model.set_option("mip_rel_gap", 0.0);
        model.set_option("mip_abs_gap", 0.0);
        model
    }

    fn apply_warm_start(
        &self,
        model: &mut Model,
        variables: &[SolverVariable],
        _constraints: &[Constraint],
    ) {
        if !COPY_SOLUTION_TO_FORMULA {
            return;
        }
        let Some(columns) = variables
            .iter()
            .map(|variable| {
                self.warm_variables
                    .get(variable)
                    .map(|warm_start| warm_start.value)
            })
            .collect::<Option<Vec<_>>>()
        else {
            log_info(
                2,
                format_args!(
                    "layout warm start: 0 columns submitted; {} of {} live variables have prior values",
                    self.warm_variables.len(),
                    variables.len(),
                ),
            );
            return;
        };
        let Some(column_duals) = variables
            .iter()
            .map(|variable| {
                self.warm_variables
                    .get(variable)
                    .map(|warm_start| warm_start.dual)
            })
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        // A formula does not own temporary root-to-screen rows, so it cannot provide a complete
        // row/dual solution for the newly built model. HiGHS accepts an incumbent column vector
        // on its own; passing partial rows or duals produces an inconsistent warm start.
        if let Err(status) = model.try_set_solution(Some(&columns), None, Some(&column_duals), None)
        {
            log::debug!("HiGHS rejected layout warm start: {status:?}");
        } else {
            log_info(
                2,
                format_args!(
                    "layout warm start: submitted {} column values and {} column duals; no row values or row duals",
                    columns.len(),
                    column_duals.len(),
                ),
            );
        }
    }

    fn solution_from_highs(
        &self,
        solved: highs::SolvedModel,
        variables: &[SolverVariable],
        constraints: &[Constraint],
    ) -> Result<Solution> {
        let solver_solution = solved.get_solution();
        let cols = solver_solution.columns();
        let mut values = HashMap::with_capacity(variables.len());
        let col_duals = solver_solution.dual_columns();
        let rows = solver_solution.rows();
        let row_duals = solver_solution.dual_rows();
        let mut warm_variables = HashMap::with_capacity(variables.len());
        for (index, variable) in variables.iter().enumerate() {
            if index < cols.len() {
                let _ = values.insert(*variable, cols[index]);
                let _ = warm_variables.insert(
                    *variable,
                    WarmStart {
                        value: cols[index],
                        dual: col_duals.get(index).copied().unwrap_or_default(),
                    },
                );
            }
        }
        let mut warm_constraints = HashMap::new();
        for (index, constraint) in constraints.iter().enumerate() {
            if let Some(name) = constraint.name() {
                let _ = warm_constraints.insert(
                    name.to_string(),
                    WarmStart {
                        value: rows.get(index).copied().unwrap_or_default(),
                        dual: row_duals.get(index).copied().unwrap_or_default(),
                    },
                );
            }
        }
        Ok(Solution {
            values,
            warm_variables,
            warm_constraints,
        })
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

    fn compute_primal_ray(
        highs_ptr: *const std::ffi::c_void,
        num_cols: usize,
    ) -> Vec<(SolverVariable, f64)> {
        let mut has_primal_ray: highs_sys::HighsInt = 0;
        let mut primal_ray_values = vec![0.0f64; num_cols];
        let ret = unsafe {
            highs_sys::Highs_getPrimalRay(
                highs_ptr,
                &mut has_primal_ray,
                primal_ray_values.as_mut_ptr(),
            )
        };
        if ret == highs_sys::HighsStatuskOk && has_primal_ray == 1 {
            primal_ray_values
                .into_iter()
                .enumerate()
                .filter_map(|(idx, val)| (val.abs() > 1e-6).then_some((SolverVariable(idx), val)))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn display_expression(&self, expression: &Expression) -> String {
        let mut terms = expression
            .coefficients
            .iter()
            .map(|(variable, coefficient)| {
                let name = self.variables.name(*variable);
                if *coefficient == 1.0 {
                    name
                } else if *coefficient == -1.0 {
                    format!("-{name}")
                } else {
                    format!("{coefficient} {name}")
                }
            })
            .collect::<Vec<_>>();

        if expression.constant != 0.0 {
            terms.push(format!("{}", expression.constant));
        }

        if terms.is_empty() {
            "0".to_string()
        } else {
            terms.join(" + ")
        }
    }

    fn is_unbounded(
        &self,
        constraints: &[Constraint],
        variable: SolverVariable,
        maximize: bool,
    ) -> bool {
        let mut expr = Expression::default();
        let _ = expr.coefficients.insert(variable, 1.0);
        let sense = if maximize {
            Sense::Maximise
        } else {
            Sense::Minimise
        };
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
        label: ObjectiveLabel,
        primal_ray: &[(SolverVariable, f64)],
        component_tree: &ComponentTree,
    ) -> Result<Solution> {
        let mut underconstrained = Vec::new();
        let mut details = Vec::new();

        if !primal_ray.is_empty() {
            for (variable, val) in primal_ray {
                let direction = if *val > 0.0 {
                    "grows to +infinity"
                } else {
                    "grows to -infinity"
                };
                details.push(format!("{} {direction}", self.variables.name(*variable)));
                underconstrained.push(*variable);
            }
        } else {
            let objective_vars = objective.referenced_variables().collect::<Vec<_>>();
            let candidate_vars = if !objective_vars.is_empty() {
                objective_vars
            } else {
                constraints
                    .iter()
                    .flat_map(|constraint| constraint.expression.referenced_variables())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect()
            };

            for variable in candidate_vars {
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
        }

        let expr_str = self.display_expression(objective);
        let header = match label {
            ObjectiveLabel::Priority(priority) => format!(
                "Layout is underconstrained; priority {priority} objective ({expr_str}) is unbounded"
            ),
            ObjectiveLabel::Weighted => {
                "Layout is underconstrained; weighted objective is unbounded".to_owned()
            }
        };

        let message = match details.is_empty() {
            true => header,
            false => format!(
                "{header}; unbounded variable ranges:\n{}",
                details.join("\n")
            ),
        };

        let msg = self.with_component_tree(message, underconstrained, component_tree);
        Err(eyre!("{msg}"))
    }

    // This is AI slop
    fn solve_internal(&self, constraints: &[Constraint], objectives: &[Goal]) -> Result<Solution> {
        let variables = self.live_variables(
            constraints,
            objectives.iter().map(|objective| &objective.expression),
        );

        let variable_count = variables.len();
        let binary_count = variables
            .iter()
            .filter(|variable| {
                let info = self.variables.metadata(**variable);
                info.is_integer && info.lower == 0.0 && info.upper == 1.0
            })
            .count();
        let (weights, priority_mode) = match PRIORITIES {
            Priorities::Weighted => (
                objectives
                    .iter()
                    .map(|goal| BLENDED_GOAL_WEIGHT.powi(goal.priority as i32))
                    .collect::<Vec<_>>(),
                "weighted",
            ),
            Priorities::Lexicographic => (vec![1.0; objectives.len()], "lexicographic"),
        };
        let offsets = objectives
            .iter()
            .map(|objective| objective.expression.constant)
            .collect::<Vec<_>>();
        let coefficients = objectives
            .iter()
            .flat_map(|objective| {
                variables.iter().map(|variable| {
                    objective
                        .expression
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
            .map(|objective| {
                highs_sys::HighsInt::try_from(objective.priority).map_err(|_| {
                    eyre!(
                        "layout priority {} does not fit HiGHS' priority type",
                        objective.priority
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let objective_count = highs_sys::HighsInt::try_from(objectives.len())
            .map_err(|_| eyre!("too many layout priorities for HiGHS"))?;

        log_info(
            2,
            format_args!(
                "MILP model: {variable_count} variables ({binary_count} binary), {} constraints, {} priorities ({priority_mode})",
                constraints.len(),
                objectives.len(),
            ),
        );

        let model_started = Instant::now();
        let mut model = self.build_model(constraints, None);

        if !objectives.is_empty() {
            match PRIORITIES {
                Priorities::Weighted => {
                    model.set_option("blend_multi_objectives", true);
                }
                Priorities::Lexicographic => {
                    model.set_option("blend_multi_objectives", false);
                }
            }

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
                        "HiGHS rejected the layout goals with status {status:?}"
                    ));
                }
                Err(status) => {
                    return Err(eyre!(
                        "HiGHS returned an invalid status while loading layout goals: {status:?}"
                    ));
                }
            }
        }
        log_info(
            2,
            format_args!("model recreation took {:?}", model_started.elapsed()),
        );

        let solve_started = Instant::now();
        self.apply_warm_start(&mut model, &variables, constraints);
        let solved = model
            .try_solve()
            .map_err(|error| eyre!("HiGHS error while solving model: {error:?}"))?;
        log_info(2, format_args!("solve {:?}", solve_started.elapsed()));

        match solved.status() {
            HighsModelStatus::Optimal
            | HighsModelStatus::ObjectiveBound
            | HighsModelStatus::ObjectiveTarget => {
                self.solution_from_highs(solved, &variables, constraints)
            }
            HighsModelStatus::Infeasible => {
                let conflict_indices = Self::compute_iis(
                    solved.as_ptr() as *mut std::ffi::c_void,
                    variable_count,
                    constraints.len(),
                );
                let conflicting_constraints = conflict_indices
                    .into_iter()
                    .filter_map(|idx| constraints.get(idx))
                    .collect::<Vec<_>>();
                let displayed =
                    self.display_constraints(conflicting_constraints.iter().copied())?;
                let conflict = self.with_component_tree(
                    displayed,
                    conflicting_constraints
                        .iter()
                        .flat_map(|c| c.expression.referenced_variables()),
                    &Vec::new(),
                );
                log::error!("layout conflicting constraints:\n{conflict}");
                Err(eyre!(
                    "Layout is overconstrained; conflicting constraints:\n{conflict}"
                ))
            }
            status => Err(eyre!("HiGHS returned status {status:?}")),
        }
    }

    fn display_constraint_side(
        &self,
        coefficients: impl IntoIterator<Item = (SolverVariable, f64)>,
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
        variables: impl IntoIterator<Item = SolverVariable>,
        tree: &ComponentTree,
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

    async fn diagnose_objective(
        &self,
        constraints: &[Constraint],
        objective: &Goal,
        label: ObjectiveLabel,
        component_tree: &ComponentTree,
    ) -> Result<Option<f64>> {
        let primal_ray = {
            let model =
                self.build_model(constraints, Some((&objective.expression, Sense::Minimise)));
            let Ok(solved) = model.try_solve() else {
                return Ok(None);
            };

            match solved.status() {
                HighsModelStatus::Optimal
                | HighsModelStatus::ObjectiveBound
                | HighsModelStatus::ObjectiveTarget => {
                    return Ok(Some(solved.objective_value()));
                }
                HighsModelStatus::Unbounded | HighsModelStatus::UnboundedOrInfeasible => {
                    Self::compute_primal_ray(
                        solved.as_ptr() as *const std::ffi::c_void,
                        self.live_variables(constraints, std::iter::once(&objective.expression))
                            .len(),
                    )
                }
                _ => return Ok(None),
            }
        };

        self.describe_underconstrained(
            constraints,
            &objective.expression,
            label,
            &primal_ray,
            component_tree,
        )
        .await
        .map(|_| None)
    }

    async fn diagnose_objectives_failure(
        &self,
        constraints: &[Constraint],
        objectives: &[Goal],
        component_tree: &ComponentTree,
    ) -> Result<Option<Solution>> {
        match PRIORITIES {
            Priorities::Weighted => {
                let expression = objectives.iter().fold(Expression::default(), |sum, goal| {
                    sum + goal.expression.clone() * BLENDED_GOAL_WEIGHT.powi(goal.priority as i32)
                });
                let objective = Goal {
                    priority: 0,
                    expression,
                };
                let _ = self
                    .diagnose_objective(
                        constraints,
                        &objective,
                        ObjectiveLabel::Weighted,
                        component_tree,
                    )
                    .await?;
            }
            Priorities::Lexicographic => {
                let mut constraints = constraints.to_vec();

                for objective in objectives.iter().rev() {
                    let Some(value) = self
                        .diagnose_objective(
                            &constraints,
                            objective,
                            ObjectiveLabel::Priority(objective.priority),
                            component_tree,
                        )
                        .await?
                    else {
                        return Ok(None);
                    };

                    constraints.push(Constraint::less_or_equal(
                        objective.expression.clone(),
                        value + objective.expression.constant,
                    ));
                }
            }
        }

        Ok(None)
    }

    pub(crate) async fn solve(
        &mut self,
        root: Hitbox,
        screen: Size,
        component_tree: &ComponentTree,
    ) -> Result<Solution> {
        let mut constraints = self.constraints.clone();
        let mut objectives_array = self.objectives.clone();
        Self::constrain_root_to_screen(
            &mut constraints,
            &mut objectives_array,
            &self.variables,
            &root,
            screen,
        );

        log_duration(0, "layouting", true, || async {

            let objectives = log_duration(2, "filter objectives", false, async || {
                objectives_array
                    .iter()
                    .enumerate()
                    .filter_map(|(priority, priority_objectives)| {
                        let objective = priority_objectives
                            .iter()
                            .cloned()
                            .fold(Expression::default(), |sum, expression| sum + expression);
                        (!objective.coefficients.is_empty()).then_some(Goal {
                            priority,
                            expression: objective,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .await;

            match self.solve_internal(&constraints, &objectives) {
                Ok(solution) => Ok(solution),
                Err(error) => {
                    let msg = error.to_string();
                    if msg.contains("Layout is overconstrained") {
                        Err(error)
                    } else {
                        self.diagnose_objectives_failure(&constraints, &objectives, component_tree)
                            .await?
                            .ok_or(error)
                    }
                }
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests;
