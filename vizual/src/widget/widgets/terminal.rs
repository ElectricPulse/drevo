use std::{path::Path, process::ExitStatus, sync::Arc};

#[cfg(unix)]
use std::io::{self, Read};

use color_eyre::eyre::{Result, WrapErr, bail};

use super::{scroll::Scroll, text::Text};
use crate::{
    config::COMMAND_WAIT_TIMEOUT,
    state::Store,
    sync::Mutex,
    unicode,
    widget::{Shared_widget, Widget_trait as _},
};

#[derive(Clone, vizual_macros::Widget_trait)]
#[widget_trait(field = content)]
pub struct Terminal {
    text: Store<String>,
    content: Shared_widget<Scroll>,
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
                Err(unicode::Error::InvalidSequence) => new_text.push('�'),
            }
        }

        if !new_text.is_empty() {
            text.write().await?.push_str(&new_text);
        }
    }

    Ok(())
}

#[cfg(unix)]
fn get_command(command: &str, working_dir: Option<impl AsRef<Path>>) -> tokio::process::Command {
    let mut process = tokio::process::Command::new("/bin/bash");
    let _ = process.arg("-c").arg(command);

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
        let text = Store::new(String::new());
        let content = Scroll::new(Text::new(text.clone())).into_shared();
        Self { text, content }
    }

    pub fn run(&self, args: impl Into<String>) -> Result<Command_handle> {
        let command = args.into();
        #[cfg(unix)]
        {
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
            run_command(get_command(&command, Some(working_dir)), self.text.clone())
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
