use std::sync::{Arc, Weak};

use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{Focus_provider, Widget_trait};
use crate::{
    Render,
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    sync::{Mutex, Thread_safe},
    text::Text_context,
    theme::Theme,
};

pub type Selector<T> = Weak<Mutex<T>>;
pub type Shared_custom_widget<T> = Arc<Mutex<T>>;

#[async_trait]
pub trait Custom_widget_trait: Thread_safe {
    type Payload: Thread_safe;

    async fn layout(
        &mut self,
        render: Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
        payload: Self::Payload,
    ) -> Result<Children>;

    fn into_shared(self) -> Shared_custom_widget<Self>
    where
        Self: Sized,
    {
        Arc::new(Mutex::new(self))
    }
}

#[async_trait]
impl<T> Custom_widget_trait for T
where
    T: Widget_trait,
{
    type Payload = bool;

    async fn layout(
        &mut self,
        render: Render,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
        _payload: Self::Payload,
    ) -> Result<Children> {
        Widget_trait::layout(
            self,
            render,
            theme,
            focus,
            hitbox,
            parent,
            problem,
            text_context,
            slots,
        )
        .await
    }
}
