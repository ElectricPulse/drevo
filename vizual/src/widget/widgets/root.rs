use crate::macros::display;
use color_eyre::eyre::Result;

use super::super::{LayoutInput, Widget, WidgetTrait};
use crate::component::Children;

#[derive(Clone)]
pub struct Root(Widget);

impl Root {
    pub fn new(widget: impl WidgetTrait) -> Self {
        Self(widget.as_any())
    }
}

#[async_trait::async_trait]
impl WidgetTrait for Root {
    async fn layout(&mut self, LayoutInput { slots, .. }: LayoutInput<'_>) -> Result<Children> {
        Ok(vec![display!(self.0.clone())])
    }

    async fn on_key_press(
        &mut self,
        input: crate::widget::KeyPress<'_>,
    ) -> Result<crate::VizualMsg> {
        let key = input.key;
        if crate::check_quit_event(key) {
            return crate::VizualMsg::new(crate::VizualCommand::Quit);
        }

        crate::VizualMsg::none()
    }
}
