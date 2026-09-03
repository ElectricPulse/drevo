use crate::{VizualMsg, event::PointerButton, macros::display, widget::MouseEvent};
use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{
        layout::grid::Grid,
        positioning::anchor::{Anchor, AnchorPosition, Anchors},
        text::Text,
    },
    ThemeChoice,
    settings::Settings,
};
use crate::{
    component::Children,
    state::Store,
    widget::{LayoutInput, WidgetTrait},
};

#[derive(Clone)]
pub struct Header {
    name: String,
    choice: Store<ThemeChoice>,
}

impl Header {
    pub fn new(name: impl Into<String>, choice: Store<ThemeChoice>) -> Self {
        Self {
            name: name.into(),
            choice,
        }
    }
}

#[async_trait]
impl WidgetTrait for Header {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            focus,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);

        let theme = theme.affect(relayout).await?;
        let mut name = Text::new(self.name.clone());
        name.style.set(theme.specific.text.title);
        let name = Anchor::new(
            name,
            Anchors {
                horizontal: Some(AnchorPosition::Start),
                vertical: Some(AnchorPosition::Middle),
            },
        );

        let settings = Settings::new(self.choice.clone());

        let settings = Anchor::new(
            settings,
            Anchors {
                horizontal: Some(AnchorPosition::End),
                vertical: Some(AnchorPosition::Start),
            },
        );

        Ok(vec![display!(Grid::new((name, settings), 0.0))])
    }

    async fn on_mouse_click(&mut self, input: MouseEvent<'_>) -> Result<VizualMsg> {
        if input.mouse.button == PointerButton::Primary
            && let Some(window) = input.window
        {
            let _ = window.drag_window();
        }

        VizualMsg::none()
    }
}
