use color_eyre::eyre::Result;
use crate::macros::display;

use super::super::{Layout_input, Widget, Widget_trait};
use crate::{
    component::Children,
    event::Key_event,
};

#[derive(Clone)]
pub struct Root(Widget);

impl Root {
    pub fn new(widget: impl Widget_trait) -> Self {
        Self(widget.as_any())
    }
}

#[async_trait::async_trait]
impl Widget_trait for Root {
    async fn layout(
        &mut self,
        Layout_input { slots, .. }: Layout_input<'_>,
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
