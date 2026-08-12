pub mod header;
mod settings;

use color_eyre::eyre::Result;
use vizual_macros::display;

use self::header::Header;
use super::{layout::axis::Axis, paper::Paper};
use crate::{
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::{Theme, Theme_choice},
    widget::{
        Focus_provider, Widget, Widget_trait,
        widgets::{
            positioning::anchor::{Anchor, Anchors},
            text::Text,
        },
    },
};

#[derive(Clone)]
pub struct Default_root {
    title: String,
    widget: Widget,
    settings_open: State<bool>,
    theme_choice: State<Theme_choice>,
}

impl Default_root {
    pub fn new(title: impl Into<String>, widget: impl Widget_trait, render: crate::Render) -> Self {
        Self {
            title: title.into(),
            widget: Box::new(widget),
            settings_open: render.new_state(false),
            theme_choice: render.new_state(Theme_choice::System),
        }
    }
}

#[async_trait::async_trait]
impl Widget_trait for Default_root {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let body = Paper::new(self.widget.clone());

        let header = Header::new(
            self.title.clone(),
            self.settings_open.clone(),
            self.theme_choice.clone(),
        );

        let axis = Axis::new(Direction::Vertical, vec![Box::new(header), Box::new(body)]);

        let mut root = Paper::new(axis);
        root.style.set(theme.load().specific.root);

        Ok(vec![display!(root)])
    }
}
