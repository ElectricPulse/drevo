use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::{display, position};

use super::{
    super::{Focus_provider, Widget_trait},
    layout::{Layout, Layout_style},
    paper::Paper,
    positioning::anchor::{Anchor, Anchors},
    text::Text,
};
use crate::{
    component::{Child, Children, context::Component_context},
    geometry::Direction,
    layouter::{hitbox::Hitbox, objective::Objective},
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::widgets::container::Container,
};

#[derive(Clone)]
pub struct Title_block {
    child: Child,
    pub title: String,
}

impl Title_block {
    pub fn new(child: Child, title: impl Into<String>) -> Self {
        Self {
            child,
            title: title.into(),
        }
    }
}

#[async_trait]
impl Widget_trait for Title_block {
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
        let mut title = Text::new(self.title.clone());
        title.style.set(theme.load().specific.text.title);
        let title = Anchor::new(title, Anchors::top_left());
        let title = position!(title);
        let child = display!(self.child.clone());

        let mut layout = Layout::new(
            Direction::Vertical,
            vec![title, child],
            Objective::default(),
            2,
        );

        layout
            .style
            .set(Layout_style::Gap(theme.load().units.em * 0.45));

        let paper = Paper::new(display!(layout));
        Ok(vec![display!(paper)])
    }
}
