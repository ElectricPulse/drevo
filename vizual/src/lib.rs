#![feature(async_fn_track_caller)]
pub mod backend;
pub mod component;
mod config;
pub mod event;
pub mod focus;
pub mod geometry;
pub mod handlers;
pub mod hitbox;
pub mod layouter;
pub mod log;
pub mod slot_manager;
pub mod state;
pub mod style;
pub mod sync;
pub mod theme;
pub mod unicode;
pub mod utils;
pub mod widget;

extern crate self as vizual;

use std::{fs::OpenOptions, path::Path, sync::Arc};

use async_recursion::async_recursion;
use color_eyre::eyre::{ContextCompat, Result, WrapErr, eyre};
use good_lp::Expression;
use simplelog::{CombinedLogger, Config as Log_config, LevelFilter, WriteLogger};
use tokio::sync::mpsc;
use vello::{
    AaConfig, Renderer, RendererOptions, Scene,
    kurbo::Affine,
    peniko::color::palette,
    util::{RenderContext, RenderSurface},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

use backend::graphics::{Paint_context, Text_resources};
use component::{Child, Child_reference, Child_slot};
use config::DEFAULT_SCREEN_SIZE;
use event::{
    Event, Key_code, Key_event, Modifiers, Pointer_button, Pointer_event, Wheel_delta, Wheel_event,
};
use focus::{Focus, Focus_search_direction};
use geometry::{Point, Size};
use hitbox::Hitbox;
use layouter::{Problem, Problem_context, Solution};
use log::{log_duration, log_info};
use state::State;
use sync::Mutex;
use theme::Theme;
use widget::{Renderable, Shared_renderable, widgets::root::Root};
pub fn init_logging(path: impl AsRef<Path>) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .wrap_err_with(|| format!("Failed to open log file {}", path.as_ref().display()))?;

    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Log_config::default(),
        file,
    )])
    .map_err(|error| eyre!("Failed to initialize logging: {error}"))?;

    Ok(())
}

#[derive(Clone)]
pub struct Vizual_msg {
    pub(crate) propagate: bool,
    pub(crate) command: Vizual_command,
}

impl Vizual_msg {
    pub fn new(command: Vizual_command) -> Result<Self> {
        Ok(Self {
            propagate: false,
            command,
        })
    }

    pub fn new_propagated(command: Vizual_command) -> Result<Self> {
        Ok(Self {
            propagate: true,
            command,
        })
    }

    pub fn none() -> Result<Self> {
        Ok(Self::bare())
    }

    pub fn bare() -> Self {
        Self {
            propagate: true,
            command: Vizual_command::None,
        }
    }

    pub(crate) fn has_command(&self) -> bool {
        !matches!(self.command, Vizual_command::None)
    }

    fn join(&mut self, message: Vizual_msg) {
        self.command = self.command.clone().join(message.command);
        self.propagate = self.propagate && message.propagate;
    }
}

#[derive(Clone)]
pub enum Vizual_command {
    None,
    Layout,
    Resolve,
    Render,
    Focus(Child_reference),
    Quit,
}

impl Vizual_command {
    fn join(self, command: Self) -> Self {
        match command {
            Self::None => self,
            command => command,
        }
    }
}

#[derive(Clone)]
pub struct Rerender(mpsc::UnboundedSender<()>);

impl Rerender {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<()>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self(sender), receiver)
    }

    pub fn send(&self) {
        let _ = self.0.send(());
    }
}

pub fn check_quit_event(event: &Key_event) -> bool {
    matches!(event.code, Key_code::Character('c' | 'C')) && event.modifiers.control
}

struct App_problem {
    root: Child,
    root_hitbox: Hitbox,
    problem_context: Problem_context,
}

impl App_problem {
    async fn new<T: Renderable>(
        root: Shared_renderable<T>,
        root_slot: &mut Child_slot,
        text_resources: Arc<Mutex<Text_resources>>,
    ) -> Result<Self> {
        let shared_problem = Arc::new(Mutex::new(Problem::new()));
        let problem_context = Problem_context::new(shared_problem, text_resources).await?;
        let root = root_slot.set(root, problem_context.clone()).await?;
        let root_hitbox = root.get_hitbox().await?;
        problem_context
            .lock()
            .await?
            .constrain_root_to_screen(root_hitbox);

        Ok(Self {
            root,
            root_hitbox,
            problem_context,
        })
    }

