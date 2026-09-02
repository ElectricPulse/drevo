use std::{
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
    ansi::{Ansi, Content},
    button::Button,
    icon::Icon,
    layout::axis::Axis,
    linebreak::Linebreak,
    paragraph::Paragraph,
    positioning::anchor::Anchor,
    scroll::Scroll,
    text::Text,
};
use crate::{
    Vizual_command, Vizual_msg,
    component::Children,
    config::COMMAND_WAIT_TIMEOUT,
    geometry::Direction,
    state::Store,
    sync::Mutex,
    unicode,
    widget::{Layout_input, Shared_widget, Widget_trait},
};
use lucide_icons::Icon as Lucide_icon;

#[derive(Clone)]
pub struct Terminal {
    directory: Store<String>,
    shell: Store<String>,
    command: Store<String>,
    text: Store<Content>,
    scroll: Shared_widget<Scroll>,
    pub restart: bool,
    current_handle: Arc<Mutex<Option<Command_handle>>>,
    working_dir: Arc<Mutex<Option<PathBuf>>>,
    envs: Arc<Mutex<Vec<(String, String)>>>,
}

#[async_trait]
impl Widget_trait for Terminal {
    async fn layout(
        &mut self,
        Layout_input {
            relayout,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout.clone()).await?;
        let directory = self.directory.affect(relayout.clone()).await?.clone();
        let shell = self.shell.affect(relayout.clone()).await?.clone();
        let command = self.command.affect(relayout.clone()).await?.clone();

        let paragraph_width = theme.units.em * 30.0;
        let label_style = theme.specific.text.paragraph.bold();

        let mut directory_label = Text::new("Directory:");
        directory_label.style.set(label_style);
        let mut directory_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        directory_paragraph.set_styled_content(directory, theme.specific.text.paragraph);

        let mut shell_label = Text::new("Shell:");
        shell_label.style.set(label_style);
        let mut shell_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        shell_paragraph.set_styled_content(shell, theme.specific.text.paragraph);

        let mut command_label = Text::new("Command:");
        command_label.style.set(label_style);
        let mut command_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        command_paragraph.set_styled_content(command, theme.specific.text.paragraph);

        let directory_row = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::v_middle(Icon::new(Lucide_icon::Folder)),
                Anchor::v_middle(directory_label),
                Anchor::v_middle(directory_paragraph),
            ),
        ));
        let shell_row = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::v_middle(Icon::new(Lucide_icon::Terminal)),
                Anchor::v_middle(shell_label),
                Anchor::v_middle(shell_paragraph),
            ),
        ));

        let command_row = if self.restart {
            let terminal = self.clone();
            let restart_button = Anchor::v_middle(Button::new(
                Icon::new(Lucide_icon::RotateCw),
                move |_payload| {
                    let terminal = terminal.clone();
                    async move {
                        let _ = terminal.restart().await;
                        Vizual_msg::new(Vizual_command::Resolve)
                    }
                },
            ));
            Anchor::left(Axis::new(
                Direction::Horizontal,
                (
                    Anchor::v_middle(Icon::new(Lucide_icon::Play)),
                    Anchor::v_middle(command_label),
                    Anchor::v_middle(command_paragraph),
                    restart_button,
                ),
            ))
        } else {
            Anchor::left(Axis::new(
                Direction::Horizontal,
                (
                    Anchor::v_middle(Icon::new(Lucide_icon::Play)),
                    Anchor::v_middle(command_label),
                    Anchor::v_middle(command_paragraph),
                ),
            ))
        };

        let axis = Axis::new(
            Direction::Vertical,
            (
                directory_row,
                shell_row,
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

#[cfg(unix)]
fn get_command(
    command: &str,
    working_dir: Option<impl AsRef<Path>>,
    envs: &[(String, String)],
) -> tokio::process::Command {
    let mut process = tokio::process::Command::new("/bin/bash");
    let _ = process
        .arg("-c")
        .arg(command)
        .env("CLICOLOR_FORCE", "1")
        .env("FORCE_COLOR", "1")
        .env("TERM", "xterm-256color")
        .env("COLORTERM", "truecolor");

    for (key, val) in envs {
        let _ = process.env(key, val);
    }

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
    text: Store<Content>,
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
    pub async fn new() -> Self {
        let directory = Store::new(
            std::env::current_dir()
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        );
        let shell = Store::new("/bin/bash".to_string());
        let command = Store::new(String::new());
        let text = Store::new(Content::default());
        let dark_style = crate::theme::dark_theme().specific.paper.block;
        let mut scroll = Scroll::new(Ansi::from_state(text.clone()));
        scroll.style = Some(dark_style);
        let scroll = scroll.into_shared();
        Self {
            directory,
            shell,
            command,
            text,
            scroll,
            restart: false,
            current_handle: Arc::new(Mutex::new(None)),
            working_dir: Arc::new(Mutex::new(None)),
            envs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_restart(mut self, restart: bool) -> Self {
        self.restart = restart;
        self
    }

    pub fn with_env(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let Ok(mut envs) = self.envs.try_lock() {
            envs.push((key.into(), value.into()));
        }
        self
    }

    pub fn env(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.with_env(key, value)
    }

    pub fn with_envs<I, K, V>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        if let Ok(mut envs) = self.envs.try_lock() {
            for (k, v) in iter {
                envs.push((k.into(), v.into()));
            }
        }
        self
    }

    pub fn envs<I, K, V>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.with_envs(iter)
    }

    pub async fn set_env(&self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let mut envs = self.envs.lock().await?;
        envs.push((key.into(), value.into()));
        Ok(())
    }

    pub async fn run(&self, args: impl Into<String>) -> Result<Command_handle> {
        let command = args.into();
        #[cfg(unix)]
        {
            let current_dir = std::env::current_dir()
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            self.directory.set(current_dir).await?;
            self.shell.set("/bin/bash".to_string()).await?;
            self.command.set(command.clone()).await?;
            *self.working_dir.lock().await? = None;
            let envs = self.envs.lock().await?.clone();
            let handle = run_command(
                get_command(&command, None::<&Path>, &envs),
                self.text.clone(),
            )?;
            *self.current_handle.lock().await? = Some(handle.clone());
            Ok(handle)
        }
        #[cfg(not(unix))]
        {
            let _ = command;
            bail!("Terminal command execution is unsupported on this platform")
        }
    }

    pub async fn run_in_dir(
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
            self.directory.set(dir_str).await?;
            self.shell.set("/bin/bash".to_string()).await?;
            self.command.set(command.clone()).await?;
            *self.working_dir.lock().await? = Some(canonical_dir.clone());
            let envs = self.envs.lock().await?.clone();
            let handle = run_command(
                get_command(&command, Some(&canonical_dir), &envs),
                self.text.clone(),
            )?;
            *self.current_handle.lock().await? = Some(handle.clone());
            Ok(handle)
        }
        #[cfg(not(unix))]
        {
            let _ = (command, working_dir);
            bail!("Terminal command execution is unsupported on this platform")
        }
    }

    pub async fn run_command(&self, command: tokio::process::Command) -> Result<Command_handle> {
        #[cfg(unix)]
        {
            let handle = run_command(command, self.text.clone())?;
            *self.current_handle.lock().await? = Some(handle.clone());
            Ok(handle)
        }
        #[cfg(not(unix))]
        {
            let _ = command;
            bail!("Terminal command execution is unsupported on this platform")
        }
    }

    pub async fn restart(&self) -> Result<Command_handle> {
        let mut handle_lock = self.current_handle.lock().await?;
        if let Some(handle) = handle_lock.take() {
            let _ = handle.ensure_stopped().await;
        }

        self.text.set(Content::default()).await?;

        let command = self.command.read().await?.clone();
        let working_dir = self.working_dir.lock().await?.clone();
        let envs = self.envs.lock().await?.clone();

        #[cfg(unix)]
        {
            let handle = run_command(
                get_command(&command, working_dir.as_deref(), &envs),
                self.text.clone(),
            )?;
            *handle_lock = Some(handle.clone());
            Ok(handle)
        }
        #[cfg(not(unix))]
        {
            let _ = (command, working_dir);
            bail!("Terminal command execution is unsupported on this platform")
        }
    }
}
