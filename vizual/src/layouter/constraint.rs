use super::expression::Expression;

/// A symbolic equality or inequality over layout expressions.
#[derive(Clone, Debug)]
pub struct Constraint {
    pub(crate) expression: Expression,
    pub(crate) equality: bool,
    pub(crate) name: Option<String>,
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
}

#[macro_export]
macro_rules! constraint {
    ([$($left:tt)*] <= $($right:tt)*) => {
        $crate::layouter::constraint::Constraint::less_or_equal($($left)*, $($right)*)
    };
    ([$($left:tt)*] >= $($right:tt)*) => {
        $crate::layouter::constraint::Constraint::greater_or_equal($($left)*, $($right)*)
    };
    ([$($left:tt)*] == $($right:tt)*) => {
        $crate::layouter::constraint::Constraint::equal($($left)*, $($right)*)
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
