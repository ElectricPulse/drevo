#![feature(async_fn_track_caller)]
#![warn(rustdoc::broken_intra_doc_links)]
//! An async, solver-driven desktop UI framework.
//!
//! Interfaces are [`widget::Widget_trait`] trees laid out through solver
//! constraints and painted with Vello. Vizual currently requires nightly Rust.

pub mod component;
mod config;
pub mod event;
pub mod focus;
pub mod geometry;
pub mod graphics;
pub mod handlers;
pub mod layouter;
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

extern crate self as vizual;

use std::{
    collections::HashSet,
    fs::OpenOptions,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use async_recursion::async_recursion;
use color_eyre::eyre::{ContextCompat, Result, WrapErr, eyre};
use simplelog::{
    ColorChoice, CombinedLogger, Config as Log_config, LevelFilter, SharedLogger, TermLogger,
    TerminalMode, WriteLogger,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
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
    window::{Theme as Window_theme, Window, WindowId},
};

use component::{Child_reference, Shared_component, context::Component_context};
use config::{DEFAULT_SCREEN_SIZE, MINIMUM_WINDOW_SIZE};
use event::{
    Event, Key_code, Key_event, Modifiers, Pointer_button, Pointer_event, Wheel_delta, Wheel_event,
};
use focus::{Focus, Focus_search_direction};
use geometry::{Point, Size};
use graphics::{scene::Scene as Graphics_scene, text::Text_context};
use layouter::{Formula, Problem, Solution, hitbox::Hitbox, variables::Variables};
use log::{log_duration, log_info};
use render_manager::{Render_manager, Render_receiver};
use slot::Component_slot;
use state::Store;
use sync::Mutex;
use theme::{System_theme, Theme};
use widget::{Shared_widget, Widget_trait, widgets::root::Root};

pub fn init_logging(path: Option<impl AsRef<Path>>) -> Result<()> {
    let logger: Box<dyn SharedLogger> = match path {
        Some(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_ref())
                .wrap_err_with(|| format!("Failed to open log file {}", path.as_ref().display()))?;

            WriteLogger::new(LevelFilter::Info, Log_config::default(), file)
        }
        None => TermLogger::new(
            LevelFilter::Info,
            Log_config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
    };

    CombinedLogger::init(vec![logger])
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

    pub(crate) fn join(&mut self, message: Vizual_msg) {
        self.command = self.command.clone().join(message.command);
        self.propagate = self.propagate && message.propagate;
    }
}

#[derive(Clone)]
pub enum Vizual_command {
    None,
    Resolve,
    Render,
    Focus(Child_reference),
    Quit,
}

impl Vizual_command {
    fn join(self, command: Self) -> Self {
        match (self, command) {
            (Self::Quit, _) | (_, Self::Quit) => Self::Quit,
            (Self::Focus(focus), _) | (_, Self::Focus(focus)) => Self::Focus(focus),
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
    target: Option<component::Id>,
    sender: mpsc::UnboundedSender<Render_request>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Render_request {
    Rerender,
    Layout(component::Id),
}

impl Signal {
    pub(crate) fn new(sender: mpsc::UnboundedSender<Render_request>) -> Self {
        Self {
            id: SIGNAL_ID.fetch_add(1, Ordering::Relaxed),
            target: None,
            sender,
        }
    }

    pub(crate) fn for_component(&self, id: component::Id) -> Self {
        Self {
            id,
            target: Some(id),
            sender: self.sender.clone(),
        }
    }

    pub fn send(&self) {
        let request = match self.target {
            Some(component) => Render_request::Layout(component),
            None => Render_request::Rerender,
        };
        let _ = self.sender.send(request);
    }
}

pub fn check_quit_event(event: &Key_event) -> bool {
    matches!(event.code, Key_code::Character('c' | 'C')) && event.modifiers.control
}

struct App_problem {
    root: Shared_component,
    root_hitbox: Hitbox,
    variables: Arc<Variables>,
    rerender: Signal,
}

impl App_problem {
    async fn new<T: Widget_trait>(
        root: Shared_widget<T>,
        root_slot: &mut Component_slot,
        variables: Arc<Variables>,
        rerender: Signal,
    ) -> Result<Self> {
        let component_context =
            Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(&variables)))));
        let root = root_slot.set(root, component_context.clone()).await?;
        let root_hitbox = root.get_hitbox().await?;

        Ok(Self {
            root,
            root_hitbox,
            variables,
            rerender,
        })
    }

    async fn layout(
        &mut self,
        rerender: Signal,
        theme: Store<Theme>,
        focus: &Focus,
        text_context: &mut Text_context,
    ) -> Result<()> {
        let focused_path = focus.focused_path().await?;
        let root = self.root.clone();
        let children = self
            .root
            .layout(
                rerender.clone(),
                theme.clone(),
                &focused_path,
                None,
                self.root_hitbox.clone(),
                Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(
                    &self.variables,
                ))))),
                text_context,
                &root,
            )
            .await?;
        self.root
            .layout_children(
                rerender,
                theme,
                &focused_path,
                children,
                Component_context::new(Arc::new(Mutex::new(Formula::new(Arc::clone(
                    &self.variables,
                ))))),
                text_context,
                &root,
            )
            .await?;

        Ok(())
    }

    async fn solve(&self, size: Size) -> Result<Solution> {
        let component_tree = self.root.component_tree().await?;
        let mut problem = Problem::new(Arc::clone(&self.variables));
        self.root.add_cached_formulas(&mut problem).await?;
        problem
            .solve(self.root_hitbox.clone(), size, &component_tree)
            .await
    }

    async fn render(
        &mut self,
        rerender: crate::Signal,
        theme: Store<Theme>,
        focus: &Focus,
        solution: &Solution,
        text_context: &mut Text_context,
    ) -> Result<Scene> {
        let focused_path = focus.focused_path().await?;
        let context = component::Render_context {
            focused_path: &focused_path,
            solution,
        };
        let mut scene = Scene::new();
        let mut graphics_scene = Graphics_scene::new(&mut scene);
        self.root
            .render(rerender, theme, &mut graphics_scene, text_context, &context)
            .await?;
        Ok(scene)
    }

    async fn signal_component(&self, component: &Shared_component) -> Result<()> {
        let id = component.lock().await?.id;
        self.rerender.for_component(id).send();
        Ok(())
    }

    #[async_recursion]
    async fn handle_pointer_press(
        &mut self,
        node: Shared_component,
        position: Point,
        event: &Event,
        solution: &Solution,
        focus: &mut Focus,
    ) -> Result<Option<Vizual_command>> {
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

        focus.set(&node);
        let cmd = self.drill_event(event, focus).await?;

        self.signal_component(&node).await?;

        Ok(Some(cmd))
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
            current_node = Shared_component::new(parent);
        }

        if !self.root.compare(&current_node) {
            return Err(eyre!("The last found parent should be the root"));
        }

        let mut message = Vizual_msg::none()?;
        for node in &nodes {
            let new_message = {
                let mut node = node.lock().await?;
                let id = node.id;
                let message = node
                    .widget
                    .forward_event(event, self.rerender.for_component(id))
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
        node: Shared_component,
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
            Shared_component::new(parent),
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
        if !matches!(event, Event::Pointer(_)) {
            let command = self.drill_event(event, focus).await?;
            if !matches!(command, Vizual_command::None) {
                return Ok(command);
            }
        }

        match event {
            Event::Key(key) => match key.code {
                Key_code::Tab | Key_code::Back_tab => {
                    let direction = match key.code {
                        Key_code::Tab => Focus_search_direction::Right,
                        _ => Focus_search_direction::Left,
                    };
                    let _ = self.move_focus(0, direction, focus).await?;
                    if let Some(node) = focus.upgrade() {
                        self.signal_component(&node).await?;
                    }
                    Ok(Vizual_command::None)
                }
                Key_code::Escape => {
                    if let Some(node) = focus.upgrade() {
                        self.signal_component(&node).await?;
                    }
                    focus.reset();
                    Ok(Vizual_command::None)
                }
                _ if check_quit_event(key) => Ok(Vizual_command::Quit),
                _ => Ok(Vizual_command::None),
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
                if let Some(focused) = initial_focus.upgrade() {
                    if focus.compare(&focused)
                        && !self
                            .component_hit(&focused, pointer.position, solution)
                            .await?
                    {
                        focus.reset();
                        self.signal_component(&focused).await?;
                        return Ok(Vizual_command::None);
                    }
                }

                Ok(command.unwrap_or(Vizual_command::None))
            }
            Event::Close_requested => Ok(Vizual_command::Quit),
            Event::Wheel(_) | Event::Text(_) => Ok(Vizual_command::None),
        }
    }

    async fn component_hit(
        &self,
        node: &Shared_component,
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

enum Ui_input {
    Initialize(Size),
    Event(Event),
    Resize(Size),
    Rerender,
    Layout(HashSet<component::Id>),
    System_theme(System_theme),
}

// TODO: This message-passing layer is probably unnecessary.
enum User_event {
    Initialize(Size),
    Scene(Scene),
    Exit,
    Error(String),
}

async fn layout_problem<T: Widget_trait>(
    root: Shared_widget<T>,
    rerender: Signal,
    theme: Store<Theme>,
    focus: &Focus,
    root_slot: &mut Component_slot,
    text_context: &mut Text_context,
    variables: Arc<Variables>,
) -> Result<App_problem> {
    let mut problem = App_problem::new(root, root_slot, variables, rerender.clone()).await?;
    log_duration(0, "app problem layout", || {
        problem.layout(rerender, theme, focus, text_context)
    })
    .await?;
    Ok(problem)
}

async fn ui_loop<T: Widget_trait>(
    root: Shared_widget<T>,
    rerender: Signal,
    theme: Store<Theme>,
    mut render_receiver: Render_receiver,
    mut input_receiver: mpsc::UnboundedReceiver<Ui_input>,
    proxy: EventLoopProxy<User_event>,
) -> Result<()> {
    let mut focus = Focus::new();
    let mut root_slot = Component_slot::new();
    let mut text_context = Text_context::new();
    let variables = Arc::new(Variables::new());
    let mut app_problem: Option<App_problem> = None;
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
                                let mut layouts = HashSet::new();
                                let mut rerender_requested = false;
                                let mut add = |request| match request {
                                    Render_request::Rerender => rerender_requested = true,
                                    Render_request::Layout(id) => { let _ = layouts.insert(id); }
                                };
                                add(first);
                                let deadline = tokio::time::Instant::from_std(Instant::now() + Duration::from_millis(10));
                                loop {
                                    tokio::select! {
                                        request = render_receiver.0.recv() => match request {
                                            Some(request) => add(request),
                                            None => { render_open = false; break; }
                                        },
                                        _ = tokio::time::sleep_until(deadline) => break,
                                    }
                                }
                                match layouts.is_empty() {
                                    true if rerender_requested => Some(Ui_input::Rerender),
                                    true => continue,
                                    false => Some(Ui_input::Layout(layouts)),
                                }
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

        let mut relayout = false;
        let mut command = match input {
            Ui_input::System_theme(system) => {
                let updated = theme.read().await?.set_system(system);
                theme.set(updated).await?;
                continue;
            }
            Ui_input::Initialize(maximum_size) => {
                let initial_size = Size::new(
                    DEFAULT_SCREEN_SIZE.width.min(maximum_size.width),
                    DEFAULT_SCREEN_SIZE.height.min(maximum_size.height),
                );
                log_info(
                    0,
                    format_args!(
                        "initial screen size {initial_size:?}, default: {DEFAULT_SCREEN_SIZE:?}, display: {maximum_size:?}"
                    ),
                );
                if proxy
                    .send_event(User_event::Initialize(initial_size))
                    .is_err()
                {
                    break;
                }
                continue;
            }
            Ui_input::Resize(size) => {
                window_size = Some(size);
                relayout = app_problem.is_none();
                Vizual_command::Resolve
            }
            Ui_input::Rerender => Vizual_command::Render,
            Ui_input::Layout(ids) => {
                if let Some(problem) = &app_problem {
                    for id in ids {
                        let _ = problem.root.invalidate_formula(id).await?;
                    }
                }
                relayout = true;
                Vizual_command::Resolve
            }
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

        if let Vizual_command::Focus(reference) = &command {
            focus.set_with_reference(reference);
            relayout = true;
            command = Vizual_command::Resolve;
        }

        if relayout {
            let problem = layout_problem(
                root.clone(),
                rerender.clone(),
                theme.clone(),
                &focus,
                &mut root_slot,
                &mut text_context,
                Arc::clone(&variables),
            )
            .await?;
            solution = Some(problem.solve(size).await?);
            app_problem = Some(problem);
            command = Vizual_command::Render;
        } else if matches!(command, Vizual_command::Resolve) {
            if let Some(problem) = &app_problem {
                solution = Some(problem.solve(size).await?);
                command = Vizual_command::Render;
            }
        }

        if matches!(command, Vizual_command::Render)
            && let (Some(problem), Some(solution)) = (&mut app_problem, &solution)
        {
            let scene = log_duration(0, "app problem render", || {
                problem.render(
                    rerender.clone(),
                    theme.clone(),
                    &focus,
                    solution,
                    &mut text_context,
                )
            })
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
// almost all of the resulting slop is here and in text

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
        if let Some(system) = window.theme().map(map_system_theme) {
            let _ = self.input.send(Ui_input::System_theme(system));
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
        self.state = Some(Render_state {
            surface,
            window,
            valid,
        });
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

        let timestamp = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".to_owned());
        log_info(0, format_args!("render timestamp: {timestamp}"));

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
        if reconfigure_after_present {
            self.context.configure_surface(&state.surface);
            state.window.request_redraw();
        }
    }
}

impl ApplicationHandler<User_event> for Window_app {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let system = event_loop
            .system_theme()
            .map(map_system_theme)
            .unwrap_or(System_theme::Dark);
        let _ = self.input.send(Ui_input::System_theme(system));

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
            WindowEvent::ThemeChanged(theme) => {
                let system = map_system_theme(theme);
                let _ = self.input.send(Ui_input::System_theme(system));
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
                    modifiers: map_modifiers(self.modifiers),
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
            User_event::Initialize(initial_size) => {
                self.initializing = false;
                self.create_window(event_loop, initial_size);
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

fn map_system_theme(theme: Window_theme) -> System_theme {
    if matches!(theme, Window_theme::Dark) {
        System_theme::Dark
    } else {
        System_theme::Light
    }
}

/// Runs a Vizual application on the calling thread.
///
/// A Tokio runtime must already be active. Winit owns the calling thread until
/// the window closes; widget tasks continue on the runtime. Vizual creates the
/// render manager used by the root widget.
pub fn run<T: Widget_trait>(title: impl Into<String>, root: T) -> Result<()> {
    let Render_manager { rerender, receiver } = Render_manager::new();
    let theme = Store::new(Theme::default());
    let root = Root::new(root.into_shared()).into_shared();
    let runtime = tokio::runtime::Handle::try_current()
        .wrap_err("vizual::run requires an active Tokio runtime")?;
    let event_loop = EventLoop::<User_event>::with_user_event()
        .build()
        .wrap_err("Failed to initialize the Winit event loop")?;
    let proxy = event_loop.create_proxy();
    let error_proxy = proxy.clone();
    let (input_sender, input_receiver) = mpsc::unbounded_channel();
    let ui_theme = theme.clone();
    let ui_task = runtime.spawn(async move {
        if let Err(error) = ui_loop(root, rerender, ui_theme, receiver, input_receiver, proxy).await
        {
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

#[cfg(test)]
mod tests;
