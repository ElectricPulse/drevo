#![warn(rustdoc::broken_intra_doc_links)]
//! An async, solver-driven desktop UI framework.
//!
//! Interfaces are [`widget::WidgetTrait`] trees laid out through solver
//! constraints and painted with Vello.

pub mod component;
mod config;
pub mod event;
pub mod focus;
pub mod geometry;
pub mod graphics;
pub mod handlers;
pub mod layouter;
pub use layouter::priorities;
pub mod log;
pub mod macros;
pub mod render_manager;
pub mod slot;
pub mod state;
pub mod style;
pub mod sync;
pub mod theme;
pub mod unicode;
pub mod utils;
pub mod widget;

extern crate self as drevo;

use std::{
    fs::OpenOptions,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use async_recursion::async_recursion;
use color_eyre::eyre::{ContextCompat, Result, WrapErr, eyre};
use simplelog::{
    ColorChoice, CombinedLogger, Config as LogConfig, LevelFilter, SharedLogger, TermLogger,
    TerminalMode, WriteLogger,
};
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
    window::{Theme as WindowTheme, WindowId},
};

pub use winit::window::Window;

use component::{ChildReference, SharedComponent, context::ComponentContext};
use config::{
    COPY_SOLUTION_TO_FORMULA, DEFAULT_SCREEN_SIZE, MINIMUM_WINDOW_SIZE, REQUEST_DEBOUNCE,
    SCROLL_SENSITIVITY,
};
use event::{Event, KeyCode, KeyEvent, Modifiers, PointerButton, PointerEvent, WheelEvent};
use focus::{Focus, FocusSearchDirection};
use geometry::{Point, Size};
use graphics::{scene::Scene as GraphicsScene, text::TextContext};
use layouter::{Problem, Solution, hitbox::Hitbox, variables::Variables};
use log::{log_duration, log_info};
use render_manager::{RenderManager, RenderReceiver};
use slot::ComponentSlot;
use state::Store;
use theme::{SystemTheme, Theme};
use widget::{SharedWidget, WidgetTrait, widgets::root::Root};

pub fn init_logging(path: Option<impl AsRef<Path>>) -> Result<()> {
    let logger: Box<dyn SharedLogger> = match path {
        Some(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_ref())
                .wrap_err_with(|| format!("Failed to open log file {}", path.as_ref().display()))?;

            WriteLogger::new(LevelFilter::Info, LogConfig::default(), file)
        }
        None => TermLogger::new(
            LevelFilter::Info,
            LogConfig::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
    };

    CombinedLogger::init(vec![logger])
        .map_err(|error| eyre!("Failed to initialize logging: {error}"))?;

    Ok(())
}

#[derive(Clone)]
pub struct DrevoMsg {
    pub(crate) propagate: bool,
    pub(crate) command: DrevoCommand,
}

impl DrevoMsg {
    pub fn new(command: DrevoCommand) -> Result<Self> {
        Ok(Self {
            propagate: false,
            command,
        })
    }

    pub fn new_propagated(command: DrevoCommand) -> Result<Self> {
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
            command: DrevoCommand::None,
        }
    }

    pub(crate) fn has_command(&self) -> bool {
        !matches!(self.command, DrevoCommand::None)
    }

    pub(crate) fn join(&mut self, message: DrevoMsg) {
        self.command = self.command.clone().join(message.command);
        self.propagate = self.propagate && message.propagate;
    }
}

#[derive(Clone)]
pub enum DrevoCommand {
    None,
    Resolve,
    Layout,
    Render,
    Focus(ChildReference),
    Quit,
}

impl DrevoCommand {
    fn join(self, command: Self) -> Self {
        match (self, command) {
            (Self::Quit, _) | (_, Self::Quit) => Self::Quit,
            (Self::Focus(focus), _) | (_, Self::Focus(focus)) => Self::Focus(focus),
            (Self::Layout, _) | (_, Self::Layout) => Self::Layout,
            (Self::Resolve, _) | (_, Self::Resolve) => Self::Resolve,
            (Self::Render, _) | (_, Self::Render) => Self::Render,
            (Self::None, Self::None) => Self::None,
        }
    }
}