    async fn layout(&mut self) -> Result<()> {
        let children = self.root.layout(None, self.problem_context.clone()).await?;
        self.root
            .layout_children(children, self.problem_context.clone())
            .await?;

        // TODO: this is all very confusing
        // Since solve_minimal gets added before root is constrained to screen
        // it makes sense to minimize the root - which makes the solve_minimal function
        // and then without having to recreate the problem constrain root to a fixed size of the window
        // and solve again normally
        for dimension in [
            self.root_hitbox.dimensions.width,
            self.root_hitbox.dimensions.height,
        ] {
            self.problem_context
                .minimize(Expression::from(dimension), 5)
                .await?;
        }

        Ok(())
    }

    fn root_size(&self, solution: &Solution) -> Size {
        Size::new(
            solution.value(self.root_hitbox.dimensions.width),
            solution.value(self.root_hitbox.dimensions.height),
        )
    }

    async fn minimum_size(&self) -> Result<Size> {
        let solution = self.problem_context.lock().await?.solve_minimum().await?;
        let minimum_size = self.root_size(&solution);
        // TODO: Without this padding the user can still push the screen below the required size somehow, causing the layout to crash.
        Ok(Size::new(
            minimum_size.width + 1.0,
            minimum_size.height + 1.0,
        ))
    }

    async fn render(
        &mut self,
        solution: &Solution,
        focus: &mut Focus,
        text_resources: &Arc<Mutex<Text_resources>>,
    ) -> Result<Scene> {
        let mut scene = Scene::new();
        let mut resources = text_resources.lock().await?;
        let mut paint = Paint_context::new(&mut scene, &mut resources);
        self.root
            .render(focus.clone(), &mut paint, solution)
            .await?;
        Ok(scene)
    }

    #[async_recursion]
    async fn handle_pointer_press(
        &mut self,
        node: Child,
        position: Point,
        event: &Event,
        solution: &Solution,
        focus: &mut Focus,
    ) -> Result<Vizual_msg> {
        let node_lock = node.lock().await?;
        if !node_lock.hitbox.hits(solution, position) {
            return Vizual_msg::none();
        }

        let children = node_lock.children.clone();
        drop(node_lock);
        let mut total_message = Vizual_msg::bare();

        for child in children.iter().rev() {
            let message = self
                .handle_pointer_press(child.clone(), position, event, solution, focus)
                .await?;
            total_message.join(message);
            if !total_message.propagate {
                return Ok(total_message);
            }
        }

        let mut node_lock = node.lock().await?;
        if node_lock.focusable {
            focus.set(&node);
            return Vizual_msg::new(Vizual_command::Layout);
        }

        let message = node_lock.widget.forward_event(event).await?;
        match message.propagate {
            false => Ok(message),
            true => Vizual_msg::none(),
        }
    }

    async fn drill_event(&mut self, event: &Event, focus: &mut Focus) -> Result<Vizual_command> {
        let Some(mut current_node) = focus.upgrade() else {
            return Ok(Vizual_command::None);
        };
        let mut nodes = Vec::new();

        loop {
            nodes.push(current_node.clone());
            let node = current_node.lock().await?;
            let Some(parent) = node
                .parent
                .as_ref()
                .map(|parent| parent.upgrade().wrap_err("Found link to stale parent"))
            else {
                break;
            };
            let parent = parent?;
            drop(node);
            current_node = Child::new(parent);
        }

        if !self.root.compare(&current_node) {
            return Err(eyre!("The last found parent should be the root"));
        }

        let mut message = Vizual_msg::none()?;
        for node in &nodes {
            let new_message = node.lock().await?.widget.forward_event(event).await?;
            message.join(new_message);
            if !message.propagate {
                break;
            }
        }

        Ok(message.command)
    }

    async fn move_focus(
        &mut self,
        skip_count: usize,
        direction: Focus_search_direction,
        focus: &mut Focus,
    ) -> Result<bool> {
        let start = focus.upgrade().unwrap_or_else(|| self.root.clone());
        let (found, skip_count) = self
            .find_focus(None, start, skip_count, direction, true, focus)
            .await?;
        if found {
            return Ok(true);
        }

        let (found, _) = self
            .find_focus(None, self.root.clone(), skip_count, direction, true, focus)
            .await?;
        Ok(found)
    }

