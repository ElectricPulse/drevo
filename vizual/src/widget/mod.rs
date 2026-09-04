pub mod conversion;
pub mod custom_widget;
pub mod widgets;

pub use conversion::IntoWidgets;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use derive_where::derive_where;
use std::sync::{Arc, Weak};
use winit::window::Window;

use crate::{
    component::{Children, RenderContext, SharedComponent, context::ComponentContext},
    event::{Event, KeyEvent, PointerEvent},
    geometry::Rect,
    graphics::scene::Scene,
    graphics::text::TextContext,
    handlers::RetrieveHandler,
    layouter::hitbox::Hitbox,
    slot::{ComponentSlot, manager::Slots},
    state::Store,
    sync::{Mutex, MutexGuard, ThreadSafe},
    theme::Theme,
};

use super::{Signal, VizualMsg};

pub type Widget = Box<dyn WidgetTrait>;

pub struct FocusProvider {
    focused: bool,
    active: bool,
}

impl FocusProvider {
    pub(crate) fn new(focused: bool) -> Self {
        Self {
            focused,
            active: false,
        }
    }

    pub fn get(&self) -> bool {
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
pub struct LayoutInput<'a> {
    pub relayout: Signal,
    pub theme: Store<Theme>,
    pub focus: &'a mut FocusProvider,
    pub hitbox: &'a mut Hitbox,
    pub parent: Hitbox,
    pub problem: ComponentContext,
    pub text_context: &'a mut TextContext,
    pub slots: &'a mut Slots<'a>,
    pub root: &'a crate::component::SharedComponent,
    pub mask: &'a mut bool,
}

pub struct RenderInput<'a, 'scene> {
    pub rerender: crate::Signal,
    pub theme: Store<Theme>,
    pub focus: &'a FocusProvider,
    pub hitbox: Rect,
    pub scene: &'a mut Scene<'scene>,
    pub text_context: &'a mut TextContext,
    pub context: &'a RenderContext<'a>,
}

pub struct AllEvents<'a> {
    pub event: &'a Event,
    pub relayout: Signal,
    pub window: Option<Arc<Window>>,
}

pub struct MouseEvent<'a> {
    pub mouse: &'a PointerEvent,
    pub relayout: Signal,
    pub window: Option<Arc<Window>>,
}

pub struct KeyPress<'a> {
    pub key: &'a KeyEvent,
    pub relayout: Signal,
    pub window: Option<Arc<Window>>,
}

pub struct OtherEvent<'a> {
    pub event: &'a Event,
    pub relayout: Signal,
    pub window: Option<Arc<Window>>,
}

#[async_trait]
/// A widget that participates in layout and painting.
///
/// Widgets are cloneable because components may be created and destroyed regularly during layout.
/// A wrapper such as `Align` must be able to return a fresh clone of its child whenever its
/// `layout` method runs. Applications can opt into shared widget identity by wrapping a widget in
/// [`SharedWidget`], which uses `Arc`-backed shared ownership. State that is inherently shared
/// across component instances, such as theme state, belongs in [`State`] instead.
///
/// This trait uses [`dyn_clone::DynClone`] rather than [`Clone`] directly because `Clone` is not
/// dyn-compatible. `DynClone` preserves the clone requirement for concrete widgets while allowing
/// heterogeneous widgets to be stored as `Box<dyn WidgetTrait>`.
pub trait WidgetTrait: ThreadSafe + dyn_clone::DynClone {
    /// Configures this widget's mutable hitbox and returns its visual children.
    ///
    /// Every child receives four solver variables which are shared with its parent by default.
    /// Positioning widgets make individual edges independent when they add another equation for
    /// that edge. Widgets that derive their size from returned children must add those
    /// relationships explicitly.
    async fn layout(&mut self, _input: LayoutInput<'_>) -> Result<Children> {
        Ok(Vec::new())
    }

    // The hitbox must be a resolved hitbox returned by the layouter.
    // RenderContext carries the root traversal's solution and focus state so a render boundary,
    // such as Scroll, can recursively paint its retained child subtree into another scene without
    // inventing a second layout or losing the frame's focus information.
    async fn render(&mut self, _input: RenderInput<'_, '_>) -> Result<()> {
        Ok(())
    }

    // Event handling defaults to no action for non-interactive widgets.

    async fn on_all_events(&mut self, _input: AllEvents<'_>) -> Result<VizualMsg> {
        VizualMsg::none()
    }

    async fn on_mouse_click(&mut self, _input: MouseEvent<'_>) -> Result<VizualMsg> {
        VizualMsg::none()
    }

    async fn on_key_press(&mut self, _input: KeyPress<'_>) -> Result<VizualMsg> {
        VizualMsg::none()
    }

    async fn on_other_event(&mut self, _input: OtherEvent<'_>) -> Result<VizualMsg> {
        VizualMsg::none()
    }

    async fn forward_event(
        &mut self,
        event: &Event,
        relayout: Signal,
        window: Arc<Window>,
    ) -> Result<VizualMsg> {
        let msg = self
            .on_all_events(AllEvents {
                event,
                relayout: relayout.clone(),
                window: Some(Arc::clone(&window)),
            })
            .await?;

        if msg.has_command() || !msg.propagate {
            return Ok(msg);
        }

        if let Event::Key(key) = event {
            return self
                .on_key_press(KeyPress {
                    key,
                    relayout,
                    window: Some(window),
                })
                .await;
        }

        if let Event::Pointer(mouse) = event {
            return self
                .on_mouse_click(MouseEvent {
                    mouse,
                    relayout,
                    window: Some(window),
                })
                .await;
        }

        self.on_other_event(OtherEvent {
            event,
            relayout,
            window: Some(window),
        })
        .await
    }

