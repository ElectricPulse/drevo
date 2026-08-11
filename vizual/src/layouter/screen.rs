use good_lp::VariableDefinition;

use super::variable::Variable;

#[derive(Clone)]
pub struct Screen {
    pub width: Variable,
    pub height: Variable,
}

impl Screen {
    pub(crate) fn new() -> Self {
        Self {
            width: Variable::solver(
                VariableDefinition::new().min(0).name("screen width"),
                "screen width",
                "",
                "",
            ),
            height: Variable::solver(
                VariableDefinition::new().min(0).name("screen height"),
                "screen height",
                "",
                "",
            ),
        }
    }
}
