use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual::{
    component::{Children, context::Component_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme_manager,
    widget::{
        Focus_provider, Widget_trait,
        widgets::{
            anchor::{Anchor, Anchors, Position},
            text::Text,
        },
    },
};
use vizual_macros::display;

use super::theme_picker::Theme_picker;

pub struct Header {
    name: String,
    open: State<bool>,
    themes: Theme_manager,
}

impl Header {
    pub fn new(name: impl Into<String>, open: State<bool>, themes: Theme_manager) -> Self {
        Self {
            name: name.into(),
            open,
            themes,
        }
    }
}

#[async_trait]
impl Widget_trait for Header {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let name = Text::new(
            self.name.clone(),
            self.themes
                .theme
                .project(|theme| &theme.specific.text.title),
        );
        let name = Anchor::center(display!(name));
        let settings = Theme_picker::new(self.open.clone(), self.themes.clone());
        let settings = Anchor::new(
            display!(settings),
            Anchors {
                horizontal: Some(Position::End),
                vertical: Some(Position::Middle),
            },
        );

        Ok(vec![display!(name), display!(settings)])
    }
}
