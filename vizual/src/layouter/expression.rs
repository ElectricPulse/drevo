use std::{
    collections::{HashMap, HashSet},
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
    sync::{Arc, Mutex},
};

use color_eyre::eyre::{Result, eyre};
use good_lp::Expression as Solver_expression;

use super::{variable::Variable, variables::Solver_variables};

pub type Shared_expression = Arc<Mutex<Expression>>;

/// A symbolic affine expression over [`Variable`] definitions and shared hitbox positions.
///
/// Shared expressions are retained as leaves until the solver model is built. This allows a
/// positioning widget to replace a hitbox coordinate after a parent has already constructed a
/// constraint which references that coordinate.
#[derive(Clone, Debug, Default)]
pub struct Expression {
    pub(crate) coefficients: HashMap<Variable, f64>,
    shared_expressions: Vec<(Shared_expression, f64)>,
    pub(crate) constant: f64,
}

impl Expression {
    pub fn shared(self) -> Shared_expression {
        Arc::new(Mutex::new(self))
    }

    pub(crate) fn resolved(&self) -> Result<Self> {
        let mut resolved = Self::default();
        self.resolve_into(1.0, &mut HashSet::new(), &mut resolved)?;
        Ok(resolved)
    }

    pub(crate) fn single_variable(&self) -> Result<Variable> {
        let resolved = self.resolved()?;
        if resolved.constant != 0.0 || resolved.coefficients.len() != 1 {
            return Err(eyre!("Expected layout position to contain one variable"));
        }

        let (variable, coefficient) = resolved.coefficients.into_iter().next().unwrap();
        if coefficient != 1.0 {
            return Err(eyre!(
                "Expected layout position variable to have coefficient 1"
            ));
        }
        Ok(variable)
    }

    fn resolve_into(
        &self,
        scale: f64,
        visiting: &mut HashSet<usize>,
        resolved: &mut Self,
    ) -> Result<()> {
        resolved.constant += self.constant * scale;
        for (variable, coefficient) in &self.coefficients {
            resolved.add_variable(variable.clone(), coefficient * scale);
        }

        for (expression, coefficient) in &self.shared_expressions {
            let id = Arc::as_ptr(expression) as usize;
            if !visiting.insert(id) {
                return Err(eyre!("Layout position expressions contain a cycle"));
            }

            let expression = expression
                .lock()
                .map_err(|_| eyre!("Layout position expression poisoned"))?
                .clone();
            expression.resolve_into(scale * coefficient, visiting, resolved)?;
            let _ = visiting.remove(&id);
        }

        Ok(())
    }

    fn add_variable(&mut self, variable: Variable, coefficient: f64) {
        let remove = {
            let stored = self.coefficients.entry(variable.clone()).or_default();
            *stored += coefficient;
            *stored == 0.0
        };
        if remove {
            let _ = self.coefficients.remove(&variable);
        }
    }

    fn add_shared_expression(&mut self, expression: Shared_expression, coefficient: f64) {
        let existing = self
            .shared_expressions
            .iter_mut()
            .find(|(stored, _)| Arc::ptr_eq(stored, &expression));

        match existing {
            Some((_, stored)) => *stored += coefficient,
            None => self.shared_expressions.push((expression, coefficient)),
        }
        self.shared_expressions
            .retain(|(_, coefficient)| *coefficient != 0.0);
    }

    pub(crate) fn referenced_variables(&self) -> impl Iterator<Item = Variable> + '_ {
        self.coefficients.keys().cloned()
    }

    pub(crate) fn eval_with(&self, values: &HashMap<usize, f64>) -> f64 {
        self.constant
            + self
                .coefficients
                .iter()
                .map(|(variable, coefficient)| {
                    coefficient
                        * values
                            .get(&variable.definition_id())
                            .copied()
                            .unwrap_or_default()
                })
                .sum::<f64>()
    }

    pub(crate) fn into_solver(
        &self,
        solver_variables: &Solver_variables,
    ) -> Result<Solver_expression> {
        let mut expression = Solver_expression::from(self.constant);

        for (variable, coefficient) in &self.coefficients {
            let solver_variable = solver_variables
                .get(variable)
                .cloned()
                .ok_or_else(|| eyre!("Layout variable {} was not materialized", variable.id()))?;

            expression += solver_variable * *coefficient;
        }

        Ok(expression)
    }
}

impl From<&Variable> for Expression {
    fn from(variable: &Variable) -> Self {
        Self::from(variable.clone())
    }
}

impl From<Variable> for Expression {
    fn from(variable: Variable) -> Self {
        Self {
            coefficients: HashMap::from([(variable, 1.0)]),
            shared_expressions: Vec::new(),
            constant: 0.0,
        }
    }
}

impl From<&Shared_expression> for Expression {
    fn from(expression: &Shared_expression) -> Self {
        Self::from(Arc::clone(expression))
    }
}

impl From<Shared_expression> for Expression {
    fn from(expression: Shared_expression) -> Self {
        Self {
            coefficients: HashMap::new(),
            shared_expressions: vec![(expression, 1.0)],
            constant: 0.0,
        }
    }
}

impl From<f64> for Expression {
    fn from(constant: f64) -> Self {
        Self {
            coefficients: HashMap::new(),
            shared_expressions: Vec::new(),
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
            self.add_variable(variable, coefficient);
        }
        for (expression, coefficient) in rhs.shared_expressions {
            self.add_shared_expression(expression, coefficient);
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
        for (_, coefficient) in &mut self.shared_expressions {
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