    #[async_recursion]
    async fn find_focus(
        &mut self,
        origin: Option<usize>,
        node: Child,
        mut skip_count: usize,
        direction: Focus_search_direction,
        up: bool,
        focus: &mut Focus,
    ) -> Result<(bool, usize)> {
        let lock = node.lock().await?;

        if !up && lock.focusable && !focus.compare(&node) {
            if skip_count == 0 {
                focus.set(&node);
                return Ok((true, skip_count));
            }
            skip_count -= 1;
        }

        let children_count = lock.children.len();
        let child_iterator: Box<dyn Iterator<Item = usize> + Send> = match (origin, direction) {
            (Some(origin), Focus_search_direction::Right) => Box::new(origin + 1..children_count),
            (Some(origin), Focus_search_direction::Left) => Box::new((0..origin).rev()),
            (None, Focus_search_direction::Right) => Box::new(0..children_count),
            (None, Focus_search_direction::Left) => Box::new((0..children_count).rev()),
        };

        for child in child_iterator.map(|index| &lock.children[index]) {
            let (found, remaining) = self
                .find_focus(None, child.clone(), skip_count, direction, false, focus)
                .await?;
            skip_count = remaining;
            if found {
                return Ok((true, skip_count));
            }
        }

        if !up {
            return Ok((false, skip_count));
        }

        if lock.focusable && !focus.compare(&node) {
            if skip_count == 0 {
                focus.set(&node);
                return Ok((true, skip_count));
            }
            skip_count -= 1;
        }

        let Some(parent) = lock
            .parent
            .as_ref()
            .map(|parent| parent.upgrade().wrap_err("Found link to stale parent"))
        else {
            return Ok((false, skip_count));
        };
        let parent = parent?;
        let new_origin = parent
            .lock()
            .await?
            .children
            .iter()
            .position(|child| child.compare(&node))
            .wrap_err("Node not found inside the children of its parent")?;

        self.find_focus(
            Some(new_origin),
            Child::new(parent),
            skip_count,
            direction,
            true,
            focus,
        )
        .await
    }

    async fn handle_event(
        &mut self,
        event: &Event,
        solution: &Solution,
        focus: &mut Focus,
    ) -> Result<Vizual_command> {
        let command = self.drill_event(event, focus).await?;
        if !matches!(command, Vizual_command::None) {
            return Ok(command);
        }

        match event {
            Event::Key(key) => match key.code {
                Key_code::Tab | Key_code::Back_tab => {
                    let direction = match key.code {
                        Key_code::Tab => Focus_search_direction::Right,
                        _ => Focus_search_direction::Left,
                    };
                    let _ = self.move_focus(0, direction, focus).await?;
                    Ok(Vizual_command::Layout)
                }
                Key_code::Escape => {
                    focus.reset();
                    Ok(Vizual_command::Layout)
                }
                _ if check_quit_event(key) => Ok(Vizual_command::Quit),
                _ => Ok(Vizual_command::None),
            },
            Event::Pointer(pointer) => Ok(self
                .handle_pointer_press(self.root.clone(), pointer.position, event, solution, focus)
                .await?
                .command),
            Event::Close_requested => Ok(Vizual_command::Quit),
            Event::Wheel(_) | Event::Text(_) => Ok(Vizual_command::None),
        }
    }
}

enum Ui_input {
    Initialize(Size),
    Event(Event),
    Resize(Size),
    Rerender,
}

// TODO: This message-passing bullshit is probably not needed.
enum User_event {
    Initialize(Size, Size),
    Minimum_size(Size),
    Scene(Scene),
    Exit,
    Error(String),
}

async fn layout_problem<T: Renderable>(
    root: Shared_renderable<T>,
    root_slot: &mut Child_slot,
    text_resources: Arc<Mutex<Text_resources>>,
) -> Result<(App_problem, Size)> {
    let mut problem = App_problem::new(root, root_slot, text_resources).await?;
    log_duration(0, "app problem layout", || problem.layout()).await?;
    let minimum_size = problem.minimum_size().await?;
    Ok((problem, minimum_size))
}

