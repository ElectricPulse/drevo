use std::ops::DerefMut;

use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};

use super::Widget_trait;
use crate::{component::Children, sync::Thread_safe};

// TODO: Merge Custom_widget_trait into Widget_trait once payload-based layout is supported there.
#[async_trait]
pub trait Custom_widget_trait: Thread_safe {
    type Payload: Thread_safe;

    async fn layout(
        &mut self,
        input: super::Layout_input<'_>,
        payload: Self::Payload,
    ) -> Result<Children>;
}

#[derive(Clone)]
pub struct Custom_widget<Widget, Payload> {
    pub widget: Widget,
    pub payload: Payload,
}

impl<Widget, Payload> Custom_widget<Widget, Payload> {
    pub fn new(widget: Widget, payload: Payload) -> Self {
        Self { widget, payload }
    }
}

#[async_trait]
impl<Widget, Payload> Widget_trait for Custom_widget<Widget, Payload>
where
    Widget: DerefMut + Clone + Thread_safe,
    Widget::Target: Custom_widget_trait<Payload = Payload>,
    Payload: Clone + Thread_safe,
{
    async fn layout(&mut self, input: super::Layout_input<'_>) -> Result<Children> {
        let contents = self.widget.layout(input, self.payload.clone()).await?;

        // TODO: handle this some other way
        if contents.len() != 1 {
            return Err(eyre!(
                "Custom widget layout must return exactly one child, got {}",
                contents.len()
            ));
        }
        Ok(contents)
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
        input: super::Layout_input<'_>,
        _payload: Self::Payload,
    ) -> Result<Children> {
        Widget_trait::layout(self, input).await
    }
}
