use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{
        layout::grid::Grid,
        positioning::anchor::{Anchor, Anchors, Position},
        text::Text,
    },
    Theme_choice,
    settings::Settings,
};
use crate::{
    Render,
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::{Focus_provider, Widget, Widget_trait},
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
        render: Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut name = Text::new(self.name.clone());
        name.style.set(theme.specific.text.title);
        let name = Anchor::new(
            name,
            Anchors {
                horizontal: Some(Position::Start),
                vertical: Some(Position::Middle),
            },
        );
        let settings = Settings::new(self.choice.clone());

        let settings = Anchor::new(
            settings,
            Anchors {
                horizontal: Some(Position::End),
                vertical: Some(Position::Middle),
            },
        );

        let items: Vec<Widget> = vec![Box::new(name), Box::new(settings)];
        Ok(vec![display!(Grid::new(items, 0.0))])
    }
}