static SIGNAL_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Signal {
    pub(crate) id: u64,
    sender: mpsc::UnboundedSender<RenderRequest>,
    request: RenderRequest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RenderRequest {
    Rerender,
    Layout,
}

impl Signal {
    pub(crate) fn new(
        sender: mpsc::UnboundedSender<RenderRequest>,
        request: RenderRequest,
    ) -> Self {
        Self {
            id: SIGNAL_ID.fetch_add(1, Ordering::Relaxed),
            sender,
            request,
        }
    }

    pub fn send(&self) {
        let _ = self.sender.send(self.request);
    }
}

pub fn check_quit_event(event: &KeyEvent) -> bool {
    matches!(event.code, KeyCode::Character('c' | 'C')) && event.modifiers.control
}

struct AppProblem {
    root: SharedComponent,
    root_hitbox: Hitbox,
    variables: Arc<Variables>,
    layout: Signal,
    window: Option<Arc<Window>>,
}

impl AppProblem {
    async fn new<T: WidgetTrait>(
        root: SharedWidget<T>,
        root_slot: &mut ComponentSlot,
        variables: Arc<Variables>,
        rerender: Signal,
    ) -> Result<Self> {
        let component_context = ComponentContext::new(Arc::clone(&variables));
        let root = root_slot.set(root, component_context.clone()).await?;
        let root_hitbox = root.get_hitbox().await?;

        Ok(Self {
            root,
            root_hitbox,
            variables,
            layout: rerender,
            window: None,
        })
    }

    async fn layout(
        &mut self,
        layout: Signal,
        theme: Store<Theme>,
        focus: &Focus,
        text_context: &mut TextContext,
    ) -> Result<()> {
        self.layout = layout.clone();
        let focused_path = focus.affected_path(layout.clone()).await?;
        let root = self.root.clone();
        let children = self
            .root
            .layout(
                layout.clone(),
                theme.clone(),
                &focused_path,
                None,
                self.root_hitbox.clone(),
                ComponentContext::new(Arc::clone(&self.variables)),
                text_context,
                &root,
            )
            .await?;
        self.root
            .layout_children(
                layout,
                theme,
                &focused_path,
                children,
                ComponentContext::new(Arc::clone(&self.variables)),
                text_context,
                &root,
            )
            .await?;

        Ok(())
    }

    async fn solve(&self, size: Size) -> Result<Solution> {
        let component_tree = self.root.component_tree().await?;
        let mut problem = Problem::new(Arc::clone(&self.variables));
        self.root.add_formulas(&mut problem).await?;
        let solution = problem
            .solve(self.root_hitbox.clone(), size, &component_tree)
            .await?;
        if COPY_SOLUTION_TO_FORMULA {
            let (loaded_variables, loaded_constraints) = problem.warm_start_counts();
            let (stored_variables, stored_constraints) =
                self.root.store_solution(&solution).await?;
            log_info(
                2,
                format_args!(
                    "layout solution copy: Solution -> Formula: {stored_variables} variables, {stored_constraints} constraints; Formula -> Problem: {loaded_variables} variables, {loaded_constraints} constraints",
                ),
            );
        }
        Ok(solution)
    }

    async fn render(
        &mut self,
        rerender: crate::Signal,
        theme: Store<Theme>,
        focus: &Focus,
        solution: &Solution,
        text_context: &mut TextContext,
    ) -> Result<Scene> {
        let focused_path = focus.focused_path().await?;
        let context = component::RenderContext {
            focused_path: &focused_path,
            solution,
        };
        let mut scene = Scene::new();
        let mut graphics_scene = GraphicsScene::new(&mut scene);
        self.root
            .render(rerender, theme, &mut graphics_scene, text_context, &context)
            .await?;
        Ok(scene)
    }

    #[async_recursion]
    async fn handle_pointer_press(
        &mut self,
        node: SharedComponent,
        position: Point,
        event: &Event,
        solution: &Solution,
        focus: &Focus,
    ) -> Result<Option<DrevoCommand>> {
        let (hits, children) = {
            let node_lock = node.lock().await?;
            (
                node_lock.hitbox.get_resolved(solution).contains(position),
                node_lock.children.clone(),
            )
        };

        if !hits {
            return Ok(None);
        }

        for child in children.iter().rev() {
            let message = self
                .handle_pointer_press(child.clone(), position, event, solution, focus)
                .await?;

            if message.is_some() {
                return Ok(message);
            }
        }

        if !node.lock().await?.focusable {
            return Ok(None);
        }

        focus.set(&node).await?;
        // It's easier to use drill_event even for a focus event while drill_event already needs to exist to pass on keyboard events
        let cmd = self.drill_event(event, focus).await?;

        Ok(Some(cmd))
    }

    async fn drill_event(&mut self, event: &Event, focus: &Focus) -> Result<DrevoCommand> {
        let Some(mut current_node) = focus.upgrade().await? else {
            return Ok(DrevoCommand::None);
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
            current_node = SharedComponent::new(parent);
        }

        if !self.root.compare(&current_node) {
            return Err(eyre!("The last found parent should be the root"));
        }

        let window = self.window.clone();
        let mut message = DrevoMsg::none()?;
        for node in &nodes {
            let new_message = {
                let mut node = node.lock().await?;
                let message = node
                    .widget
                    .forward_event(event, self.layout.clone(), window.clone())
                    .await?;
                message
            };
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
        direction: FocusSearchDirection,
        focus: &Focus,
    ) -> Result<bool> {
        let start = focus.upgrade().await?.unwrap_or_else(|| self.root.clone());
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
        node: SharedComponent,
        mut skip_count: usize,
        direction: FocusSearchDirection,
        up: bool,
        focus: &Focus,
    ) -> Result<(bool, usize)> {
        let lock = node.lock().await?;

        if !up && lock.focusable && !focus.compare(&node).await? {
            if skip_count == 0 {
                focus.set(&node).await?;
                return Ok((true, skip_count));
            }
            skip_count -= 1;
        }

        let children_count = lock.children.len();
        let child_iterator: Box<dyn Iterator<Item = usize> + Send> = match (origin, direction) {
            (Some(origin), FocusSearchDirection::Right) => Box::new(origin + 1..children_count),
            (Some(origin), FocusSearchDirection::Left) => Box::new((0..origin).rev()),
            (None, FocusSearchDirection::Right) => Box::new(0..children_count),
            (None, FocusSearchDirection::Left) => Box::new((0..children_count).rev()),
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

        if lock.focusable && !focus.compare(&node).await? {
            if skip_count == 0 {
                focus.set(&node).await?;
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
            SharedComponent::new(parent),
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
        focus: &Focus,
    ) -> Result<DrevoCommand> {
        if !matches!(event, Event::Pointer(_)) {
            let command = self.drill_event(event, focus).await?;
            if !matches!(command, DrevoCommand::None) {
                return Ok(command);
            }
        }

        match event {
            Event::Key(key) => match key.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    let direction = match key.code {
                        KeyCode::Tab => FocusSearchDirection::Right,
                        _ => FocusSearchDirection::Left,
                    };
                    let _ = self.move_focus(0, direction, focus).await?;
                    Ok(DrevoCommand::None)
                }
                KeyCode::Escape => {
                    focus.reset().await?;
                    Ok(DrevoCommand::None)
                }
                _ if check_quit_event(key) => Ok(DrevoCommand::Quit),
                _ => Ok(DrevoCommand::None),
            },
            Event::Pointer(pointer) => {
                let initial_focus = focus.clone();
                let command = self
                    .handle_pointer_press(
                        self.root.clone(),
                        pointer.position,
                        event,
                        solution,
                        focus,
                    )
                    .await?;
                if let Some(focused) = initial_focus.upgrade().await? {
                    if focus.compare(&focused).await?
                        && !self
                            .component_hit(&focused, pointer.position, solution)
                            .await?
                    {
                        focus.reset().await?;
                        return Ok(DrevoCommand::None);
                    }
                }

                Ok(command.unwrap_or(DrevoCommand::None))
            }
            Event::CloseRequested => Ok(DrevoCommand::Quit),
            Event::Wheel(_) | Event::Text(_) => Ok(DrevoCommand::None),
        }
    }

    async fn component_hit(
        &self,
        node: &SharedComponent,
        position: Point,
        solution: &Solution,
    ) -> Result<bool> {
        let (hits, children) = {
            let node_lock = node.lock().await?;
            (
                node_lock.hitbox.get_resolved(solution).contains(position),
                node_lock.children.clone(),
            )
        };
        if hits {
            return Ok(true);
        }
        for child in children {
            if Box::pin(self.component_hit(&child, position, solution)).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

enum UiInput {
    Initialize(Size),
    Window(Arc<Window>),
    Event(Event),
    Resize(Size),
    Rerender,
    Layout,
    SystemTheme(SystemTheme),
}

// TODO: This message-passing layer is probably unnecessary.
enum UserEvent {
    Initialize(Size),
    Scene(Scene),
    Exit,
    Error(String),
}

async fn layout_problem<T: WidgetTrait>(
    root: SharedWidget<T>,
    rerender: Signal,
    layout: Signal,
    theme: Store<Theme>,
    focus: &Focus,
    root_slot: &mut ComponentSlot,
    text_context: &mut TextContext,
    variables: Arc<Variables>,
    window: Option<Arc<Window>>,
) -> Result<AppProblem> {
    let mut problem = AppProblem::new(root, root_slot, variables, rerender.clone()).await?;
    problem.window = window;
    log_duration(0, "component layout()", true, || {
        problem.layout(layout, theme, focus, text_context)
    })
    .await?;
    Ok(problem)
}

async fn ui_loop<T: WidgetTrait>(
    root: SharedWidget<T>,
    rerender: Signal,
    layout: Signal,
    theme: Store<Theme>,
    mut render_receiver: RenderReceiver,
    mut input_receiver: mpsc::UnboundedReceiver<UiInput>,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<()> {
    let focus = Focus::new();
    let mut root_slot = ComponentSlot::new();
    let mut text_context = TextContext::new();
    let variables = Arc::new(Variables::new());
    let mut app_problem: Option<AppProblem> = None;
    let mut window: Option<Arc<Window>> = None;
    let mut solution = None;
    let mut window_size = None;
    let mut render_open = true;
    let mut buffered_input = None;

    loop {
        let input = match buffered_input.take() {
            Some(input) => Some(input),
            None => {
                tokio::select! {
                    input = input_receiver.recv() => input,
                    render_request = render_receiver.0.recv(), if render_open => {
                        match render_request {
                            Some(first) => {
                                let mut needs_layout = matches!(first, RenderRequest::Layout);
                                let deadline = tokio::time::Instant::from_std(Instant::now() + REQUEST_DEBOUNCE);
                                loop {
                                    tokio::select! {
                                        request = render_receiver.0.recv() => match request {
                                            Some(RenderRequest::Layout) => needs_layout = true,
                                            Some(RenderRequest::Rerender) => {},
                                            None => { render_open = false; break; }
                                        },
                                        _ = tokio::time::sleep_until(deadline) => break,
                                    }
                                }
                                Some(if needs_layout { UiInput::Layout } else { UiInput::Rerender })
                            }
                            None => {
                                render_open = false;
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
            UiInput::Resize(mut latest_size) => {
                loop {
                    match input_receiver.try_recv() {
                        Ok(UiInput::Resize(size)) => latest_size = size,
                        Ok(input) => {
                            buffered_input = Some(input);
                            break;
                        }
                        Err(_) => break,
                    }
                }

                UiInput::Resize(latest_size)
            }
            input => input,
        };

        let mut relayout = false;
        let mut command = match input {
            UiInput::Window(value) => {
                window = Some(Arc::clone(&value));
                if let Some(problem) = &mut app_problem {
                    problem.window = Some(value);
                }
                continue;
            }
            UiInput::SystemTheme(system) => {
                let updated = theme.read().await?.set_system(system);
                theme.set(updated).await?;
                continue;
            }
            UiInput::Initialize(maximum_size) => {
                let initial_size = Size::new(
                    DEFAULT_SCREEN_SIZE.width.min(maximum_size.width),
                    DEFAULT_SCREEN_SIZE.height.min(maximum_size.height),
                );
                if proxy
                    .send_event(UserEvent::Initialize(initial_size))
                    .is_err()
                {
                    break;
                }
                continue;
            }
            UiInput::Resize(size) => {
                window_size = Some(size);
                relayout = app_problem.is_none();
                DrevoCommand::Resolve
            }
            UiInput::Rerender => DrevoCommand::Render,
            UiInput::Layout => DrevoCommand::Layout,
            UiInput::Event(event) => match (&mut app_problem, &solution) {
                (Some(problem), Some(solution)) => {
                    problem.handle_event(&event, solution, &focus).await?
                }
                _ if matches!(event, Event::CloseRequested) => DrevoCommand::Quit,
                _ => DrevoCommand::None,
            },
        };

        if matches!(command, DrevoCommand::Quit) {
            let _ = proxy.send_event(UserEvent::Exit);
            break;
        }

        let Some(size) = window_size else {
            continue;
        };

        if let DrevoCommand::Focus(reference) = &command {
            focus.set_with_reference(reference).await?;
            command = DrevoCommand::None;
        }

        if relayout || matches!(command, DrevoCommand::Layout) {
            let problem = layout_problem(
                root.clone(),
                rerender.clone(),
                layout.clone(),
                theme.clone(),
                &focus,
                &mut root_slot,
                &mut text_context,
                Arc::clone(&variables),
                window.clone(),
            )
            .await?;
            solution = Some(problem.solve(size).await?);
            app_problem = Some(problem);
            command = DrevoCommand::Render;
        } else if matches!(command, DrevoCommand::Resolve) {
            if let Some(problem) = &app_problem {
                solution = Some(problem.solve(size).await?);
                command = DrevoCommand::Render;
            }
        }

        if matches!(command, DrevoCommand::Render)
            && let (Some(problem), Some(solution)) = (&mut app_problem, &solution)
        {
            let scene = log_duration(0, "component render()", false, || {
                problem.render(
                    rerender.clone(),
                    theme.clone(),
                    &focus,
                    solution,
                    &mut text_context,
                )
            })
            .await?;
            println!();
            if proxy.send_event(UserEvent::Scene(scene)).is_err() {
                break;
            }
        }
    }

    Ok(())
}

struct RenderState {
    surface: RenderSurface<'static>,
    window: Arc<Window>,
    valid: bool,
}

struct WindowApp {
    title: String,
    context: RenderContext,
    renderer: Option<Renderer>,
    state: Option<RenderState>,
    cached_window: Option<Arc<Window>>,
    initializing: bool,
    scene: Option<Scene>,
    input: mpsc::UnboundedSender<UiInput>,
    cursor: Point,
    modifiers: ModifiersState,
    scale_factor: f64,
    occluded: bool,
    error: Option<String>,
}

impl WindowApp {
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
            .send(UiInput::Resize(Size::new(logical.width, logical.height)));
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
        if let Some(system) = window.theme().map(map_system_theme) {
            let _ = self.input.send(UiInput::SystemTheme(system));
        }
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
        self.state = Some(RenderState {
            surface,
            window: Arc::clone(&window),
            valid,
        });
        let _ = self.input.send(UiInput::Window(window));
        if valid {
            self.send_size();
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop, initial_size: Size) {
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(self.title.clone())
                // TODO: have to implement custom resizing after that
                //.with_decorations(false)
                //.with_resizable(true)
                .with_inner_size(LogicalSize::new(initial_size.width, initial_size.height))
                .with_min_inner_size(LogicalSize::new(
                    MINIMUM_WINDOW_SIZE.width,
                    MINIMUM_WINDOW_SIZE.height,
                )),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(
                    event_loop,
                    format!("Failed to create Drevo window: {error}"),
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

        let (surface_texture, reconfigure_after_present) =
            match state.surface.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(texture) => (texture, false),
                wgpu::CurrentSurfaceTexture::Suboptimal(texture) => (texture, true),
                wgpu::CurrentSurfaceTexture::Outdated => {
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
                    label: Some("drevo surface blit"),
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
        if reconfigure_after_present {
            self.context.configure_surface(&state.surface);
            state.window.request_redraw();
        }
    }
}

impl ApplicationHandler<UserEvent> for WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let system = event_loop
            .system_theme()
            .map(map_system_theme)
            .unwrap_or(SystemTheme::Dark);
        let _ = self.input.send(UiInput::SystemTheme(system));

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
            self.initializing = self.input.send(UiInput::Initialize(maximum_size)).is_ok();
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
                let _ = self.input.send(UiInput::Event(Event::CloseRequested));
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
            WindowEvent::ThemeChanged(theme) => {
                let system = map_system_theme(theme);
                let _ = self.input.send(UiInput::SystemTheme(system));
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
                let pointer = PointerEvent {
                    position: self.cursor,
                    button: map_pointer_button(button),
                };
                let _ = self.input.send(UiInput::Event(Event::Pointer(pointer)));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Point::new(
                        f64::from(x) * SCROLL_SENSITIVITY,
                        f64::from(y) * SCROLL_SENSITIVITY,
                    ),
                    MouseScrollDelta::PixelDelta(delta) => {
                        let delta = delta.to_logical::<f64>(self.scale_factor);
                        Point::new(delta.x, delta.y)
                    }
                };
                let _ = self.input.send(UiInput::Event(Event::Wheel(WheelEvent {
                    position: self.cursor,
                    delta,
                    modifiers: map_modifiers(self.modifiers),
                })));
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let modifiers = map_modifiers(self.modifiers);
                let key = KeyEvent {
                    code: map_key(&event.logical_key, modifiers),
                    modifiers,
                    text: event.text.as_ref().map(ToString::to_string),
                    repeat: event.repeat,
                };
                let _ = self.input.send(UiInput::Event(Event::Key(key)));
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                let _ = self.input.send(UiInput::Event(Event::Text(text)));
            }
            WindowEvent::RedrawRequested => self.render(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Initialize(initial_size) => {
                self.initializing = false;
                self.create_window(event_loop, initial_size);
            }
            UserEvent::Scene(scene) => {
                self.scene = Some(scene);
                if let Some(state) = &self.state {
                    state.window.request_redraw();
                }
            }
            UserEvent::Exit => event_loop.exit(),
            UserEvent::Error(error) => self.fail(event_loop, error),
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

fn map_key(key: &Key, modifiers: Modifiers) -> KeyCode {
    match key.as_ref() {
        Key::Character(" ") => KeyCode::Space,
        Key::Character(text) => text
            .chars()
            .next()
            .map(KeyCode::Character)
            .unwrap_or(KeyCode::Unidentified),
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Escape) => KeyCode::Escape,
        Key::Named(NamedKey::Tab) if modifiers.shift => KeyCode::BackTab,
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => KeyCode::ArrowRight,
        Key::Named(NamedKey::ArrowUp) => KeyCode::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => KeyCode::ArrowDown,
        Key::Named(NamedKey::PageUp) => KeyCode::PageUp,
        Key::Named(NamedKey::PageDown) => KeyCode::PageDown,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Named(NamedKey::Space) => KeyCode::Space,
        _ => KeyCode::Unidentified,
    }
}

fn map_pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Other(4),
        MouseButton::Forward => PointerButton::Other(5),
        MouseButton::Other(value) => PointerButton::Other(value),
    }
}

fn map_system_theme(theme: WindowTheme) -> SystemTheme {
    if matches!(theme, WindowTheme::Dark) {
        SystemTheme::Dark
    } else {
        SystemTheme::Light
    }
}

/// Runs a Drevo application on the calling thread.
///
/// A Tokio runtime must already be active. Winit owns the calling thread until
/// the window closes; widget tasks continue on the runtime. Drevo creates the
/// render manager used by the root widget.
pub fn run<T: WidgetTrait>(title: impl Into<String>, root: T) -> Result<()> {
    let RenderManager {
        rerender,
        layout,
        receiver,
    } = RenderManager::new();
    let theme = Store::new(Theme::default());
    let root = Root::new(root.into_shared()).into_shared();
    let runtime = tokio::runtime::Handle::try_current()
        .wrap_err("drevo::run requires an active Tokio runtime")?;
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .wrap_err("Failed to initialize the Winit event loop")?;
    let proxy = event_loop.create_proxy();
    let error_proxy = proxy.clone();
    let (input_sender, input_receiver) = mpsc::unbounded_channel();
    let ui_theme = theme.clone();
    let ui_task = runtime.spawn(async move {
        if let Err(error) = ui_loop(
            root,
            rerender,
            layout,
            ui_theme,
            receiver,
            input_receiver,
            proxy,
        )
        .await
        {
            let _ = error_proxy.send_event(UserEvent::Error(format!("{error:?}")));
        }
    });
    let mut app = WindowApp {
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

#[cfg(test)]
mod tests;
