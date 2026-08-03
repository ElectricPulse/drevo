use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::Children,
    component::context::Component_context,
    config::DEFAULT_FONT_SIZE,
    constraint,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    style::Color,
    theme::Theme,
};
use async_trait::async_trait;
use color_eyre::Result;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Text_style {
    pub size: f32,
    pub color: Color,
}

impl Default for Text_style {
    fn default() -> Self {
        Self {
            size: DEFAULT_FONT_SIZE,
            color: Color::White,
        }
    }
}

impl From<&State<Theme>> for State<Text_style> {
    fn from(theme: &State<Theme>) -> Self {
        theme.project(|theme| &theme.specific.text.paragraph)
    }
}

pub struct Text {
    content: String,
    pub style: State<Text_style>,
}

impl Text {
    pub fn new(content: impl Into<String>, style: State<Text_style>) -> Self {
        Self {
            content: content.into(),
            style,
        }
    }
}

#[async_trait]
impl Widget_trait for Text {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let size = text_context.measure(&self.content, self.style.load().size);
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Horizontal) == size.width
            ))
            .await?;
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) == size.height
            ))
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        let _ = display.draw_text(&self.content, hitbox.origin, *self.style.load());
        Ok(None)
    }
}
