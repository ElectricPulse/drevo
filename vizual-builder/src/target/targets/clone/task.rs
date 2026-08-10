use crate::{target::targets::terminal, task};

use async_trait::async_trait;
use color_eyre::eyre::WrapErr;
use std::path::PathBuf;
use vizual::widget::{Shared_widget, Widget};

#[derive(Clone, vizual_macros::Widget_trait)]
#[widget_trait(field = widget)]
pub(super) struct Task {
    pub(super) path: PathBuf,
    pub(super) remote_path: String,
    pub(super) widget: Shared_widget<Widget>,
}

impl Task {
    pub fn new(path: PathBuf, remote_path: String) -> Self {
        Self {
            path,
            remote_path,
            widget: task::empty_widget(),
        }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, manager: &mut task::Manager<'_>) -> task::Task_result {
        let git_dir = self.path.join(".git");

        let _ = git_dir.try_exists().wrap_err("")?;

        if git_dir.is_dir() {
            return Ok(((), task::Status::Already_built));
        }

        // Use terminal task to run git clone with progress
        let terminal_task = terminal::task::Task::new_with_widget(
            format!(
                "git clone --progress {} {}",
                self.remote_path,
                self.path.display()
            ),
            self.widget.clone(),
        );

        let result = terminal_task.run(manager).await;
        let _ = result?;

        return Ok(((), task::Status::Built));
    }
}
