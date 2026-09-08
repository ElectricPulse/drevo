use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
};

#[cfg(unix)]
use std::io::{self, Read};

use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, bail};

use super::{
    button::Button,
    icon::Icon,
    layout::axis::Axis,
    linebreak::Linebreak,
    paragraph::Paragraph,
    positioning::anchor::Anchor,
    scroll::Scroll,
    text::{
        Text,
        ansi::{Ansi, Content},
    },
};
use crate::{
    DrevoCommand, DrevoMsg,
    component::Children,
    config::COMMAND_WAIT_TIMEOUT,
    geometry::Direction,
    state::Store,
    sync::Mutex,
    unicode,
    widget::{LayoutInput, SharedWidget, WidgetTrait},
};
use lucide_icons::Icon as LucideIcon;

#[derive(Clone)]
pub struct Terminal {
    directory: Store<String>,
    command: Store<String>,
    text: Store<Content>,
    scroll: SharedWidget<Scroll>,
    pub restart: bool,
    current_handle: Arc<Mutex<Option<CommandHandle>>>,
    run: Arc<Mutex<Option<Run>>>,
}

#[derive(Clone)]
struct Run {
    environment: Vec<(String, String)>,
    program: OsString,
    arguments: Vec<OsString>,
    working_dir: Option<PathBuf>,
}

impl Run {
    fn command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&self.program);
        let _ = command
            .args(&self.arguments)
            .envs(self.environment.iter().map(|(key, value)| (key, value)));

        if let Some(working_dir) = &self.working_dir {
            let _ = command.current_dir(working_dir);
        }

        command
    }

    fn display(&self) -> String {
        std::iter::once(self.program.to_string_lossy())
            .chain(
                self.arguments
                    .iter()
                    .map(|argument| argument.to_string_lossy()),
            )
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[async_trait]
impl WidgetTrait for Terminal {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout.clone()).await?;
        let directory = self.directory.affect(relayout.clone()).await?.clone();
        let command = self.command.affect(relayout.clone()).await?.clone();

        let paragraph_width = theme.units.em * 30.0;
        let label_style = theme.specific.text.paragraph.bold();

        let mut directory_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        directory_paragraph.set_styled_content(directory, theme.specific.text.paragraph);

        let mut command_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        command_paragraph.set_styled_content(command, theme.specific.text.paragraph);

        let directory_row = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::v_middle(Icon::new(LucideIcon::Folder)),
                Anchor::v_middle(Text::new("Directory:").style(label_style)),
                Anchor::v_middle(directory_paragraph),
            ),
        ));
        let command_row = if self.restart {
            let terminal = self.clone();
            let restart_button = Anchor::v_middle(Button::new(
                Icon::new(LucideIcon::RotateCw),
                move |_payload| {
                    let terminal = terminal.clone();
                    async move {
                        let _ = terminal.restart().await;
                        DrevoMsg::new(DrevoCommand::Resolve)
                    }
                },
            ));
            Anchor::left(Axis::new(
                Direction::Horizontal,
                (
                    Anchor::v_middle(Icon::new(LucideIcon::Play)),
                    Anchor::v_middle(Text::new("Command:").style(label_style)),
                    Anchor::v_middle(command_paragraph),
                    restart_button,
                ),
            ))
        } else {
            Anchor::left(Axis::new(
                Direction::Horizontal,
                (
                    Anchor::v_middle(Icon::new(LucideIcon::Play)),
                    Anchor::v_middle(Text::new("Command:").style(label_style)),
                    Anchor::v_middle(command_paragraph),
                ),
            ))
        };

        let axis = Axis::new(
            Direction::Vertical,
            (
                directory_row,
                command_row,
                Linebreak::new(Direction::Horizontal),
                self.scroll.clone(),
            ),
        );

        Ok(vec![display!(axis)])
    }
}

#[cfg(unix)]
async fn read(mut output: io::PipeReader, text: Store<Content>) -> Result<()> {
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
            let mut current = text.read().await?.clone();
            current.append(&new_text);
            text.set(current).await?;
        }
    }

    Ok(())
}

pub struct CommandHandleInner {
    read_handle: tokio::task::JoinHandle<Result<()>>,
    command_handle: tokio::process::Child,
}

