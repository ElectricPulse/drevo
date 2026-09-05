pub mod header;
mod settings;

use crate::macros::display;
use color_eyre::eyre::Result;

use self::header::Header;
use super::{block::Block, layout::axis, paper::Paper};
use crate::{
    component::Children,
    geometry::Direction,
    state::Store,
    widget::{LayoutInput, Widget, WidgetTrait},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeChoice {
    System,
    Dark,
    Light,
}

#[derive(Clone)]
pub struct DefaultRoot {
    title: String,
    widget: Widget,
    theme_choice: Store<ThemeChoice>,
}

impl DefaultRoot {
    pub fn new(title: impl Into<String>, widget: impl WidgetTrait) -> Self {
        Self {
            title: title.into(),
            widget: widget.as_any(),
            theme_choice: Store::new(ThemeChoice::System),
        }
    }
}

#[async_trait::async_trait]
impl WidgetTrait for DefaultRoot {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme_value = theme.affect(relayout).await?;
        let body = Paper::new(self.widget.clone()).style(theme_value.specific.body);

        let header = Header::new(self.title.clone(), self.theme_choice.clone());
        let header = Block::new(header, theme_value.specific.header);

        let root = axis::Axis::new(Direction::Vertical, (header, body))
            .style(axis::AxisStyle::Gap(0.0));

        Ok(vec![display!(root)])
    }
}
