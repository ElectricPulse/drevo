pub mod widgets;

use async_trait::async_trait;
use auto_impl::auto_impl;
use color_eyre::eyre::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    component::{Children, context::Component_context},
    display::Display,
    event::{Event, Key_event, Pointer_event},
    geometry::{Direction, Rect},
    handlers::Retrieve_handler,
    layouter::{constraints::shrink_wrap, hitbox::Hitbox},
    slot::{Component_slot, manager::Slots},
    sync::{Mutex, MutexGuard, Thread_safe},
    text::Text_context,
};

use super::{Rerender, Vizual_msg};

pub type Widget = Box<dyn Widget_trait>;

/// Describes whether a widget introduces a visual component or only another widget layer.
pub enum Widget_type {
    /// A visual leaf whose dimensions are fully described by its own layout constraints.
    None,
    /// A virtual widget has no visual children of its own. The consumer immediately lays out the
    /// inner widget with the same focus, hitbox, problem, and slots. Reusing the hitbox this way is
    /// a fragile way to implement hierarchy because virtual layers do not receive distinct hitboxes.
    /// TODO: A widget returned through `Virtual` is only used for layout, so its `render`
    /// callback will not be called.
    Virtual(Widget),
    /// A visual widget owns the current component and returns the children laid out beneath it.
    #[non_exhaustive]
    Visual { children: Children },
}

impl Widget_type {
    pub fn none() -> Self {
        Self::None
    }

    /// Client-facing constructor for a component with multiple visual children.
    ///
    /// This applies containment constraints and shrink-wrap objectives on both axes so client
    /// widgets receive safe, predictable bounds by default. A widget with only one child should
    /// normally return that child through [`Widget_type::Virtual`] instead of introducing another
    /// component and another pair of shrink-wrap objectives.
    pub async fn visual(
        children: Children,
        hitbox: Hitbox,
        problem: &Component_context,
    ) -> Result<Self> {
        debug_assert!(
            children.len() > 1,
            "Widget_type::visual expects multiple children; use Widget_type::Virtual for one child"
        );
        Self::visual_with_shrink_wrap(children, hitbox, problem, true, true).await
    }

    pub(crate) async fn visual_with_shrink_wrap(
        children: Children,
        hitbox: Hitbox,
        problem: &Component_context,
        vertical_shrink: bool,
        horizontal_shrink: bool,
    ) -> Result<Self> {
        if vertical_shrink {
            shrink_wrap(problem, hitbox, &children, Direction::Vertical).await?;
        }
        if horizontal_shrink {
            shrink_wrap(problem, hitbox, &children, Direction::Horizontal).await?;
        }

        Ok(Self::Visual { children })
    }
}

pub struct Focus_provider {
    focused: bool,
    active: bool,
}

#[async_trait]
pub trait Shared_widget_trait: Thread_safe {
    async fn get(&mut self) -> Widget;
}

impl Focus_provider {
    pub(crate) fn new(focused: bool) -> Self {
        Self {
            focused,
            active: false,
        }
    }

    pub fn get(&mut self) -> bool {
        self.set_active(true);
        self.focused
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }
}

// TODO: Add a Themeable subtrait of Widget_trait that receives the theme in layout and render,
// just as Slots is passed to layout now.
#[async_trait]
/// A widget that participates in layout and painting.
pub trait Widget_trait: Control + Thread_safe {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        Ok(Widget_type::none())
    }

    // The hitbox must be a resolved hitbox returned by the layouter.
    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Rect,
        _display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        Ok(None)
    }

    fn into_shared(self) -> Shared_widget<Self>
    where
        Self: Sized,
    {
        Shared_widget::new(self)
    }

    async fn into_children(
        self,
        slot: &mut Component_slot,
        problem: Component_context,
    ) -> Result<Children>
    where
        Self: Sized,
    {
        Ok(vec![slot.set(self, problem).await?])
    }
}

pub struct Shared_widget<T: Widget_trait>(Arc<Mutex<T>>);

#[async_trait]
impl Widget_trait for Widget {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        (**self)
            .layout(focus, hitbox, problem, text_context, slots)
            .await
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        (**self).render(focus, hitbox, display).await
    }
}

impl<T: Widget_trait> Clone for Shared_widget<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait]
impl<T, Value> Retrieve_handler<Value> for Shared_widget<T>
where
    T: Widget_trait + Retrieve_handler<Value>,
    Value: Thread_safe,
{
    async fn on_retrieve(&mut self) -> Result<Value> {
        self.lock().await?.on_retrieve().await
    }
}

impl<T: Widget_trait> Shared_widget<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, T>> {
        self.0.lock().await
    }
}

#[async_trait]
impl<T: Widget_trait> Shared_widget_trait for Shared_widget<T> {
    async fn get(&mut self) -> Widget {
        Box::new(self.clone())
    }
}

#[async_trait]
impl<T: Widget_trait> Widget_trait for Shared_widget<T> {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        component: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        self.0
            .lock()
            .await?
            .layout(focus, component, problem, text_context, slots)
            .await
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        self.0.lock().await?.render(focus, hitbox, display).await
    }
}

impl<T: Widget_trait> From<Shared_widget<T>> for Widget {
    fn from(value: Shared_widget<T>) -> Self {
        Box::new(value)
    }
}

#[async_trait]
impl<T: Widget_trait> Control for Shared_widget<T> {
    async fn on_all_events(&mut self, event: &Event) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_all_events(event).await
    }

    async fn on_mouse_click(&mut self, mouse: &Pointer_event) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_mouse_click(mouse).await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_other_event(event).await
    }

    async fn forward_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.forward_event(event).await
    }
}

#[async_trait]
#[auto_impl(Box)]
/// Handles normalized UI events.
pub trait Control {
    // If needed, Focus_provider, the hitbox, or any other field from Shared_component can be passed into these methods.
    async fn on_all_events(&mut self, _event: &Event) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn on_key_press(&mut self, _key: &Key_event) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn on_other_event(&mut self, _event: &Event) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn forward_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let msg = self.on_all_events(event).await?;

        if msg.has_command() || !msg.propagate {
            return Ok(msg);
        }

        if let Event::Key(key) = event {
            return self.on_key_press(key).await;
        }

        if let Event::Pointer(mouse) = event {
            return self.on_mouse_click(mouse).await;
        }

        self.on_other_event(event).await
    }
}

pub type View_receiver = Widget_receiver;
pub type View_sender = Widget_sender;

type Message = Option<Widget>;

#[derive(Clone)]
pub struct Widget_sender {
    pub channel: mpsc::Sender<Message>,
    pub rerender: Rerender,
}

impl Widget_sender {
    pub async fn set(&self, widget: impl Widget_trait) {
        let _ = self.channel.send(Some(Box::new(widget))).await;
        self.rerender.send();
    }

    pub async fn reset(&self) {
        let _ = self.channel.send(None).await;
    }
}

pub struct Widget_receiver {
    pub channel: mpsc::Receiver<Message>,
}

impl Widget_receiver {
    pub fn get(&mut self) -> Message {
        match self.channel.try_recv() {
            Err(err) => match err {
                mpsc::error::TryRecvError::Empty => None,
                mpsc::error::TryRecvError::Disconnected => None,
            },
            Ok(value) => value,
        }
    }
}

pub fn new_view(rerender: Rerender) -> (Widget_receiver, Widget_sender) {
    let (tx, rx) = mpsc::channel(1);
    let sender = Widget_sender {
        channel: tx,
        rerender,
    };
    let receiver = Widget_receiver { channel: rx };
    (receiver, sender)
}
