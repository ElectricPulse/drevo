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
    geometry::Direction,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::{Focus_provider, Layout_input, Widget, Widget_trait},
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
            hitbox,
            problem,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(render).await?;
        let mut name = Text::new(self.name.clone());
        name.style.set(theme.specific.text.title);
        /*let name = Anchor::new(
            name,
            Anchors {
                horizontal: Some(Position::Start),
                vertical: Some(Position::Middle),
            },
        );*/

        let name = Anchor::left(name);

        let settings = Settings::new(self.choice.clone());

        let settings = Anchor::new(
            settings,
            Anchors {
                horizontal: Some(Position::End),
                vertical: Some(Position::Start),
            },
        );

        let items: Vec<Widget> = vec![Box::new(name), Box::new(settings)];

        // The whole thing is encased in a grid nothing is teoretically stopping the grid from becoming taller
        // this shrink wraps it to the smallest size
        //problem.minimize(hitbox.get_dimension(Direction::Vertical), 1).await?;

        Ok(vec![display!(Grid::new(items, 0.0))])
    }
}