async fn ui_loop<T: Renderable>(
    root: Shared_renderable<T>,
    mut render_signal: mpsc::UnboundedReceiver<()>,
    mut input_receiver: mpsc::UnboundedReceiver<Ui_input>,
    proxy: EventLoopProxy<User_event>,
) -> Result<()> {
    let mut focus = Focus::new();
    let mut root_slot = Child_slot::new();
    let text_resources = Arc::new(Mutex::new(Text_resources::new()));
    let mut app_problem: Option<App_problem> = None;
    let mut solution = None;
    let mut window_size = None;
    let mut pending_minimum_size: Option<Size> = None;
    let mut rerender_open = true;
    let mut buffered_input = None;

    loop {
        let input = match buffered_input.take() {
            Some(input) => Some(input),
            None => {
                tokio::select! {
                    input = input_receiver.recv() => input,
                    rerender = render_signal.recv(), if rerender_open => {
                        match rerender {
                            Some(()) => {
                                while render_signal.try_recv().is_ok() {}
                                Some(Ui_input::Rerender)
                            }
                            None => {
                                rerender_open = false;
                                continue;
                            }
                        }
                    }
                }
            }
        };

        let Some(input) = input else {
            break;
        };

        // TODO: This resize buffering code is slop.
        let input = match input {
            Ui_input::Resize(mut latest_size) => {
                loop {
                    match input_receiver.try_recv() {
                        Ok(Ui_input::Resize(size)) => latest_size = size,
                        Ok(input) => {
                            buffered_input = Some(input);
                            break;
                        }
                        Err(_) => break,
                    }
                }

                Ui_input::Resize(latest_size)
            }
            input => input,
        };

        let mut command = match input {
            Ui_input::Initialize(maximum_size) => {
                // This technically performs one extra layout call before the window-driven layout loop starts.
                let (_, minimum_size) =
                    layout_problem(root.clone(), &mut root_slot, text_resources.clone()).await?;
                let default_size = Size::new(
                    DEFAULT_SCREEN_SIZE.width.min(maximum_size.width),
                    DEFAULT_SCREEN_SIZE.height.min(maximum_size.height),
                );
                let initial_size = Size::new(
                    minimum_size.width.max(default_size.width),
                    minimum_size.height.max(default_size.height),
                );
                let dimension_source = |minimum: f64, default: f64, maximum: f64| match (
                    minimum >= default.min(maximum),
                    maximum < default,
                ) {
                    (true, _) => "layout minimum dimension snapped",
                    (false, true) => "screen display size bound snapped",
                    (false, false) => "default snapped",
                };
                let width_source = dimension_source(
                    minimum_size.width,
                    DEFAULT_SCREEN_SIZE.width,
                    maximum_size.width,
                );
                let height_source = dimension_source(
                    minimum_size.height,
                    DEFAULT_SCREEN_SIZE.height,
                    maximum_size.height,
                );
                log_info(
                    0,
                    format_args!(
                        "initial screen size {initial_size:?}; width: {width_source}; height: {height_source}; minimum layout: {minimum_size:?}, default: {DEFAULT_SCREEN_SIZE:?}, display: {maximum_size:?}"
                    ),
                );
                if proxy
                    .send_event(User_event::Initialize(initial_size, minimum_size))
                    .is_err()
                {
                    break;
                }
                continue;
            }
            Ui_input::Resize(size) => {
                window_size = Some(size);
                match pending_minimum_size {
                    Some(minimum) if size.width < minimum.width || size.height < minimum.height => {
                        continue;
                    }
                    Some(_) => {
                        pending_minimum_size = None;
                        Vizual_command::Resolve
                    }
                    None => match app_problem.is_some() {
                        true => Vizual_command::Resolve,
                        false => Vizual_command::Layout,
                    },
                }
            }
            Ui_input::Rerender => Vizual_command::Layout,
            Ui_input::Event(event) => match (&mut app_problem, &solution) {
                (Some(problem), Some(solution)) => {
                    problem.handle_event(&event, solution, &mut focus).await?
                }
                _ if matches!(event, Event::Close_requested) => Vizual_command::Quit,
                _ => Vizual_command::None,
            },
        };

        if matches!(command, Vizual_command::Quit) {
            let _ = proxy.send_event(User_event::Exit);
            break;
        }

        let Some(size) = window_size else {
            continue;
        };

        if matches!(command, Vizual_command::Layout) {
            let (problem, minimum) =
                layout_problem(root.clone(), &mut root_slot, text_resources.clone()).await?;
            app_problem = Some(problem);
            pending_minimum_size = Some(minimum);
            if proxy.send_event(User_event::Minimum_size(minimum)).is_err() {
                break;
            }
            continue;
        } else if matches!(command, Vizual_command::Resolve) {
            if let Some(problem) = &app_problem {
                solution = Some(problem.problem_context.lock().await?.solve(size).await?);
                command = Vizual_command::Render;
            }
        } else if let Vizual_command::Focus(reference) = &command {
            focus.set_with_reference(reference);
            command = Vizual_command::Render;
        }

        if matches!(command, Vizual_command::Render)
            && let (Some(problem), Some(solution)) = (&mut app_problem, &solution)
        {
            let scene = problem
                .render(solution, &mut focus, &text_resources)
                .await?;
            if proxy.send_event(User_event::Scene(scene)).is_err() {
                break;
            }
        }
    }

    Ok(())
}

