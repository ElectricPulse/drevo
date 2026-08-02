use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Focus_provider, Widget_trait},
    full::Full,
    layout::{Layout, Style as Layout_style},
    paper::Paper,
    text::Text,
};
use crate::{
    component::{Child, Children, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
};

pub struct Title_block {
    child: Child,
    pub title: String,
    pub theme: State<Theme>,
}

impl Title_block {
    pub fn new(child: Child, title: impl Into<String>, theme: State<Theme>) -> Self {
        Self {
            child,
            title: title.into(),
            theme,
        }
    }
}

#[async_trait]
impl Widget_trait for Title_block {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let style = self.theme.project(|theme| &theme.specific.text.title);
        let title = Text::new(self.title.clone(), style);

        let layout = Layout::new(
            Direction::Vertical,
            vec![display!(title), self.child.clone()],
            Layout_style::Gap(self.theme.load().units.em * 0.45),
            Objective::default(),
            2,
        );

        let paper = Paper::new(display!(layout), (&self.theme).into());
        let full = Full::new(display!(paper));

        Ok(vec![display!(full)])
    }
}
