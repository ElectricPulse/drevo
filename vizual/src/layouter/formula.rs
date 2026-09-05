use std::{collections::HashMap, panic::Location, sync::Arc};

use color_eyre::eyre::{Result, ensure, eyre};

use super::{
    PRIORITY_LEVELS, Solution, constraint::Constraint, expression::Expression, objective::Delta,
    variable::Variable, variables::Variables,
};
use crate::config::COPY_SOLUTION_TO_FORMULA;
use crate::constraint;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WarmStart {
    pub value: f64,
    pub dual: f64,
}

#[derive(Clone, Default)]
struct Record {
    warm_start: Option<WarmStart>,
    used: bool,
}

/// Layout declarations made by one component in the current pass.
///
/// Formula declarations are rebuilt for every layout; this is not a constraint cache. It retains
/// the previous primal and dual values for named variables and constraints, which seed the next
/// rebuilt model. Formula IDs are strings because constraints need names in conflict diagnostics. A
/// numeric-or-string ID enum would add complexity without helping the current callers, so it is
/// deferred.
///
/// Keeping only a solution is less efficient than retaining the old model. Reusing a model would
/// require safely applying incremental `addCol`, `delCol`, `changeCol`, and `addRow` updates.
/// Copying a basis is not usable here because layout can contain integer variables and a basis is
/// invalid after a MIP solve.
#[derive(Clone)]
pub struct Formula {
    pub(crate) variables: Vec<Variable>,
    pub(crate) constraints: Vec<Constraint>,
    pub(crate) objectives: [Vec<Expression>; PRIORITY_LEVELS],
    pub(crate) registry: Arc<Variables>,
    component_path: String,
    variable_records: HashMap<String, Record>,
    constraint_records: HashMap<String, Record>,
    active_variables: Vec<(String, Variable)>,
    active_constraints: Vec<(String, String)>,
}

impl Formula {
    pub fn new(registry: Arc<Variables>) -> Self {
        Self {
            variables: Vec::new(),
            constraints: Vec::new(),
            objectives: std::array::from_fn(|_| Vec::new()),
            registry,
            component_path: String::new(),
            variable_records: HashMap::new(),
            constraint_records: HashMap::new(),
            active_variables: Vec::new(),
            active_constraints: Vec::new(),
        }
    }

    pub(crate) fn begin(&mut self, component_path: String) {
        self.component_path = component_path;
        self.variables.clear();
        self.constraints.clear();
        self.objectives = std::array::from_fn(|_| Vec::new());
        self.active_variables.clear();
        self.active_constraints.clear();
        for record in self.variable_records.values_mut() {
            record.used = false;
            if !COPY_SOLUTION_TO_FORMULA {
                record.warm_start = None;
            }
        }
        for record in self.constraint_records.values_mut() {
            record.used = false;
            if !COPY_SOLUTION_TO_FORMULA {
                record.warm_start = None;
            }
        }
    }

    pub(crate) fn finish(&mut self) {
        self.variable_records.retain(|_, record| record.used);
        self.constraint_records.retain(|_, record| record.used);
    }

    fn record_variable(&mut self, id: String, variable: Variable) -> Result<Variable> {
        self.variables.push(variable);
        self.active_variables.push((id.into(), variable));
        Ok(variable)
    }

    #[track_caller]
    pub fn variable(&mut self, id: impl Into<String>) -> Result<Variable> {
        let id = id.into();
        let variable = self.registry.make(
            id.clone(),
            format!(
                "{}:{}",
                Location::caller().file(),
                Location::caller().line()
            ),
            self.component_path.clone(),
        );
        self.record_variable(id, variable)
    }

    #[track_caller]
    pub fn bounded_variable(
        &mut self,
        id: impl Into<String>,
        lower: f64,
        upper: f64,
        integer: bool,
    ) -> Result<Variable> {
        let id = id.into();
        let variable = self.registry.make_bounded(
            lower,
            upper,
            integer,
            id.clone(),
            format!(
                "{}:{}",
                Location::caller().file(),
                Location::caller().line()
            ),
            self.component_path.clone(),
        );
        self.record_variable(id, variable)
    }

