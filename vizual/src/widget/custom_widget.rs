use std::sync::{Arc, Weak};

use async_trait::async_trait;
use color_eyre::eyre::Result;
use tokio::sync::MutexGuard;

use super::{Focus_provider, Widget_trait};
use crate::{
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    sync::{Mutex, Thread_safe},
    text::Text_context,
};

pub type Selector<Payload: Thread_safe, T: Custom_widget_trait<Payload>> = Weak<Mutex<T>>;

pub struct Shared_custom_widget<Payload: Thread_safe, T: Custom_widget_trait<Payload> + ?Sized>(
    Arc<Mutex<T>>,
);

impl<Payload: Thread_safe, Widget: Custom_widget_trait<Payload>>
    Shared_custom_widget<Payload, Widget>
{
    pub fn new(widget: Widget) -> Self {
        Self(Arc::new(Mutex::new(widget)))
    }
}

impl<Payload: Thread_safe, T: Custom_widget_trait<Payload> + ?Sized>
    Shared_custom_widget<Payload, T>
{
    pub fn selector(&self) -> Selector<Payload, T> {
        Arc::downgrade(&self.0)
    }

    pub async fn lock(&self) -> Result<MutexGuard<T>> {
        self.0.lock().await
    }
}

#[async_trait]
pub trait Custom_widget_trait<Payload: Thread_safe>: Thread_safe {
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
        payload: Payload,
    ) -> Result<Children>;
}

#[async_trait]
impl<Payload, T> Custom_widget_trait<Payload> for T
where
    Payload: Thread_safe,
    T: Widget_trait,
{
    async fn layout(
        &mut self,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
        _payload: Payload,
    ) -> Result<Children> {
        Widget_trait::layout(self, focus, hitbox, parent, problem, text_context, slots).await
    }
}