    fn as_any(self) -> Widget
    where
        Self: Sized,
    {
        Box::new(self)
    }

    fn into_shared(self) -> SharedWidget<Self>
    where
        Self: Sized,
    {
        SharedWidget::new(self)
    }

    async fn into_children(
        self,
        slot: &mut ComponentSlot,
        problem: ComponentContext,
    ) -> Result<Children>
    where
        Self: Sized,
    {
        Ok(vec![slot.set(self, problem).await?])
    }
}

dyn_clone::clone_trait_object!(WidgetTrait);

#[derive_where(Clone)]
pub struct SharedWidget<T: ThreadSafe + ?Sized>(Arc<Mutex<T>>);

#[async_trait]
impl WidgetTrait for Widget {
    async fn layout(&mut self, input: LayoutInput<'_>) -> Result<Children> {
        (**self).layout(input).await
    }

    async fn render(&mut self, input: RenderInput<'_, '_>) -> Result<()> {
        (**self).render(input).await
    }

    async fn on_all_events(&mut self, input: AllEvents<'_>) -> Result<VizualMsg> {
        (**self).on_all_events(input).await
    }

    async fn on_mouse_click(&mut self, input: MouseEvent<'_>) -> Result<VizualMsg> {
        (**self).on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: KeyPress<'_>) -> Result<VizualMsg> {
        (**self).on_key_press(input).await
    }

    async fn on_other_event(&mut self, input: OtherEvent<'_>) -> Result<VizualMsg> {
        (**self).on_other_event(input).await
    }

    async fn forward_event(
        &mut self,
        event: &Event,
        relayout: Signal,
        window: Arc<Window>,
    ) -> Result<VizualMsg> {
        (**self).forward_event(event, relayout, window).await
    }
}

#[async_trait]
impl WidgetTrait for SharedComponent {
    async fn layout(&mut self, input: LayoutInput<'_>) -> Result<Children> {
        self.lock().await?.widget.layout(input).await
    }

    async fn render(&mut self, input: RenderInput<'_, '_>) -> Result<()> {
        self.lock().await?.widget.render(input).await
    }

    async fn on_all_events(&mut self, input: AllEvents<'_>) -> Result<VizualMsg> {
        self.lock().await?.widget.on_all_events(input).await
    }

    async fn on_mouse_click(&mut self, input: MouseEvent<'_>) -> Result<VizualMsg> {
        self.lock().await?.widget.on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: KeyPress<'_>) -> Result<VizualMsg> {
        self.lock().await?.widget.on_key_press(input).await
    }

    async fn on_other_event(&mut self, input: OtherEvent<'_>) -> Result<VizualMsg> {
        self.lock().await?.widget.on_other_event(input).await
    }

    async fn forward_event(
        &mut self,
        event: &Event,
        relayout: Signal,
        window: Arc<Window>,
    ) -> Result<VizualMsg> {
        self.lock()
            .await?
            .widget
            .forward_event(event, relayout, window)
            .await
    }
}

#[async_trait]
impl<T, Value> RetrieveHandler<Value> for SharedWidget<T>
where
    T: RetrieveHandler<Value> + ThreadSafe + ?Sized,
    Value: ThreadSafe,
{
    async fn on_retrieve(&mut self) -> Result<crate::state::State<Value>> {
        self.lock().await?.on_retrieve().await
    }
}

impl<T: ThreadSafe> SharedWidget<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

impl<T: ThreadSafe + ?Sized> SharedWidget<T> {
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
impl<T: WidgetTrait + ?Sized> WidgetTrait for SharedWidget<T> {
    async fn layout(&mut self, input: LayoutInput<'_>) -> Result<Children> {
        self.0.lock().await?.layout(input).await
    }

    async fn render(&mut self, input: RenderInput<'_, '_>) -> Result<()> {
        self.0.lock().await?.render(input).await
    }

    async fn on_all_events(&mut self, input: AllEvents<'_>) -> Result<VizualMsg> {
        let mut inner = self.0.lock().await?;
        inner.on_all_events(input).await
    }

    async fn on_mouse_click(&mut self, input: MouseEvent<'_>) -> Result<VizualMsg> {
        let mut inner = self.0.lock().await?;
        inner.on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: KeyPress<'_>) -> Result<VizualMsg> {
        let mut inner = self.0.lock().await?;
        inner.on_key_press(input).await
    }

    async fn on_other_event(&mut self, input: OtherEvent<'_>) -> Result<VizualMsg> {
        let mut inner = self.0.lock().await?;
        inner.on_other_event(input).await
    }

    async fn forward_event(
        &mut self,
        event: &Event,
        relayout: Signal,
        window: Arc<Window>,
    ) -> Result<VizualMsg> {
        let mut inner = self.0.lock().await?;
        inner.forward_event(event, relayout, window).await
    }
}

impl<T: WidgetTrait + ?Sized> From<SharedWidget<T>> for Widget {
    fn from(value: SharedWidget<T>) -> Self {
        Box::new(value)
    }
}
