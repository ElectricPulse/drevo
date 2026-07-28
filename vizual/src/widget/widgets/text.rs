use async_trait::async_trait;
use color_eyre::Result;
use good_lp::constraint;

use super::super::{Control, Focus_provider, Renderable, Widget_type};
use crate::{
    backend::graphics::Paint_context,
    config::DEFAULT_FONT_SIZE,
    geometry::Rect,
    hitbox::{Direction, Hitbox},
    layouter::Problem_context,
    slot_manager::Slots,
    style::Color,
};

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

impl Control for Text {}

#[async_trait]
impl Renderable for Text {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let size = problem.measure_text(&self.content, self.style.size).await?;
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

        Ok(Widget_type::Visual(Vec::new()))
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        let _ = paint.draw_text(&self.content, hitbox.origin, self.style);
        Ok(None)
    }
}