// TODO:
// DISCLAIMER:
// Originally this was a tui library, but later I let codex convert it to wgpu
// almost all of the resulting slop is here and in backend

struct Render_state {
    surface: RenderSurface<'static>,
    window: Arc<Window>,
    valid: bool,
}

struct Window_app {
    title: String,
    context: RenderContext,
    renderer: Option<Renderer>,
    state: Option<Render_state>,
    cached_window: Option<Arc<Window>>,
    initializing: bool,
    scene: Option<Scene>,
    input: mpsc::UnboundedSender<Ui_input>,
    cursor: Point,
    modifiers: ModifiersState,
    scale_factor: f64,
    occluded: bool,
    error: Option<String>,
}

impl Window_app {
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: impl Into<String>) {
        self.error = Some(message.into());
        event_loop.exit();
    }

    fn send_size(&self) {
        let Some(state) = &self.state else {
            return;
        };
        let logical = state
            .window
            .inner_size()
            .to_logical::<f64>(self.scale_factor);
        let _ = self
            .input
            .send(Ui_input::Resize(Size::new(logical.width, logical.height)));
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        let Some(state) = &mut self.state else {
            return;
        };

        if size.width == 0 || size.height == 0 {
            state.valid = false;
        } else {
            self.context
                .resize_surface(&mut state.surface, size.width, size.height);
            state.valid = true;
            self.send_size();
        }
    }

    fn initialize_window(&mut self, event_loop: &ActiveEventLoop, window: Arc<Window>) {
        self.scale_factor = window.scale_factor();
        let size = window.inner_size();
        let valid = size.width > 0 && size.height > 0;
        let surface = match pollster::block_on(self.context.create_surface(
            window.clone(),
            size.width.max(1),
            size.height.max(1),
            wgpu::PresentMode::AutoVsync,
        )) {
            Ok(surface) => surface,
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("Vello GPU initialization is unsupported: {error}"),
                );
                return;
            }
        };
        let device_handle = &self.context.devices[surface.dev_id];
        let renderer = match Renderer::new(
            &device_handle.device,
            RendererOptions {
                antialiasing_support: [AaConfig::Area].into_iter().collect(),
                ..RendererOptions::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("Vello GPU renderer initialization failed: {error}"),
                );
                return;
            }
        };
        self.renderer = Some(renderer);
        self.state = Some(Render_state {
            surface,
            window,
            valid,
        });
        if valid {
            self.send_size();
        }
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        initial_size: Size,
        minimum_size: Size,
    ) {
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(self.title.clone())
                .with_resizable(true)
                .with_inner_size(LogicalSize::new(initial_size.width, initial_size.height))
                .with_min_inner_size(LogicalSize::new(minimum_size.width, minimum_size.height)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("Failed to create Vizual window: {error}"),
                );
                return;
            }
        };
        self.initialize_window(event_loop, window);
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            return;
        };
        if !state.valid || self.occluded {
            return;
        }
        let Some(logical_scene) = &self.scene else {
            return;
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let mut physical_scene = Scene::new();
        physical_scene.append(logical_scene, Some(Affine::scale(self.scale_factor)));
        let device_handle = &self.context.devices[state.surface.dev_id];
        let parameters = vello::RenderParams {
            base_color: palette::css::BLACK,
            width: state.surface.config.width,
            height: state.surface.config.height,
            antialiasing_method: AaConfig::Area,
        };
        if let Err(error) = renderer.render_to_texture(
            &device_handle.device,
            &device_handle.queue,
            &physical_scene,
            &state.surface.target_view,
            &parameters,
        ) {
            self.fail(
                event_loop,
                format!("Vello failed to render a frame: {error}"),
            );
            return;
        }

        let surface_texture = match state.surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                self.context.configure_surface(&state.surface);
                state.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Timeout => {
                state.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.context.configure_surface(&state.surface);
                state.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.fail(event_loop, "Vello surface validation failed");
                return;
            }
        };
        let mut encoder =
            device_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("vizual surface blit"),
                });
        state.surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &state.surface.target_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let _ = device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();
        let _ = device_handle.device.poll(wgpu::PollType::Poll);
    }
}

