use async_trait::async_trait;
use color_eyre::eyre::Result;
use parley::Layout;

use super::super::{Focus_provider, Widget_trait};
use crate::{
    geometry::{Rect, Size},
    graphics::{
        scene::Scene,
        text::{Styled_text, Text_brush, Text_context},
    },
    layouter::hitbox::Hitbox,
};

/// Text which wraps to its resolved width and renders only when it fits its resolved height.
#[derive(Clone)]
pub struct Paragraph {
    content: Styled_text,
    lines: Option<usize>,
}

impl Paragraph {
    pub fn new() -> Self {
        Self {
            content: Styled_text::ansi(""),
            lines: Some(2),
        }
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        let content = content.into();
        self.content = Styled_text::ansi(&content);
    }

    pub fn set_lines(&mut self, lines: Option<usize>) {
        self.lines = lines;
    }

    fn fit(&self, text_context: &mut Text_context, size: Size) -> Option<Layout<Text_brush>> {
        if size.width <= 0.0 || size.height <= 0.0 {
            return None;
        }

        let layout = text_context.build_wrapped_layout(&self.content, size.width as f32);
        let fits = f64::from(layout.full_width()) <= size.width
            && f64::from(layout.height()) <= size.height;
        fits.then_some(layout)
    }
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::new()
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
        match self.lines {
            Some(lines) => {
                let layout = text_context.build_layout(&self.content);
                let line_height = layout
                    .lines()
                    .next()
                    .map(|line| f64::from(line.metrics().line_height))
                    .unwrap_or_default();
                hitbox
                    .set_static_dimension(
                        &problem,
                        crate::geometry::Direction::Vertical,
                        line_height * lines as f64,
                    )
                    .await?;
            }
            None => {
                // Normally one would constrain the paragraph to at least width * height == number
                // of characters, but even that excludes the possibility that sometimes a stray
                // line ending (-afa) wrapping might get added, &c. That's why if the text doesn't
                // fit it isn't rendered and in the future maybe a system where the layouter asks:
                // can you do this size, can you do this size, &c. is implemented.
            }
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
        if let Some(layout) = self.fit(text_context, hitbox.size) {
            scene.paint_layout(&layout, hitbox.origin, true);
        }

        Ok(None)
    }
}
