use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::Children,
    component::context::Component_context,
    config::DEFAULT_FONT_SIZE,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    style::{Color, Style},
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

impl From<Theme> for Text_style {
    fn from(theme: Theme) -> Self {
        theme.specific.text.paragraph
    }
}

#[derive(Clone)]
pub struct Text {
    content: String,
    pub style: Style<Text_style>,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }
}

#[async_trait]
impl Widget_trait for Text {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let size = text_context.measure(&self.content, self.style.get(&theme).size);
        hitbox.set_static_dimension(Direction::Horizontal, size.width);
        hitbox.set_static_dimension(Direction::Vertical, size.height);

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        let _ = display.draw_text(&self.content, hitbox.origin, self.style.get(&theme));
        Ok(None)
    }
}