impl ApplicationHandler<User_event> for Window_app {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        if let Some(window) = self.cached_window.take() {
            self.initialize_window(event_loop, window);
        } else if !self.initializing {
            let monitor = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next());
            let maximum_size = monitor
                .map(|monitor| {
                    let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
                    Size::new(size.width, size.height)
                })
                .unwrap_or(DEFAULT_SCREEN_SIZE);
            self.initializing = self.input.send(Ui_input::Initialize(maximum_size)).is_ok();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &self.state else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                let _ = self.input.send(Ui_input::Event(Event::Close_requested));
            }
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                let size = state.window.inner_size();
                self.resize(size);
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if !occluded {
                    state.window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.to_logical::<f64>(self.scale_factor);
                self.cursor = Point::new(position.x, position.y);
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                let pointer = Pointer_event {
                    position: self.cursor,
                    button: map_pointer_button(button),
                };
                let _ = self.input.send(Ui_input::Event(Event::Pointer(pointer)));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        Wheel_delta::Lines(Point::new(f64::from(x), f64::from(y)))
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = delta.to_logical::<f64>(self.scale_factor);
                        Wheel_delta::Logical_pixels(Point::new(delta.x, delta.y))
                    }
                };
                let _ = self.input.send(Ui_input::Event(Event::Wheel(Wheel_event {
                    position: self.cursor,
                    delta,
                })));
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let modifiers = map_modifiers(self.modifiers);
                let key = Key_event {
                    code: map_key(&event.logical_key, modifiers),
                    modifiers,
                    text: event.text.as_ref().map(ToString::to_string),
                    repeat: event.repeat,
                };
                let _ = self.input.send(Ui_input::Event(Event::Key(key)));
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                let _ = self.input.send(Ui_input::Event(Event::Text(text)));
            }
            WindowEvent::RedrawRequested => self.render(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: User_event) {
        match event {
            User_event::Initialize(initial_size, minimum_size) => {
                self.initializing = false;
                self.create_window(event_loop, initial_size, minimum_size);
            }
            User_event::Minimum_size(size) => {
                let mut send_size = false;
                let requested_size = self.state.as_ref().and_then(|state| {
                    let current_size = state
                        .window
                        .inner_size()
                        .to_logical::<f64>(self.scale_factor);
                    let minimum_size = LogicalSize::new(size.width, size.height);
                    state.window.set_min_inner_size(Some(minimum_size));

                    match (
                        current_size.width < minimum_size.width,
                        current_size.height < minimum_size.height,
                    ) {
                        (false, false) => {
                            send_size = true;
                            None
                        }
                        _ => {
                            let requested_size = LogicalSize::new(
                                current_size.width.max(minimum_size.width),
                                current_size.height.max(minimum_size.height),
                            );
                            log_info(
                                0,
                                format_args!(
                                    "auto resize requested from {current_size:?} to {requested_size:?} for minimum size {minimum_size:?}"
                                ),
                            );
                            state.window.request_inner_size(requested_size)
                        }
                    }
                });
                match requested_size {
                    Some(size) => self.resize(size),
                    None if send_size => self.send_size(),
                    None => {}
                }
            }
            User_event::Scene(scene) => {
                self.scene = Some(scene);
                if let Some(state) = &self.state {
                    state.window.request_redraw();
                }
            }
            User_event::Exit => event_loop.exit(),
            User_event::Error(error) => self.fail(event_loop, error),
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.take() {
            self.cached_window = Some(state.window);
        }
    }
}

