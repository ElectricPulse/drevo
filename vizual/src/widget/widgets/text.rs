use super::super::{Control, Focus_provider, Widget_trait};
use crate::{
    component::Children,
    component::context::Component_context,
    config::DEFAULT_FONT_SIZE,
    constraint,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    style::Color,
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

pub struct Text {
    content: String,
    pub style: Text_style,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            style: Text_style::default(),
        }
    }

    pub fn set_style(mut self, style: Text_style) -> Self {
        self.style = style;
        self
    }

    pub fn set_color(mut self, color: Color) -> Self {
        self.style.color = color;
        self
    }
}



#[async_trait]
impl Widget_trait for Text {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let size = text_context.measure(&self.content, self.style.size);
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
        let _ = display.draw_text(&self.content, hitbox.origin, self.style);
        Ok(None)
    }
}
