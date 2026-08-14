use color_eyre::eyre::Result;
use vizual_macros::display;

use super::super::{Focus_provider, Widget, Widget_trait};
use crate::{
    component::{Children, context::Component_context},
    event::Key_event,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
};

#[derive(Clone)]
pub struct Root(Widget);

impl Root {
    pub fn new(widget: impl Widget_trait) -> Self {
        Self(Box::new(widget))
    }
}

#[async_trait::async_trait]
impl Widget_trait for Root {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        Ok(vec![display!(self.0.clone())])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<crate::Vizual_msg> {
        if crate::check_quit_event(key) {
            return crate::Vizual_msg::new(crate::Vizual_command::Quit);
        }

        crate::Vizual_msg::none()
    }
}
