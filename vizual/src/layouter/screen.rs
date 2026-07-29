use super::variable::Variable;

/// The variables reserved for the application screen.
///
/// Index `0` is intentionally unused. Screen width permanently owns index `1`, screen height
/// permanently owns index `2`, and dynamically allocated layout variables start at index `3`.
#[derive(Clone, Copy)]
pub struct Screen {
    pub width: Variable,
    pub height: Variable,
}

pub static SCREEN: Screen = Screen {
    width: Variable::new(1),
    height: Variable::new(2),
};
