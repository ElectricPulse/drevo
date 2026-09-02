use crate::macros::display;
use color_eyre::eyre::Result;

use super::super::{Layout_input, Widget, Widget_trait};
use crate::component::Children;

#[derive(Clone)]
pub struct Root(Widget);

impl Root {
    pub fn new(widget: impl Widget_trait) -> Self {
        Self(widget.as_any())
    }
}

#[async_trait::async_trait]
impl Widget_trait for Root {
    async fn layout(&mut self, Layout_input { slots, .. }: Layout_input<'_>) -> Result<Children> {
        Ok(vec![display!(self.0.clone())])
    }

    async fn on_key_press(
        &mut self,
        input: crate::widget::Key_press<'_>,
    ) -> Result<crate::Vizual_msg> {
        let key = input.key;
        if crate::check_quit_event(key) {
            return crate::Vizual_msg::new(crate::Vizual_command::Quit);
        }

        crate::Vizual_msg::none()
    }
}
