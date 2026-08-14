use std::sync::Weak;

use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{Focus_provider, Shared_widget, Widget_trait};
use crate::{
    Render,
    component::{Children, context::Component_context},
    graphics::text::Text_context,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::Store,
    sync::{Mutex, Thread_safe},
    theme::Theme,
};

pub type Selector<T> = Weak<Mutex<T>>;

// TODO: Merge Custom_widget_trait into Widget_trait once payload-based layout is supported there.
#[async_trait]
pub trait Custom_widget_trait: Thread_safe {
    type Payload: Thread_safe;

    async fn layout(
        &mut self,
        render: Render,
        theme: Store<Theme>,
        focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        parent: Hitbox,
        problem: Component_context,
        text_context: &mut Text_context,
        slots: &mut Slots,
        payload: Self::Payload,
    ) -> Result<Children>;

    fn into_shared(self) -> Shared_widget<Self>
    where
        Self: Sized,
    {
        Shared_widget::new(self)
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
        theme: Store<Theme>,
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
