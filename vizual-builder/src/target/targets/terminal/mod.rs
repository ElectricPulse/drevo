pub(super) mod task;

use crate::target::{Dependencies, Target};
use std::path::PathBuf;
use vizual::widget::{Shared_widget, Widget};

pub fn new(
    name: impl Into<String>,
    command: impl Into<String>,
    dependencies: Dependencies,
) -> Target<()> {
    build(name, task::Task::new(command), dependencies)
}

pub fn new_in_dir(
    name: impl Into<String>,
    command: impl Into<String>,
    working_dir: impl Into<PathBuf>,
    dependencies: Dependencies,
) -> Target<()> {
    build(
        name,
        task::Task::new_in_dir(command, working_dir),
        dependencies,
    )
}

pub fn new_with_widget(
    name: impl Into<String>,
    command: impl Into<String>,
    working_dir: Option<PathBuf>,
    widget: Shared_widget<Widget>,
    dependencies: Dependencies,
) -> Target<()> {
    let task = if let Some(working_dir) = working_dir {
        task::Task::new_in_dir_with_widget(command, working_dir, widget)
    } else {
        task::Task::new_with_widget(command, widget)
    };
    build(name, task, dependencies)
}

fn build(name: impl Into<String>, task: task::Task, dependencies: Dependencies) -> Target<()> {
    let widget = task.widget.clone();
    let mut target = match task.working_dir.clone() {
        Some(path) => Target::new_with_path(name, path, task, dependencies),
        None => Target::new(name, task, dependencies),
    };
    target.set_widget(widget.into());
    target
}
