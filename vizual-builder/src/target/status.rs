use std::sync::Arc;

use color_eyre::eyre::Error;

use crate::task::Status;

#[derive(Clone)]
pub enum Target_status {
    Unsatisfied,
    Running,
    // Error is gonna be read only so an Arc is fine
    Error(Arc<Error>),
    Satisfied(Status),
    Running_dependencies,
}

impl Target_status {
    pub fn get_icon(&self) -> String {
        match self {
            Target_status::Unsatisfied => ".",
            Target_status::Running_dependencies => "..",
            Target_status::Running => "...",
            Target_status::Satisfied(status) => match status {
                Status::Built => "⚒",
                Status::Already_built => "✔",
            },
            Target_status::Error(_err) => "✖",
        }
        .to_owned()
    }

    pub fn satisfied(&self) -> bool {
        matches!(self, Target_status::Satisfied(_))
    }
}
