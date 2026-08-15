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
    Render, component::{Children, context::Component_context}, geometry::Direction, layouter::hitbox::Hitbox, slot::manager::Slots, state::{State, Store}, theme::Theme, widget::{Focus_provider, Widget, Widget_trait},
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
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
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

        // We don't want to leave nothing in header component as height definining as it isn't clear what will always be bigger
        // Example currently an application name and settings cog is shown
        // If you unanchor the name then if it were to happen that the settings cog be bigger than the name - then the app will crash
        // so it's easiest to just minimize height after anchoring

        problem.minimize(hitbox.get_dimension(Direction::Horizontal));

        
        Ok(vec![display!(Grid::new(items, 0.0))])
    }
}
