use std::ops::DerefMut;

use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};

use super::WidgetTrait;
use crate::{component::Children, sync::ThreadSafe};

// TODO: Merge CustomWidgetTrait into WidgetTrait once payload-based layout is supported there.
#[async_trait]
pub trait CustomWidgetTrait: ThreadSafe {
    type Payload: ThreadSafe;

    async fn layout(
        &mut self,
        input: super::LayoutInput<'_>,
        payload: Self::Payload,
    ) -> Result<Children>;
}

#[derive(Clone)]
pub struct CustomWidget<Widget, Payload> {
    pub widget: Widget,
    pub payload: Payload,
}

impl<Widget, Payload> CustomWidget<Widget, Payload> {
    pub fn new(widget: Widget, payload: Payload) -> Self {
        Self { widget, payload }
    }
}

#[async_trait]
impl<Widget, Payload> WidgetTrait for CustomWidget<Widget, Payload>
where
    Widget: DerefMut + Clone + ThreadSafe,
    Widget::Target: CustomWidgetTrait<Payload = Payload>,
    Payload: Clone + ThreadSafe,
{
    async fn layout(&mut self, input: super::LayoutInput<'_>) -> Result<Children> {
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
impl<T> CustomWidgetTrait for T
where
    T: WidgetTrait,
{
    type Payload = bool;

    async fn layout(
        &mut self,
        input: super::LayoutInput<'_>,
        _payload: Self::Payload,
    ) -> Result<Children> {
        WidgetTrait::layout(self, input).await
    }
}
