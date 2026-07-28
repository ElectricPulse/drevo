pub mod widgets;

use async_trait::async_trait;
use auto_impl::auto_impl;
use color_eyre::eyre::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    backend::graphics::Paint_context,
    component::{Child_slot, Children},
    event::{Event, Key_event, Pointer_event},
    geometry::Rect,
    handlers::Retrieve_handler,
    hitbox::Hitbox,
    layouter::Problem_context,
    slot_manager::Slots,
    sync::{Mutex, MutexGuard, Thread_safe},
};

use super::{Rerender, Vizual_msg};

pub type Any_renderable = Box<dyn Renderable>;

/// Describes whether a widget introduces a visual component or only another widget layer.
pub enum Widget_type {
    /// A virtual widget has no visual children of its own. The consumer immediately lays out the
    /// inner renderable with the same focus, hitbox, problem, and slots. Reusing the hitbox this way is
    /// a fragile way to implement hierarchy because virtual layers do not receive distinct hitboxes.
    /// TODO: A renderable returned through `Virtual` is only used for layout, so its `render`
    /// callback will not be called.
    Virtual(Box<dyn Renderable>),
    /// A visual widget owns the current component and returns the children laid out beneath it.
    Visual(Children),
}

pub struct Focus_provider {
    focused: bool,
    active: bool,
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

#[async_trait]
pub trait Shared_widget: Thread_safe {
    async fn get(&mut self) -> Any_renderable;
}

// TODO: Add a Themeable subtrait of Renderable that receives the theme in layout and render,
// just as Slots is passed to layout now.
#[async_trait]
pub trait Renderable: Control + Thread_safe {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        Ok(Widget_type::Visual(Vec::new()))
    }

    // The hitbox must be a resolved hitbox returned by the layouter.
    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Rect,
        _paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        Ok(None)
    }

    fn into_shared(self) -> Shared_renderable<Self>
    where
        Self: Sized,
    {
        Shared_renderable::new(self)
    }

    async fn into_children(
        self,
        slot: &mut Child_slot,
        problem: Problem_context,
    ) -> Result<Children>
    where
        Self: Sized,
    {
        Ok(vec![slot.set(self, problem).await?])
    }
}

pub struct Shared_renderable<T: Renderable>(Arc<Mutex<T>>);

#[async_trait]
impl Renderable for Any_renderable {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Problem_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        (**self).layout(focus, hitbox, problem, slots).await
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        (**self).render(focus, hitbox, paint).await
    }
}

impl<T: Renderable> Clone for Shared_renderable<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[async_trait]
impl<T, Value> Retrieve_handler<Value> for Shared_renderable<T>
where
    T: Renderable + Retrieve_handler<Value>,
    Value: Thread_safe,
{
    async fn on_retrieve(&mut self) -> Result<Value> {
        self.lock().await?.on_retrieve().await
    }
}

impl<T: Renderable> Shared_renderable<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub async fn lock(&self) -> Result<MutexGuard<'_, T>> {
        self.0.lock().await
    }
}

#[async_trait]
impl<T: Renderable> Shared_widget for Shared_renderable<T> {
    async fn get(&mut self) -> Any_renderable {
        Box::new(self.clone())
    }
}

#[async_trait]
impl<T: Renderable> Renderable for Shared_renderable<T> {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        component: Hitbox,
        problem: Problem_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        self.0
            .lock()
            .await?
            .layout(focus, component, problem, slots)
            .await
    }

    async fn render(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        self.0.lock().await?.render(focus, hitbox, paint).await
    }
}

impl<T: Renderable> From<Shared_renderable<T>> for Any_renderable {
    fn from(value: Shared_renderable<T>) -> Self {
        Box::new(value)
    }
}

#[async_trait]
impl<T: Renderable> Control for Shared_renderable<T> {
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
pub trait Control {
    // If needed, Focus_provider, the hitbox, or any other field from Child can be passed into these methods.
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

pub type View_receiver = Renderable_receiver;
pub type View_sender = Renderable_sender;

type Message = Option<Any_renderable>;

#[derive(Clone)]
pub struct Renderable_sender {
    pub channel: mpsc::Sender<Message>,
    pub rerender: Rerender,
}

impl Renderable_sender {
    pub async fn set(&self, widget: impl Renderable) {
        let _ = self.channel.send(Some(Box::new(widget))).await;
        self.rerender.send();
    }

    pub async fn reset(&self) {
        let _ = self.channel.send(None).await;
    }
}

pub struct Renderable_receiver {
    pub channel: mpsc::Receiver<Message>,
}

impl Renderable_receiver {
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

pub fn new_view(rerender: Rerender) -> (Renderable_receiver, Renderable_sender) {
    let (tx, rx) = mpsc::channel(1);
    let sender = Renderable_sender {
        channel: tx,
        rerender,
    };
    let receiver = Renderable_receiver { channel: rx };
    (receiver, sender)
}
