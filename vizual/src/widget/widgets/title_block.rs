use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Layout_input, Widget_trait},
    layout::axis::{Axis, Axis_style},
    paper::Paper,
    positioning::anchor::Anchor,
    text::Text,
};
use crate::{
    component::Children,
    geometry::Direction,
    widget::Widget,
};

#[derive(Clone)]
pub struct Title_block {
    child: Widget,
    pub title: String,
}

impl Title_block {
    pub fn new(child: impl Widget_trait, title: impl Into<String>) -> Self {
        Self {
            child: child.as_any(),
            title: title.into(),
        }
    }
}

#[async_trait]
impl Widget_trait for Title_block {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut title = Text::new(self.title.clone());
        title.style.set(theme.specific.text.title);
        let title = Anchor::left(title);
        let child = Anchor::left(self.child.clone());

        let mut axis = Axis::new(Direction::Vertical, (title, child));

        axis.style.set(Axis_style::Gap(theme.units.em * 0.45));

        let paper = Paper::new(axis);
        Ok(vec![display!(paper)])
    }
}
