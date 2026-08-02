use crate::{
    target::{Dependencies, Target},
    task::{self, Status},
};

use async_trait::async_trait;

struct Task {}

impl vizual::widget::Widget_trait for Task {}

// TODO: Remove this workaround
#[async_trait]
impl task::Task_trait for Task {
    type Output = ();
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        return Ok(((), Status::Built));
    }
}

pub fn new(name: impl Into<String>, dependencies: Dependencies) -> Target<()> {
    Target::new(name, Task {}, dependencies)
}
