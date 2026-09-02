use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{
        layout::grid::Grid,
        positioning::anchor::{Anchor, Anchor_position, Anchors},
        text::Text,
    },
    Theme_choice,
    settings::Settings,
};
use crate::{
    component::Children,
    state::Store,
    widget::{Layout_input, Widget_trait},
};

#[derive(Clone)]
pub struct Header {
    name: String,
    choice: Store<Theme_choice>,
}

impl Header {
    pub fn new(name: impl Into<String>, choice: Store<Theme_choice>) -> Self {
        Self {
            name: name.into(),
            choice,
        }
    }
}

#[async_trait]
impl Widget_trait for Header {
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
        let mut name = Text::new(self.name.clone());
        name.style.set(theme.specific.text.title);
        let name = Anchor::new(
            name,
            Anchors {
                horizontal: Some(Anchor_position::Start),
                vertical: Some(Anchor_position::Middle),
            },
        );

        let settings = Settings::new(self.choice.clone());

        let settings = Anchor::new(
            settings,
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: Some(Anchor_position::Start),
            },
        );

        Ok(vec![display!(Grid::new((name, settings), 0.0))])
    }
}