// TODO: these map functions are AI slop - sorry
fn map_modifiers(modifiers: ModifiersState) -> Modifiers {
    Modifiers {
        control: modifiers.control_key(),
        alt: modifiers.alt_key(),
        shift: modifiers.shift_key(),
        super_key: modifiers.super_key(),
    }
}

fn map_key(key: &Key, modifiers: Modifiers) -> Key_code {
    match key.as_ref() {
        Key::Character(" ") => Key_code::Space,
        Key::Character(text) => text
            .chars()
            .next()
            .map(Key_code::Character)
            .unwrap_or(Key_code::Unidentified),
        Key::Named(NamedKey::Enter) => Key_code::Enter,
        Key::Named(NamedKey::Escape) => Key_code::Escape,
        Key::Named(NamedKey::Tab) if modifiers.shift => Key_code::Back_tab,
        Key::Named(NamedKey::Tab) => Key_code::Tab,
        Key::Named(NamedKey::Backspace) => Key_code::Backspace,
        Key::Named(NamedKey::Delete) => Key_code::Delete,
        Key::Named(NamedKey::ArrowLeft) => Key_code::Arrow_left,
        Key::Named(NamedKey::ArrowRight) => Key_code::Arrow_right,
        Key::Named(NamedKey::ArrowUp) => Key_code::Arrow_up,
        Key::Named(NamedKey::ArrowDown) => Key_code::Arrow_down,
        Key::Named(NamedKey::PageUp) => Key_code::Page_up,
        Key::Named(NamedKey::PageDown) => Key_code::Page_down,
        Key::Named(NamedKey::Home) => Key_code::Home,
        Key::Named(NamedKey::End) => Key_code::End,
        Key::Named(NamedKey::Space) => Key_code::Space,
        _ => Key_code::Unidentified,
    }
}

fn map_pointer_button(button: MouseButton) -> Pointer_button {
    match button {
        MouseButton::Left => Pointer_button::Primary,
        MouseButton::Right => Pointer_button::Secondary,
        MouseButton::Middle => Pointer_button::Middle,
        MouseButton::Back => Pointer_button::Other(4),
        MouseButton::Forward => Pointer_button::Other(5),
        MouseButton::Other(value) => Pointer_button::Other(value),
    }
}

pub fn run<T: Renderable>(
    title: impl Into<String>,
    root: Shared_renderable<T>,
    theme: State<Theme>,
    render_signal: mpsc::UnboundedReceiver<()>,
) -> Result<()> {
    let root = Root::new(root, theme).into_shared();
    let runtime = tokio::runtime::Handle::try_current()
        .wrap_err("vizual::run requires an active Tokio runtime")?;
    let event_loop = EventLoop::<User_event>::with_user_event()
        .build()
        .wrap_err("Failed to initialize the Winit event loop")?;
    let proxy = event_loop.create_proxy();
    let error_proxy = proxy.clone();
    let (input_sender, input_receiver) = mpsc::unbounded_channel();
    let ui_task = runtime.spawn(async move {
        if let Err(error) = ui_loop(root, render_signal, input_receiver, proxy).await {
            let _ = error_proxy.send_event(User_event::Error(format!("{error:?}")));
        }
    });
    let mut app = Window_app {
        title: title.into(),
        context: RenderContext::new(),
        renderer: None,
        state: None,
        cached_window: None,
        initializing: false,
        scene: None,
        input: input_sender,
        cursor: Point::default(),
        modifiers: ModifiersState::default(),
        scale_factor: 1.0,
        occluded: false,
        error: None,
    };

    let event_result = event_loop
        .run_app(&mut app)
        .wrap_err("Winit event loop failed");
    ui_task.abort();
    event_result?;
    if let Some(error) = app.error {
        return Err(eyre!(error));
    }
    Ok(())
}
