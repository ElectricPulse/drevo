pub mod header;
mod settings;

use color_eyre::eyre::Result;
use crate::macros::display;

use self::header::Header;
use super::{layout::axis::Axis, paper::Paper};
use crate::{
    component::Children,
    geometry::Direction,
    state::Store,
    widget::{Layout_input, Widget, Widget_trait},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme_choice {
    System,
    Dark,
    Light,
}

#[derive(Clone)]
pub struct Default_root {
    title: String,
    widget: Widget,
    theme_choice: Store<Theme_choice>,
}

impl Default_root {
    pub fn new(title: impl Into<String>, widget: impl Widget_trait) -> Self {
        Self {
            title: title.into(),
            widget: widget.as_any(),
            theme_choice: Store::new(Theme_choice::System),
        }
    }
}

#[async_trait::async_trait]
impl Widget_trait for Default_root {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme_value = theme.affect(render).await?;
        let mut body = Paper::new(self.widget.clone());
        body.style.set(theme_value.specific.body);

        let header = Header::new(self.title.clone(), self.theme_choice.clone());

        let axis = Axis::new(Direction::Vertical, (header, body));

        let mut root = Paper::new(axis);
        root.style.set(theme_value.specific.root);

        Ok(vec![display!(root)])
    }
}
