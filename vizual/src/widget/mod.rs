pub mod widgets;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    component::{Children, context::Component_context},
    display::Display,
    event::{Event, Key_event, Pointer_event},
    geometry::Rect,
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::{Component_slot, manager::Slots},
    sync::{Mutex, MutexGuard, Thread_safe},
    text::Text_context,
};

use super::{Rerender, Vizual_msg};

pub type Widget = Box<dyn Widget_trait>;

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
pub trait Widget_trait: Thread_safe {
    /// Configures this widget's mutable hitbox and returns its visual children.
    ///
    /// A widget can reuse parent variables through [`Hitbox::share_start`],
    /// [`Hitbox::share_end`], [`Hitbox::share_dimension`], or [`Hitbox::full`]. Returned children
    /// are shrink-wrapped by default wherever neither side of an edge is shared.
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        Ok(Vec::new())
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

    // Event handling defaults to no action for non-interactive widgets.

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

// Basically a cloneable widget
pub type Generic_shared_widget = Shared_widget<Widget>;
pub struct Shared_widget<T: Widget_trait>(Arc<Mutex<T>>);

#[async_trait]
impl Widget_trait for Widget {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        (**self)
            .layout(focus, hitbox, parent, problem, text_context, slots)
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

    async fn on_all_events(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).on_all_events(event).await
    }

    async fn on_mouse_click(&mut self, mouse: &Pointer_event) -> Result<Vizual_msg> {
        (**self).on_mouse_click(mouse).await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        (**self).on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).on_other_event(event).await
    }

    async fn forward_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).forward_event(event).await
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
        component: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        self.0
            .lock()
            .await?
            .layout(focus, component, parent, problem, text_context, slots)
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

impl<T: Widget_trait> From<Shared_widget<T>> for Widget {
    fn from(value: Shared_widget<T>) -> Self {
        Box::new(value)
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
