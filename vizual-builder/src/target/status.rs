use std::sync::Arc;

use color_eyre::eyre::Error;
use lucide_icons::Icon;

use super::task::Status;

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
    pub fn get_icon(&self) -> Icon {
        match self {
            Target_status::Unsatisfied => Icon::CircleDashed,
            Target_status::Running_dependencies => Icon::CircleEllipsis,
            Target_status::Running => Icon::Loader,
            Target_status::Satisfied(status) => match status {
                Status::Built => Icon::Hammer,
                Status::Already_built => Icon::CheckCircle,
            },
            Target_status::Error(_err) => Icon::CircleX,
        }
    }

    pub fn satisfied(&self) -> bool {
        matches!(self, Target_status::Satisfied(_))
    }
}
