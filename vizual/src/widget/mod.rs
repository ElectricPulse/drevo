pub mod conversion;
pub mod custom_widget;
pub mod widgets;

pub use conversion::Into_widgets;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use derive_where::derive_where;
use std::sync::{Arc, Weak};

use crate::{
    component::{Children, Render_context, context::Component_context},
    event::{Event, Key_event, Pointer_event},
    geometry::Rect,
    graphics::scene::Scene,
    graphics::text::Text_context,
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::{Component_slot, manager::Slots},
    state::Store,
    sync::{Mutex, MutexGuard, Thread_safe},
    theme::Theme,
};

use super::{Signal, Vizual_msg};

pub type Widget = Box<dyn Widget_trait>;

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
        self.set_interactive(true);
        self.focused
    }

    pub fn set_interactive(&mut self, active: bool) {
        self.active = active;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }
}

// These two input structs have been created to solve two issues
// 1. when adding a new property into input of layout()/render() - which happens often
//    it means that one doesn't have to fix all the outdated function signatures
// 2. the function signatures are really big and even though no component used even a
//    fraction of everything present in the arguments they still took up a couple lines that one had to copy paste
// Now as for what this huge argument list says about the state of the current architecture:
// I think its obscenely huge and should be made smaller - functional programming explicitly tries to avoid functions that seemingly take in everything
// it's a sign of failure to separate concerns and a failure to implement abstractions
pub struct Layout_input<'a> {
    pub relayout: Signal,
    pub theme: Store<Theme>,
    pub focus: &'a mut Focus_provider,
    pub hitbox: &'a mut Hitbox,
    pub parent: Hitbox,
    pub problem: Component_context,
    pub text_context: &'a mut Text_context,
    pub slots: &'a mut Slots<'a>,
    pub root: &'a crate::component::Shared_component,
    pub mask: &'a mut bool,
}

pub struct Render_input<'a, 'scene> {
    pub rerender: crate::Signal,
    pub theme: Store<Theme>,
    pub focus: &'a mut Focus_provider,
    pub hitbox: Rect,
    pub scene: &'a mut Scene<'scene>,
    pub text_context: &'a mut Text_context,
    pub context: &'a Render_context<'a>,
}

pub struct All_events<'a> {
    pub event: &'a Event,
    pub relayout: Signal,
}

pub struct Mouse_event<'a> {
    pub mouse: &'a Pointer_event,
    pub relayout: Signal,
}

pub struct Key_press<'a> {
    pub key: &'a Key_event,
    pub relayout: Signal,
}

pub struct Other_event<'a> {
    pub event: &'a Event,
    pub relayout: Signal,
}

#[async_trait]
/// A widget that participates in layout and painting.
///
/// Widgets are cloneable because components may be created and destroyed regularly during layout.
/// A wrapper such as `Align` must be able to return a fresh clone of its child whenever its
/// `layout` method runs. Applications can opt into shared widget identity by wrapping a widget in
/// [`Shared_widget`], which uses `Arc`-backed shared ownership. State that is inherently shared
/// across component instances, such as theme state, belongs in [`State`] instead.
///
/// This trait uses [`dyn_clone::DynClone`] rather than [`Clone`] directly because `Clone` is not
/// dyn-compatible. `DynClone` preserves the clone requirement for concrete widgets while allowing
/// heterogeneous widgets to be stored as `Box<dyn Widget_trait>`.
pub trait Widget_trait: Thread_safe + dyn_clone::DynClone {
    /// Configures this widget's mutable hitbox and returns its visual children.
    ///
    /// Every child receives four solver variables which are shared with its parent by default.
    /// Positioning widgets make individual edges independent when they add another equation for
    /// that edge. Widgets that derive their size from returned children must add those
    /// relationships explicitly.
    async fn layout(&mut self, _input: Layout_input<'_>) -> Result<Children> {
        Ok(Vec::new())
    }

    // The hitbox must be a resolved hitbox returned by the layouter.
    // Render_context carries the root traversal's solution and focus state so a render boundary,
    // such as Scroll, can recursively paint its retained child subtree into another scene without
    // inventing a second layout or losing the frame's focus information.
    async fn render(&mut self, _input: Render_input<'_, '_>) -> Result<()> {
        Ok(())
    }

    // Event handling defaults to no action for non-interactive widgets.

