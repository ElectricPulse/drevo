use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::super::{Focus_provider, Widget_trait};
use crate::{
    geometry::{Direction, Rect, Size},
    graphics::{scene::Scene, text::Styled_text},
    layouter::hitbox::Hitbox,
    widget::widgets::text::Text_style,
};

/// Text which wraps to its resolved width and clips any overflow outside its resolved height.
#[derive(Clone)]
pub struct Paragraph {
    content: Styled_text,
    static_direction: Direction,
    static_size: f64,
}

impl Paragraph {
    /// Fixes `size` along `direction` and derives the other dimension from the shaped text.
    /// A fixed height chooses the narrowest width that fits whenever fitting is possible.
    pub fn new(direction: Direction, size: f64) -> Self {
        assert!(size.is_finite() && size >= 0.0);
        Self {
            content: Styled_text::ansi(""),
            static_direction: direction,
            static_size: size,
        }
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        let content = content.into();
        self.content = Styled_text::ansi(&content);
    }

    pub fn set_styled_content(&mut self, content: impl Into<String>, style: Text_style) {
        self.content = Styled_text::styled(content, style);
    }

    fn width_for_height(&self, text_context: &mut crate::graphics::text::Text_context) -> f64 {
        let unwrapped = text_context.build_layout(&self.content);
        let natural_width = f64::from(unwrapped.full_width());
        if natural_width <= 0.0 || f64::from(unwrapped.height()) > self.static_size {
            return natural_width;
        }

        let mut minimum = 0.0;
        let mut maximum = natural_width;
        for _ in 0..16 {
            if maximum - minimum <= 0.25 {
                break;
            }

            let candidate = (minimum + maximum) / 2.0;
            let layout = text_context.build_wrapped_layout(&self.content, candidate as f32);
            let fits = f64::from(layout.height()) <= self.static_size
                && f64::from(layout.full_width()) <= candidate;
            if fits {
                maximum = candidate;
            } else {
                minimum = candidate;
            }
        }

        let layout = text_context.build_wrapped_layout(&self.content, maximum as f32);
        maximum.max(f64::from(layout.full_width()))
    }

    fn size(&self, text_context: &mut crate::graphics::text::Text_context) -> Size {
        match self.static_direction {
            Direction::Horizontal => {
                let layout =
                    text_context.build_wrapped_layout(&self.content, self.static_size as f32);
                Size::new(self.static_size, f64::from(layout.height()))
            }
            Direction::Vertical => Size::new(self.width_for_height(text_context), self.static_size),
        }
    }
}

#[async_trait]
impl Widget_trait for Paragraph {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: crate::component::context::Component_context,
        text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut crate::slot::manager::Slots,
    ) -> Result<crate::component::Children> {
        let size = self.size(text_context);
        for (direction, size) in [
            (Direction::Horizontal, size.width),
            (Direction::Vertical, size.height),
        ] {
            hitbox
                .set_static_dimension(&problem, direction, size)
                .await?;
        }

        Ok(vec![])
    }

    async fn render(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::Store<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        if hitbox.size.width > 0.0 && hitbox.size.height > 0.0 {
            let layout = text_context.build_wrapped_layout(&self.content, hitbox.size.width as f32);
            scene.paint_layout_clipped(&layout, hitbox.origin, hitbox, true);
        }

        Ok(None)
    }
}
