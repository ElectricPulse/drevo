use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{Focus_provider, Widget_trait};
use crate::{
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    sync::{Mutex, Thread_safe},
    text::Text_context,
};

pub type Shared_custom_widget<T> = Arc<Mutex<T>>;

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

    fn into_shared(self) -> Shared_custom_widget<Self>
    where
        Self: Sized,
    {
        Arc::new(Mutex::new(self))
    }
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
