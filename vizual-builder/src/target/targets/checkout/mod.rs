mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;

pub fn new(
    name: impl Into<String>,
    repo_path: PathBuf,
    branch: String,
    dependencies: Dependencies,
) -> Target<()> {
    let task = task::Task::new(repo_path, branch.clone());
    let widget = task.widget.clone();
    let mut target = Target::new(name, task, dependencies);
    target.set_widget(widget.into());
    target
}
