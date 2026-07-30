use std::{
    collections::HashMap,
    ops::{Add, AddAssign, Div, Mul, Neg, Sub},
};

use color_eyre::eyre::{Result, eyre};
use good_lp::Expression as Solver_expression;

use super::{screen::SCREEN, variable::Variable, variables::Solver_variables};
use crate::geometry::Size;

/// A symbolic affine expression over stable [`Variable`] indices.
#[derive(Clone, Debug, Default)]
pub struct Expression {
    pub(crate) coefficients: HashMap<Variable, f64>,
    pub(crate) constant: f64,
}

impl Expression {
    pub(crate) fn referenced_variables(&self) -> impl Iterator<Item = Variable> + '_ {
        self.coefficients.keys().copied()
    }

    pub(crate) fn replace_variable(&mut self, old: Variable, new: Variable) {
        if old == new {
            return;
        }
        let Some(coefficient) = self.coefficients.remove(&old) else {
            return;
        };
        let remove = {
            let stored = self.coefficients.entry(new).or_default();
            *stored += coefficient;
            *stored == 0.0
        };
        if remove {
            let _ = self.coefficients.remove(&new);
        }
    }

    pub(crate) fn eval_with(&self, values: &HashMap<Variable, f64>) -> f64 {
        self.constant
            + self
                .coefficients
                .iter()
                .map(|(variable, coefficient)| {
                    coefficient * values.get(variable).copied().unwrap_or_default()
                })
                .sum::<f64>()
    }

    pub(crate) fn into_solver(
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
