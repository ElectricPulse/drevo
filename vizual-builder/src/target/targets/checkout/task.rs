use crate::{target::targets::terminal, task};

use async_trait::async_trait;
use color_eyre::eyre::{WrapErr, bail};
use std::path::PathBuf;
use tokio::process::Command;
use vizual::widget::{Shared_widget, Widget};

#[derive(vizual_macros::Widget_trait)]
#[widget_trait(field = widget)]
pub(super) struct Task {
    pub(super) repo_path: PathBuf,
    pub(super) branch: String,
    pub(super) widget: Shared_widget<Widget>,
}

impl Task {
    pub fn new(repo_path: PathBuf, branch: String) -> Self {
        Self {
            repo_path,
            branch,
            widget: task::empty_widget(),
        }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, manager: &mut task::Manager<'_>) -> task::Task_result {
        // Check current branch silently
        let output = Command::new("git")
            .arg("branch")
            .arg("--show-current")
            .current_dir(&self.repo_path)
            .output()
            .await
            .wrap_err("Failed to get current branch")?;

        if !output.status.success() {
            bail!(
                "Git branch command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if current_branch == self.branch {
            return Ok(((), task::Status::Already_built));
        }

        // Checkout the branch using terminal task to show output
        let terminal_task = terminal::task::Task::new_in_dir_with_widget(
            format!("git checkout {}", self.branch),
            self.repo_path.clone(),
            self.widget.clone(),
        );

        let result = terminal_task.run(manager).await;
        result
    }
}
