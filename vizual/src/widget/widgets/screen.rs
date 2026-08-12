use std::{path::Path, process::ExitStatus, sync::Arc};

#[cfg(unix)]
use std::io::{self, Read};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, bail};
use vizual_macros::display;

use super::{
    super::{Focus_provider, Shared_widget, Widget_trait},
    text_viewport::Text_viewport,
    title_block::Title_block,
};
use crate::{
    Render, Vizual_command, Vizual_msg,
    component::{Children, context::Component_context},
    config::COMMAND_WAIT_TIMEOUT,
    display::Display,
    event::{Event, Key_code, Key_event, Wheel_delta},
    geometry::Rect,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    sync::Mutex,
    theme::Theme,
    unicode,
};

type Text = Arc<ArcSwap<String>>;

/// The process output text is intentionally shared, while each clone gets an independent viewport
/// whose renderer-derived Parley layout cache is rebuilt on demand.
#[derive(Clone)]
struct Screen_content {
    text: Text,
    viewport: Text_viewport,
    following: bool,
    last_text_len: usize,
}

#[derive(Clone)]
pub struct Screen {
    command: String,
    text: Text,
    content: Shared_widget<Screen_content>,
    render: Render,
}

#[cfg(unix)]
async fn read(mut output: io::PipeReader, render: Render, text: Text) -> Result<()> {
    let mut buffer = [0_u8; 1024];
    let mut queue = Vec::new();

    loop {
        let length = output.read(&mut buffer)?;
        if length == 0 {
            break;
        }

        let mut new_bytes = queue.clone();
        new_bytes.extend_from_slice(&buffer[..length]);
        queue.clear();
        let decoder = unicode::Decoder::new(new_bytes.into_iter());
        let mut new_text = String::new();

        for group in decoder {
            match group {
                Ok(character) => new_text.push(character),
                Err(unicode::Error::InvalidEndingSequence { bytes }) => queue = bytes,
                Err(unicode::Error::InvalidSequence) => new_text.push('�'),
            }
        }

        let _ = text.rcu(|old| {
            let mut updated = String::with_capacity(old.len() + new_text.len());
            updated.push_str(old);
            updated.push_str(&new_text);
            Arc::new(updated)
        });
        render.send();
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
    render: Render,
    text: Text,
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
    let read_handle = tokio::spawn(read(output_reader, render, text));
    let state = Arc::new(Mutex::new(Command_handle_inner {
        read_handle,
        command_handle,
    }));
    Ok(Command_handle(state))
}

impl Screen {
    pub fn new(render: Render) -> Self {
        let text = Arc::new(ArcSwap::from_pointee(String::new()));
        Self {
            command: String::new(),
            text: text.clone(),
            content: Screen_content {
                text,
                viewport: Text_viewport::new(),
                following: true,
                last_text_len: 0,
            }
            .into_shared(),
            render,
        }
    }

    pub fn run(&mut self, args: impl Into<String>) -> Result<Command_handle> {
        self.command = args.into();
        #[cfg(unix)]
        {
            let command = get_command(&self.command, None::<String>);
            run_command(command, self.render.clone(), self.text.clone())
        }
        #[cfg(not(unix))]
        {
            bail!("Screen command execution is unsupported on this platform")
        }
    }

    pub fn run_in_dir(
        &mut self,
        args: impl Into<String>,
        working_dir: impl AsRef<Path>,
    ) -> Result<Command_handle> {
        self.command = args.into();
        #[cfg(unix)]
        {
            let command = get_command(&self.command, Some(working_dir));
            run_command(command, self.render.clone(), self.text.clone())
        }
        #[cfg(not(unix))]
        {
            let _ = working_dir;
            bail!("Screen command execution is unsupported on this platform")
        }
    }
}

impl Screen_content {
    fn update_following_state(&mut self) {
        if self.viewport.offset().y != self.viewport.maximum_offset().y {
            self.following = false;
        }
    }

    fn scroll_y(&mut self, amount: f64) {
        self.viewport.scroll_y(amount);
        self.update_following_state();
    }

    fn status(&self) -> String {
        let total = self.viewport.line_count();
        if self.viewport.offset().y != self.viewport.maximum_offset().y {
            let current = self.viewport.current_line().saturating_add(1);
            let percent = match total {
                0 => 0,
                _ => current * 100 / total,
            };
            format!(
                " [Line {current}/{total} ({percent}%) - {} lines back | f:follow g/G:top/bottom]",
                self.viewport.lines_from_end()
            )
        } else {
            format!(" [FOLLOWING - {total} lines]")
        }
    }
}

#[async_trait]
impl Widget_trait for Screen_content {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        Ok(vec![])
    }

    async fn render(
        &mut self,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        let text = self.text.load();
        if text.len() != self.last_text_len {
            self.viewport.set_content(&text);
            self.last_text_len = text.len();
        }

        self.viewport.prepare(display, hitbox.size);
        if self.following {
            self.viewport.jump_to_bottom();
        }
        self.viewport.paint(display, hitbox.origin);
        Ok(None)
    }
}

#[async_trait]
impl Widget_trait for Screen {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);
        let title = {
            let content = self.content.lock().await?;
            format!("{}{}", self.command, content.status())
        };

        let block = Title_block::new(self.content.clone(), title);
        Ok(vec![display!(block)])
    }

    async fn on_key_press(&mut self, event: &Key_event) -> Result<Vizual_msg> {
        let mut content = self.content.lock().await?;
        let line = content.viewport.line_step();
        match event.code {
            Key_code::Arrow_up => content.scroll_y(-line),
            Key_code::Arrow_down => content.scroll_y(line),
            Key_code::Page_up => {
                let height = content.viewport.viewport_size().height;
                content.scroll_y(-height);
            }
            Key_code::Page_down => {
                let height = content.viewport.viewport_size().height;
                content.scroll_y(height);
            }
            Key_code::Home | Key_code::Character('g') => {
                content.viewport.jump_to_top();
                content.following = false;
            }
            Key_code::End | Key_code::Character('G') if event.modifiers.shift => {
                content.viewport.jump_to_bottom();
                content.following = false;
            }
            Key_code::End => {
                content.viewport.jump_to_bottom();
                content.following = false;
            }
            Key_code::Character('f' | 'F') => {
                content.viewport.jump_to_bottom();
                content.following = true;
            }
            _ => return Vizual_msg::none(),
        }
        Vizual_msg::new(Vizual_command::Layout)
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let Event::Wheel(wheel) = event else {
            return Vizual_msg::none();
        };
        let mut content = self.content.lock().await?;
        match wheel.delta {
            Wheel_delta::Lines(delta) => {
                let line = content.viewport.line_step();
                content.scroll_y(-delta.y * line * 5.0);
            }
            Wheel_delta::Logical_pixels(delta) => content.scroll_y(-delta.y),
        }
        Vizual_msg::new(Vizual_command::Layout)
    }
}