    pub fn binary_variable(&mut self, id: impl Into<String>) -> Result<Variable> {
        self.bounded_variable(id, 0.0, 1.0, true)
    }

    pub(crate) fn register_variable(
        &mut self,
        id: impl Into<String>,
        variable: Variable,
    ) -> Result<()> {
        self.record_variable(id.into(), variable).map(|_| ())
    }

    pub fn constrain(&mut self, id: impl Into<String>, constraint: Constraint) -> Result<()> {
        let id = id.into();

        let name = if self.component_path.is_empty() {
            id.clone()
        } else {
            format!("{}:{id}", self.component_path)
        };

        self.constraints.push(constraint.set_name(name.clone()));
        self.active_constraints.push((id, name));
        Ok(())
    }

    pub fn add_delta(&mut self, id: impl Into<String>, priority: usize) -> Result<Delta> {
        let id = id.into();
        let delta = self.bounded_variable(id.clone(), 0.0, 1.0, false)?;
        self.minimize(format!("{id}.objective"), delta, priority)?;
        Ok(delta)
    }

    pub fn maximize(
        &mut self,
        id: impl Into<String>,
        expression: impl Into<Expression>,
        priority: usize,
    ) -> Result<()> {
        self.minimize(id, expression.into() * -1.0, priority)
    }

    pub fn minimize(
        &mut self,
        _id: impl Into<String>,
        expression: impl Into<Expression>,
        priority: usize,
    ) -> Result<()> {
        if priority >= PRIORITY_LEVELS {
            return Err(eyre!(
                "Layout objective priority {priority} is outside 0..{}",
                PRIORITY_LEVELS - 1
            ));
        }
        self.objectives[priority].push(expression.into());
        Ok(())
    }

    pub fn minimize_delta(
        &mut self,
        id: impl Into<String>,
        expression: impl Into<Expression>,
        target: f64,
        delta: Delta,
        priority: usize,
    ) -> Result<()> {
        ensure!(
            target > 0.0,
            "minimize-difference target must be greater than zero"
        );
        let id = id.into();
        let difference = (Expression::from(target) - expression.into()) / target;
        self.constrain(format!("{id}.difference"), constraint!(difference == delta))?;
        self.minimize(format!("{id}.objective"), delta, priority)
    }

    pub(crate) fn variable_warm_start(&self, variable: Variable) -> Option<WarmStart> {
        self.active_variables
            .iter()
            .find_map(|(id, current)| (*current == variable).then_some(id))
            .and_then(|id| self.variable_records.get(id))
            .and_then(|record| record.warm_start)
    }

    pub(crate) fn constraint_warm_start(&self, name: &str) -> Option<WarmStart> {
        self.active_constraints
            .iter()
            .find_map(|(id, active_name)| (active_name == name).then_some(id))
            .and_then(|id| self.constraint_records.get(id))
            .and_then(|record| record.warm_start)
    }

    /// Returns the numbers of variable and constraint solution records retained by this formula.
    pub(crate) fn store_solution(&mut self, solution: &Solution) -> (usize, usize) {
        if !COPY_SOLUTION_TO_FORMULA {
            return (0, 0);
        }
        let mut variables = 0;
        let mut constraints = 0;
        for (id, variable) in &self.active_variables {
            if let Some(value) = solution.warm_start_for_variable(*variable) {
                self.variable_records
                    .entry(id.clone())
                    .or_default()
                    .warm_start = Some(value);
                variables += 1;
            }
        }
        for (id, name) in &self.active_constraints {
            if let Some(value) = solution.warm_start_for_constraint(name) {
                self.constraint_records
                    .entry(id.clone())
                    .or_default()
                    .warm_start = Some(value);
                constraints += 1;
            }
        }
        (variables, constraints)
    }
}
