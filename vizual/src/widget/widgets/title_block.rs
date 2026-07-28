use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Control, Focus_provider, Renderable, Widget_type},
    layout::{Layout, Style as Layout_style},
    paper::Paper,
    text::Text,
};
use crate::{
    component::Child,
    hitbox::{Direction, Hitbox},
    layouter::{Problem_context, constraints::Objective},
    slot_manager::Slots,
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

impl Control for Title_block {}

#[async_trait]
impl Renderable for Title_block {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Problem_context,
        slots: &mut Slots,
    ) -> Result<Widget_type> {
        let title =
            Text::new(self.title.clone()).set_style(self.theme.load().semantic.text.title());

        let layout = Layout::new(
            Direction::Vertical,
            vec![Some(display!(title)), Some(self.child.clone())],
            Layout_style::Gap(self.theme.load().units.em * 0.45),
            Objective::default(),
            2,
        );

        let paper = Paper::new(display!(layout), self.theme.clone());

        Ok(Widget_type::Virtual(Box::new(paper)))
    }
}
