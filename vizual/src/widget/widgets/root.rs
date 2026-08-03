use color_eyre::eyre::Result;
use vizual_macros::display;

use super::super::{Focus_provider, Shared_widget, Widget_trait};
use crate::{
    component::{Children, context::Component_context},
    event::Key_event,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
};

pub struct Root<T: Widget_trait>(Shared_widget<T>);

impl<T: Widget_trait> Root<T> {
    pub fn new(widget: Shared_widget<T>) -> Self {
        Self(widget)
    }
}

#[async_trait::async_trait]
impl<T: Widget_trait> Widget_trait for Root<T> {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
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