pub type CommandState = Arc<Mutex<CommandHandleInner>>;

#[derive(Clone)]
pub struct CommandHandle(pub CommandState);

fn get_program_exit_status(result: std::io::Result<ExitStatus>) -> Result<()> {
    let result = result.wrap_err("")?;

    match result.success() {
        true => Ok(()),
        false => bail!("Command exited with {result}"),
    }
}

impl CommandHandle {
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

    async fn wait_locked(inner: &mut CommandHandleInner) -> Result<()> {
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
    text: Store<Content>,
) -> Result<CommandHandle> {
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
    let state = Arc::new(Mutex::new(CommandHandleInner {
        read_handle,
        command_handle,
    }));

    Ok(CommandHandle(state))
}

impl Terminal {
    pub async fn new() -> Self {
        let directory = Store::new(
            std::env::current_dir()
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .map(crate::utils::normalize_path)
                .unwrap_or_default(),
        );
        let command = Store::new(String::new());
        let text = Store::new(Content::default());
        let dark_style = crate::theme::dark_theme().specific.paper.block;
        let mut scroll = Scroll::new(Ansi::from_state(text.clone()));
        scroll.style = Some(dark_style);
        let scroll = scroll.into_shared();
        Self {
            directory,
            command,
            text,
            scroll,
            restart: false,
            current_handle: Arc::new(Mutex::new(None)),
            run: Arc::new(Mutex::new(None)),
        }
    }

    async fn start(&self, run: Run) -> Result<CommandHandle> {
        #[cfg(unix)]
        {
            let working_dir = run
                .working_dir
                .as_deref()
                .map_or_else(std::env::current_dir, |working_dir| {
                    Ok(working_dir.to_path_buf())
                })
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .map(crate::utils::normalize_path)
                .unwrap_or_default();
            self.directory.set(working_dir).await?;
            self.command.set(run.display()).await?;
            let handle = run_command(run.command(), self.text.clone())?;
            *self.current_handle.lock().await? = Some(handle.clone());
            *self.run.lock().await? = Some(run);
            Ok(handle)
        }
        #[cfg(not(unix))]
        {
            let _ = run;
            bail!("Terminal command execution is unsupported on this platform")
        }
    }

    pub async fn run<Environment, Program, Arguments, Argument, WorkingDir>(
        &self,
        environment: Environment,
        program: Program,
        arguments: Arguments,
        working_dir: Option<WorkingDir>,
    ) -> Result<CommandHandle>
    where
        Environment: IntoIterator<Item = (String, String)>,
        Program: AsRef<OsStr>,
        Arguments: IntoIterator<Item = Argument>,
        Argument: AsRef<OsStr>,
        WorkingDir: AsRef<Path>,
    {
        self.start(Run {
            environment: environment.into_iter().collect(),
            program: program.as_ref().to_os_string(),
            arguments: arguments
                .into_iter()
                .map(|argument| argument.as_ref().to_os_string())
                .collect(),
            working_dir: working_dir.map(|working_dir| working_dir.as_ref().to_path_buf()),
        })
        .await
    }

    pub async fn run_shell<Environment, Command, WorkingDir>(
        &self,
        environment: Environment,
        command: Command,
        working_dir: Option<WorkingDir>,
    ) -> Result<CommandHandle>
    where
        Environment: IntoIterator<Item = (String, String)>,
        Command: AsRef<OsStr>,
        WorkingDir: AsRef<Path>,
    {
        self.run(
            environment,
            "/bin/bash",
            vec![
                "-c".to_string(),
                format!("set -euo pipefail\n{}", command.as_ref().to_string_lossy()),
            ],
            working_dir,
        )
        .await
    }

    async fn restart(&self) -> Result<CommandHandle> {
        if let Some(handle) = self.current_handle.lock().await?.take() {
            let _ = handle.ensure_stopped().await;
        }

        self.text.set(Content::default()).await?;
        let run = self
            .run
            .lock()
            .await?
            .clone()
            .ok_or_else(|| color_eyre::eyre::eyre!("No command has been run"))?;
        self.start(run).await
    }
}
