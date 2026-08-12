use crate::task;
use vizual::widget::{Shared_widget, Widget, widgets::screen::Screen};

use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Clone, vizual_macros::Widget_trait)]
#[widget_trait(field = widget)]
pub(crate) struct Task {
    pub(super) command: String,
    pub(super) working_dir: Option<PathBuf>,
    pub(super) widget: Shared_widget<Widget>,
}

impl Task {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            working_dir: None,
            widget: task::empty_widget(),
        }
    }
    pub fn new_in_dir(command: impl Into<String>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            working_dir: Some(working_dir.into()),
            widget: task::empty_widget(),
        }
    }

    pub fn new_with_widget(command: impl Into<String>, widget: Shared_widget<Widget>) -> Self {
        Self {
            command: command.into(),
            working_dir: None,
            widget,
        }
    }

    pub fn new_in_dir_with_widget(
        command: impl Into<String>,
        working_dir: impl Into<PathBuf>,
        widget: Shared_widget<Widget>,
    ) -> Self {
        Self {
            command: command.into(),
            working_dir: Some(working_dir.into()),
            widget,
        }
    }
}

#[async_trait]
impl task::Task_trait for Task {
    type Output = ();

    async fn run(&self, manager: &mut task::Manager<'_>) -> task::Task_result {
        let mut screen = Screen::new(manager.view.render.clone());
        let handle = match &self.working_dir {
            Some(working_dir) => screen.run_in_dir(self.command.clone(), working_dir),
            None => screen.run(self.command.clone()),
        }?;

        task::set_widget(&self.widget, screen).await?;
        manager.view.refresh();

        handle.wait().await?;

        Ok(((), task::Status::Built))
    }
}
