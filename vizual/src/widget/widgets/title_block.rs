use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{LayoutInput, WidgetTrait},
    layout::axis::{Axis, AxisStyle},
    paper::Paper,
    positioning::anchor::Anchor,
    text::Text,
};
use crate::{component::Children, geometry::Direction, widget::Widget};

#[derive(Clone)]
pub struct TitleBlock {
    child: Widget,
    pub title: String,
}

impl TitleBlock {
    pub fn new(child: impl WidgetTrait, title: impl Into<String>) -> Self {
        Self {
            child: child.as_any(),
            title: title.into(),
        }
    }
}

#[async_trait]
impl WidgetTrait for TitleBlock {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let mut title = Text::new(self.title.clone());
        title.style.set(theme.specific.text.title);
        let title = Anchor::left(title);
        let child = Anchor::left(self.child.clone());

        let mut axis = Axis::new(Direction::Vertical, (title, child));

        axis.style.set(AxisStyle::Gap(theme.units.em * 0.45));

        let paper = Paper::new(axis);
        Ok(vec![display!(paper)])
    }
}