    async fn on_all_events(&mut self, _input: All_events<'_>) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn on_mouse_click(&mut self, _input: Mouse_event<'_>) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn on_key_press(&mut self, _input: Key_press<'_>) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn on_other_event(&mut self, _input: Other_event<'_>) -> Result<Vizual_msg> {
        Vizual_msg::none()
    }

    async fn forward_event(&mut self, event: &Event, relayout: Signal) -> Result<Vizual_msg> {
        let msg = self
            .on_all_events(All_events {
                event,
                relayout: relayout.clone(),
            })
            .await?;

        if msg.has_command() || !msg.propagate {
            return Ok(msg);
        }

        if let Event::Key(key) = event {
            return self.on_key_press(Key_press { key, relayout }).await;
        }

        if let Event::Pointer(mouse) = event {
            return self.on_mouse_click(Mouse_event { mouse, relayout }).await;
        }

        self.on_other_event(Other_event { event, relayout }).await
    }

    fn as_any(self) -> Widget
    where
        Self: Sized,
    {
        Box::new(self)
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

dyn_clone::clone_trait_object!(Widget_trait);

#[derive_where(Clone)]
pub struct Shared_widget<T: Thread_safe + ?Sized>(Arc<Mutex<T>>);

#[async_trait]
impl Widget_trait for Widget {
    async fn layout(&mut self, input: Layout_input<'_>) -> Result<Children> {
        (**self).layout(input).await
    }

    async fn render(&mut self, input: Render_input<'_, '_>) -> Result<()> {
        (**self).render(input).await
    }

    async fn on_all_events(&mut self, input: All_events<'_>) -> Result<Vizual_msg> {
        (**self).on_all_events(input).await
    }

    async fn on_mouse_click(&mut self, input: Mouse_event<'_>) -> Result<Vizual_msg> {
        (**self).on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: Key_press<'_>) -> Result<Vizual_msg> {
        (**self).on_key_press(input).await
    }

    async fn on_other_event(&mut self, input: Other_event<'_>) -> Result<Vizual_msg> {
        (**self).on_other_event(input).await
    }

    async fn forward_event(&mut self, event: &Event, relayout: Signal) -> Result<Vizual_msg> {
        (**self).forward_event(event, relayout).await
    }
}

#[async_trait]
impl<T, Value> Retrieve_handler<Value> for Shared_widget<T>
where
    T: Retrieve_handler<Value> + Thread_safe + ?Sized,
    Value: Thread_safe,
{
    async fn on_retrieve(&mut self) -> Result<crate::state::State<Value>> {
        self.lock().await?.on_retrieve().await
    }
}

impl<T: Thread_safe> Shared_widget<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

impl<T: Thread_safe + ?Sized> Shared_widget<T> {
    pub async fn lock(&self) -> Result<MutexGuard<'_, T>> {
        self.0.lock().await
    }

    pub fn as_reference(&self) -> Weak<Mutex<T>> {
        Arc::downgrade(&self.0)
    }

    pub fn compare(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub fn compare_reference(&self, other: &Arc<Mutex<T>>) -> bool {
        Arc::ptr_eq(&self.0, other)
    }
}

#[async_trait]
impl<T: Widget_trait + ?Sized> Widget_trait for Shared_widget<T> {
    async fn layout(&mut self, input: Layout_input<'_>) -> Result<Children> {
        self.0.lock().await?.layout(input).await
    }

    async fn render(&mut self, input: Render_input<'_, '_>) -> Result<()> {
        self.0.lock().await?.render(input).await
    }

    async fn on_all_events(&mut self, input: All_events<'_>) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_all_events(input).await
    }

    async fn on_mouse_click(&mut self, input: Mouse_event<'_>) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: Key_press<'_>) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_key_press(input).await
    }

    async fn on_other_event(&mut self, input: Other_event<'_>) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.on_other_event(input).await
    }

    async fn forward_event(&mut self, event: &Event, relayout: Signal) -> Result<Vizual_msg> {
        let mut inner = self.0.lock().await?;
        inner.forward_event(event, relayout).await
    }
}

impl<T: Widget_trait + ?Sized> From<Shared_widget<T>> for Widget {
    fn from(value: Shared_widget<T>) -> Self {
        Box::new(value)
    }
}
