mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;

pub fn new(
    name: impl Into<String>,
    path: PathBuf,
    remote_path: String,
    dependencies: Dependencies,
) -> Target<()> {
    let task = task::Task::new(path, remote_path);
    let widget = task.widget.clone();
    let mut target = Target::new(name, task, dependencies);
    target.set_widget(widget.into());
    target
}
