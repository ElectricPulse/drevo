use crate::task;

use async_trait::async_trait;
use color_eyre::eyre::WrapErr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use vizual::{
    Render,
    component::{Children, context::Component_context},
    graphics::text::Text_context,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::Focus_provider,
};

#[derive(Clone)]
pub(super) struct Copy_file_task {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
}

#[async_trait]
impl vizual::widget::Widget_trait for Copy_file_task {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> color_eyre::Result<Children> {
        Ok(vec![])
    }
}

#[async_trait]
impl task::Task_trait for Copy_file_task {
    type Output = ();
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        let _ = tokio::fs::copy(&self.source, &self.destination)
            .await
            .wrap_err(format!(
                "Failed to copy file from {:?} to {:?}",
                self.source, self.destination
            ))?;

        Ok(((), task::Status::Built))
    }
}

#[derive(Clone)]
pub(super) struct Copy_dir_task {
    pub(super) source: PathBuf,
    pub(super) destination: PathBuf,
}

#[async_trait]
impl vizual::widget::Widget_trait for Copy_dir_task {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> color_eyre::Result<Children> {
        Ok(vec![])
    }
}

#[async_trait]
impl task::Task_trait for Copy_dir_task {
    type Output = ();
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        // Check if dest already exists
        if self.destination.exists()
            && let Some(content) = self.destination.read_dir()?.next()
        {
            let _ = content?;
            return Ok(((), task::Status::Already_built));
        }

        copy_dir_all(&self.source, &self.destination).wrap_err(format!(
            "Failed to copy directory from {:?} to {:?}",
            self.source, self.destination
        ))?;

        Ok(((), task::Status::Built))
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            let _ = fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[derive(Clone)]
pub(super) struct Create_dir_task {
    pub(super) path: PathBuf,
}

#[async_trait]
impl vizual::widget::Widget_trait for Create_dir_task {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> color_eyre::Result<Children> {
        Ok(vec![])
    }
}

#[async_trait]
impl task::Task_trait for Create_dir_task {
    type Output = ();
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        if self.path.exists() {
            return Ok(((), task::Status::Already_built));
        }

        tokio::fs::create_dir_all(&self.path)
            .await
            .wrap_err(format!("Failed to create directory: {:?}", self.path))?;

        Ok(((), task::Status::Built))
    }
}

#[derive(Clone)]
pub(super) struct Write_file_task {
    pub(super) path: PathBuf,
    pub(super) content: String,
}

#[async_trait]
impl vizual::widget::Widget_trait for Write_file_task {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> color_eyre::Result<Children> {
        Ok(vec![])
    }
}

#[async_trait]
impl task::Task_trait for Write_file_task {
    type Output = ();
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result {
        // Check if file exists with same content
        if self.path.exists()
            && let Ok(existing_content) = tokio::fs::read_to_string(&self.path).await
            && existing_content == self.content
        {
            return Ok(((), task::Status::Already_built));
        }

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .wrap_err(format!("Failed to create parent directory: {:?}", parent))?;
        }

        tokio::fs::write(&self.path, &self.content)
            .await
            .wrap_err(format!("Failed to write file: {:?}", self.path))?;

        Ok(((), task::Status::Built))
    }
}

#[derive(Clone)]
pub(super) struct Create_directory_task {
    pub(super) path: PathBuf,
    pub(super) subdirs: Vec<String>,
}

#[async_trait]
impl vizual::widget::Widget_trait for Create_directory_task {
    async fn layout(
        &mut self,
        _render: Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> color_eyre::Result<Children> {
        Ok(vec![])
    }
}

#[async_trait]
impl task::Task_trait for Create_directory_task {
    type Output = PathBuf;
    async fn run(&self, _manager: &mut task::Manager<'_>) -> task::Task_result<PathBuf> {
        if self.path.exists() {
            return Ok((self.path.clone(), task::Status::Already_built));
        }

        tokio::fs::create_dir_all(&self.path)
            .await
            .wrap_err(format!("Failed to create directory: {:?}", self.path))?;

        // Create subdirectories if specified
        for subdir in &self.subdirs {
            let subdir_path = self.path.join(subdir);
            tokio::fs::create_dir_all(&subdir_path)
                .await
                .wrap_err(format!("Failed to create subdirectory: {:?}", subdir_path))?;
        }

        Ok((self.path.clone(), task::Status::Built))
    }
}
