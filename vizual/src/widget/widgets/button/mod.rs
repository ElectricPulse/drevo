use crate::macros::display;
use async_trait::async_trait;
use color_eyre::Result;

use super::{
    super::{Layout_input, Widget_trait},
    block::{Block, Block_style},
};
use crate::{
    Vizual_msg, component::Children, event::Key_code, handlers::Submit_handler,
    layouter::objective::Delta, style::Color, theme::Theme, widget::Widget,
};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq)]
pub struct Button_style {
    pub block: Block_style,
    pub highlight: Color,
}

#[derive(Clone)]
pub struct Button {
    content: Widget,
    // payload is if the button is focused
    // there is no use case for this but one has to provide some payload to Submit_handler
    // and in the future one probably will provide some useful payload
    // so there is no reason for me to implement a Submit_without_payload_handler right now
    click_handler: Option<Box<dyn Submit_handler<bool>>>,
    pub highlighted: bool,
    pub delta: Option<Delta>,
}

fn resolve_block_style(theme: &Theme, highlighted: bool) -> Block_style {
    let button = theme.specific.button;
    let mut block = button.block;
    if highlighted {
        block.background = button.highlight;
    }
    block
}

impl Button {
    pub fn new(content: impl Widget_trait, click_handler: impl Submit_handler<bool>) -> Self {
        Self {
            content: content.as_any(),
            click_handler: Some(Box::new(click_handler)),
            highlighted: false,
            delta: None,
        }
    }

    pub fn around(content: impl Widget_trait) -> Self {
        Self {
            content: content.as_any(),
            click_handler: None,
            highlighted: false,
            delta: None,
        }
    }
}

impl Button {
    async fn submit(&mut self, value: bool, relayout: crate::Signal) -> Result<Vizual_msg> {
        let Some(handler) = &mut self.click_handler else {
            return Vizual_msg::none();
        };
        let message = handler.on_submit(value).await?;
        if matches!(message.command, crate::Vizual_command::Resolve) {
            relayout.send();
            return Vizual_msg::none();
        }
        Ok(message)
    }
}

#[async_trait]
impl Widget_trait for Button {
    async fn layout(
        &mut self,
        Layout_input {
            relayout,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let style = resolve_block_style(&theme, self.highlighted);

        let mut block = Block::new(self.content.clone(), style);
        block.focusable = true;
        block.delta = self.delta.clone();

        Ok(vec![display!(block)])
    }

    async fn on_mouse_click(
        &mut self,
        input: crate::widget::Mouse_event<'_>,
    ) -> Result<Vizual_msg> {
        self.submit(false, input.relayout).await
    }

    async fn on_key_press(&mut self, input: crate::widget::Key_press<'_>) -> Result<Vizual_msg> {
        let key = input.key;
        if matches!(key.code, Key_code::Enter) {
            return self.submit(true, input.relayout).await;
        }

        Vizual_msg::none()
    }
}
