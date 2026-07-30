use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use super::{
    super::{Control, Focus_provider, Widget_trait, Widget_type},
    layout::{Layout, Style as Layout_style},
    paper::Paper,
    text::Text,
};
use crate::{
    component::{Shared_component, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
};

pub struct Title_block {
    child: Shared_component,
    pub title: String,
    pub theme: State<Theme>,
}

impl Title_block {
    pub fn new(child: Shared_component, title: impl Into<String>, theme: State<Theme>) -> Self {
        Self {
            child,
            title: title.into(),
            theme,
        }
    }
}

impl Control for Title_block {}

#[async_trait]
impl Widget_trait for Title_block {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
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
