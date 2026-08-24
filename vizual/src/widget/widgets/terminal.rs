use std::{path::Path, process::ExitStatus, sync::Arc};

#[cfg(unix)]
use std::io::{self, Read};

use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, bail};
use crate::macros::display;

use super::{
    ansi::Ansi, layout::axis::Axis, linebreak::Linebreak, paragraph::Paragraph,
    positioning::anchor::Anchor, scroll::Scroll,
};
use crate::{
    component::Children,
    config::COMMAND_WAIT_TIMEOUT,
    geometry::Direction,
    state::Store,
    sync::Mutex,
    unicode,
    widget::{Layout_input, Shared_widget, Widget_trait},
};

#[derive(Clone)]
pub struct Terminal {
    directory: Store<String>,
    shell: Store<String>,
    command: Store<String>,
    text: Store<String>,
    scroll: Shared_widget<Scroll>,
}

#[async_trait]
impl Widget_trait for Terminal {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(render.clone()).await?;
        let directory = self.directory.affect(render.clone()).await?.clone();
        let shell = self.shell.affect(render.clone()).await?.clone();
        let command = self.command.affect(render.clone()).await?.clone();

        let paragraph_width = theme.units.em * 35.0;

        let mut directory_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        directory_paragraph.set_styled_content(
            format!("Directory: {directory}"),
            theme.specific.text.paragraph,
        );

        let mut shell_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        shell_paragraph.set_styled_content(
            format!("Shell: {shell}"),
            theme.specific.text.paragraph,
        );

        let mut command_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        command_paragraph.set_styled_content(
            command,
            theme.specific.text.paragraph,
        );

        let directory = Anchor::left(directory_paragraph);
        let shell = Anchor::left(shell_paragraph);
        let command = Anchor::left(command_paragraph);

        let axis = Axis::new(
            Direction::Vertical,
            (
                directory,
                shell,
                command,
                Linebreak::new(Direction::Horizontal),
                self.scroll.clone(),
            ),
        );

        Ok(vec![display!(axis)])
    }
}

#[cfg(unix)]
async fn read(mut output: io::PipeReader, text: Store<String>) -> Result<()> {
    let mut buffer = [0_u8; 1024];
    let mut queue = Vec::new();

    loop {
        let length = output.read(&mut buffer)?;
        if length == 0 {
            break;
        }

        let mut new_bytes = std::mem::take(&mut queue);
        new_bytes.extend_from_slice(&buffer[..length]);
        let decoder = unicode::Decoder::new(new_bytes.into_iter());
        let mut new_text = String::new();

        for group in decoder {
            match group {
                Ok(character) => new_text.push(character),
                Err(unicode::Error::InvalidEndingSequence { bytes }) => queue = bytes,
                Err(unicode::Error::InvalidSequence) => new_text.push('\u{FFFD}'),
            }
        }

        if !new_text.is_empty() {
            let current = text.read().await?.clone();
            text.set(format!("{current}{new_text}")).await?;
        }
    }

    Ok(())
}

#[cfg(unix)]
fn get_command(command: &str, working_dir: Option<impl AsRef<Path>>) -> tokio::process::Command {
    let mut process = tokio::process::Command::new("/bin/bash");
    let _ = process
        .arg("-c")
        .arg(command)
        .env("CLICOLOR_FORCE", "1")
        .env("FORCE_COLOR", "1")
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor");

    if let Some(directory) = working_dir {
        let _ = process.current_dir(directory);
    }
    
    process
}

pub struct Command_handle_inner {
    read_handle: tokio::task::JoinHandle<Result<()>>,
    command_handle: tokio::process::Child,
}

pub type Command_state = Arc<Mutex<Command_handle_inner>>;

#[derive(Clone)]
pub struct Command_handle(pub Command_state);

fn get_program_exit_status(result: std::io::Result<ExitStatus>) -> Result<()> {
    let result = result.wrap_err("")?;

    match result.success() {
        true => Ok(()),
        false => bail!("Command exited with {result}"),
    }
}

