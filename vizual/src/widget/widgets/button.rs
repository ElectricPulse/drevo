use async_trait::async_trait;
use color_eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Widget_trait},
    block::Block,
};
use crate::{
    Vizual_msg,
    component::{Children, context::Component_context},
    event::Pointer_event,
    handlers::Submit_handler,
    layouter::{hitbox::Hitbox, objective::Delta},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::Widget,
};

#[derive(Clone)]
pub struct Button {
    content: Widget,
    click_handler: Option<Box<dyn Submit_handler<String>>>,
    pub highlighted: bool,
    pub delta: Option<Delta>,
}

impl Button {
    pub fn new(content: impl Widget_trait, click_handler: impl Submit_handler<String>) -> Self {
        Self {
            content: Box::new(content),
            click_handler: Some(Box::new(click_handler)),
            highlighted: false,
            delta: None,
        }
    }

    pub fn around(content: impl Widget_trait) -> Self {
        Self {
            content: Box::new(content),
            click_handler: None,
            highlighted: false,
            delta: None,
        }
    }
}

#[async_trait]
impl Widget_trait for Button {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let mut block = Block::new(self.content.clone());
        block.highlighted = self.highlighted;
        block.delta = self.delta.clone();

        Ok(vec![display!(block)])
    }

    async fn on_mouse_click(&mut self, _mouse: &Pointer_event) -> Result<Vizual_msg> {
        match &mut self.click_handler {
            Some(click_handler) => click_handler.on_submit(None).await,
            None => Vizual_msg::none(),
        }
    }
}
