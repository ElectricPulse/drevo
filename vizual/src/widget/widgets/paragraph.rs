use async_trait::async_trait;
use color_eyre::eyre::Result;
use parley::Layout;

use super::super::{Layout_input, Render_input, Widget_trait};
use crate::{
    geometry::{Direction, Size},
    graphics::text::{Styled_text, Text_brush},
    widget::widgets::text::Text_style,
};

/// Text which wraps to its resolved width and clips any overflow outside its resolved height.
#[derive(Clone)]
pub struct Paragraph {
    content: Styled_text,
    static_direction: Direction,
    static_size: f64,
    cached_layout: Option<Layout<Text_brush>>,
}

impl Paragraph {
    /// Fixes `size` along `direction` and derives the other dimension from the shaped text.
    /// A fixed height chooses the narrowest width that fits whenever fitting is possible.
    pub fn new(direction: Direction, size: f64) -> Self {
        assert!(size.is_finite() && size >= 0.0);
        Self {
            content: Styled_text::styled("", Text_style::default()),
            static_direction: direction,
            static_size: size,
            cached_layout: None,
        }
    }

    pub fn set_styled_content(&mut self, content: impl Into<String>, style: Text_style) {
        self.content = Styled_text::styled(content, style);
        self.cached_layout = None;
    }

    pub fn set_ansi_content(&mut self, content: impl AsRef<str>, font_size: f32) {
        let mut text = Styled_text::ansi(content.as_ref());
        text.size = font_size;
        self.content = text;
        self.cached_layout = None;
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

    fn compute_layout(&self, text_context: &mut crate::graphics::text::Text_context) -> (Size, Layout<Text_brush>) {
        match self.static_direction {
            Direction::Horizontal => {
                let layout =
                    text_context.build_wrapped_layout(&self.content, self.static_size as f32);
                let size = Size::new(self.static_size, f64::from(layout.height()));
                (size, layout)
            }
            Direction::Vertical => {
                let width = self.width_for_height(text_context);
                let layout = text_context.build_wrapped_layout(&self.content, width as f32);
                let size = Size::new(width, self.static_size);
                (size, layout)
            }
        }
    }
}

#[async_trait]
impl Widget_trait for Paragraph {
    async fn layout(
        &mut self,
        Layout_input {
            hitbox,
            problem,
            text_context,
            ..
        }: Layout_input<'_>,
    ) -> Result<crate::component::Children> {
        let (size, layout) = self.compute_layout(text_context);
        self.cached_layout = Some(layout);

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
        Render_input {
            hitbox,
            scene,
            text_context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        if hitbox.size.width > 0.0 && hitbox.size.height > 0.0 {
            if let Some(layout) = &self.cached_layout {
                scene.paint_layout_clipped(layout, hitbox.origin, hitbox, true);
            } else {
                let layout = text_context.build_wrapped_layout(&self.content, hitbox.size.width as f32);
                scene.paint_layout_clipped(&layout, hitbox.origin, hitbox, true);
            }
        }

        Ok(())
    }
}