impl Command_handle {
    pub fn ensure_stopped_in_background(&self) {
        let handle = self.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            drop(runtime.spawn(async move {
                let _ = handle.ensure_stopped().await;
            }));
        } else {
            let _ = std::thread::spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                let _ = runtime.block_on(handle.ensure_stopped());
            });
        }
    }

    pub async fn ensure_stopped(&self) -> Result<()> {
        let mut inner = self.0.lock().await?;
        if inner.command_handle.try_wait()?.is_none() {
            inner.command_handle.kill().await?;
            let _ = Self::wait_locked(&mut inner).await;
            Ok(())
        } else {
            Self::wait_locked(&mut inner).await
        }
    }

    pub async fn wait(&self) -> Result<()> {
        let mut inner = self.0.lock().await?;
        Self::wait_locked(&mut inner).await
    }

    async fn wait_locked(inner: &mut Command_handle_inner) -> Result<()> {
        tokio::select! {
            command_handle = inner.command_handle.wait() => {
                inner.read_handle.abort();
                get_program_exit_status(command_handle)
            }
            read_result = &mut inner.read_handle => {
                let command_handle = match tokio::time::timeout(
                    COMMAND_WAIT_TIMEOUT,
                    inner.command_handle.wait()
                ).await {
                    Ok(result) => result,
                    Err(_) => {
                        let _ = inner.command_handle.kill().await;
                        inner.command_handle.wait().await
                    }
                };
                
                let command_handle = get_program_exit_status(command_handle);

                match command_handle {
                    Err(error) => Err(error),
                    Ok(()) => read_result.wrap_err("")?,
                }
            }
        }
    }
}

#[cfg(unix)]
fn run_command(
    mut command: tokio::process::Command,
    text: Store<String>,
) -> Result<Command_handle> {
    let (output_reader, stdout) = io::pipe().wrap_err("")?;
    let stderr = stdout.try_clone().wrap_err("")?;
    let _ = command.kill_on_drop(true);
    let command_handle = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(stdout))
        .stderr(std::process::Stdio::from(stderr))
        .spawn()
        .wrap_err("")?;

    let read_handle = tokio::spawn(read(output_reader, text));
    let state = Arc::new(Mutex::new(Command_handle_inner {
        read_handle,
        command_handle,
    }));

    Ok(Command_handle(state))
}

impl Terminal {
    pub fn new() -> Self {
        let directory = Store::new(
            std::env::current_dir()
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        );
        let shell = Store::new("/bin/bash".to_string());
        let command = Store::new(String::new());
        let text = Store::new(String::new());
        let dark_style = crate::theme::dark_theme().specific.paper.block;
        let mut scroll = Scroll::new(Ansi::new(text.clone()));
        scroll.style = Some(dark_style);
        let scroll = scroll.into_shared();
        Self {
            directory,
            shell,
            command,
            text,
            scroll,
        }
    }

    fn update_info(&self, directory: String, shell: String, command: String) {
        let dir_store = self.directory.clone();
        let shell_store = self.shell.clone();
        let cmd_store = self.command.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            drop(runtime.spawn(async move {
                let _ = dir_store.set(directory).await;
                let _ = shell_store.set(shell).await;
                let _ = cmd_store.set(command).await;
            }));
        } else {
            let _ = std::thread::spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                let _ = runtime.block_on(async move {
                    let _ = dir_store.set(directory).await;
                    let _ = shell_store.set(shell).await;
                    let _ = cmd_store.set(command).await;
                });
            });
        }
    }

    pub fn run(&self, args: impl Into<String>) -> Result<Command_handle> {
        let command = args.into();
        #[cfg(unix)]
        {
            let current_dir = std::env::current_dir()
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            self.update_info(current_dir, "/bin/bash".to_string(), command.clone());
            run_command(get_command(&command, None::<&Path>), self.text.clone())
        }
        #[cfg(not(unix))]
        {
            let _ = command;
            bail!("Terminal command execution is unsupported on this platform")
        }
    }

    pub fn run_in_dir(
        &self,
        args: impl Into<String>,
        working_dir: impl AsRef<Path>,
    ) -> Result<Command_handle> {
        let command = args.into();
        #[cfg(unix)]
        {
            let canonical_dir = std::fs::canonicalize(working_dir.as_ref())
                .unwrap_or_else(|_| working_dir.as_ref().to_path_buf());
            let dir_str = canonical_dir.display().to_string();
            self.update_info(dir_str, "/bin/bash".to_string(), command.clone());
            run_command(get_command(&command, Some(&canonical_dir)), self.text.clone())
        }
        #[cfg(not(unix))]
        {
            let _ = (command, working_dir);
            bail!("Terminal command execution is unsupported on this platform")
        }
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}
