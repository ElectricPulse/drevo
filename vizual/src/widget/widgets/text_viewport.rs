use parley::Layout;

use crate::{
    geometry::{Point, Size},
    graphics::{
        scene::Scene,
        text::{Styled_text, Text_brush, Text_window},
    },
};

// This is complete AI slop.
pub(crate) struct Text_viewport {
    styled: Styled_text,
    layout: Option<Layout<Text_brush>>,
    viewport: Size,
    offset: Point,
}

impl Clone for Text_viewport {
    fn clone(&self) -> Self {
        Self {
            styled: self.styled.clone(),
            // Parley's layout is derived renderer-facing cache, not durable widget state. A clone
            // keeps the text, viewport, and scroll offset, then rebuilds shaping during render.
            layout: None,
            viewport: self.viewport,
            offset: self.offset,
        }
    }
}

impl Text_viewport {
    pub fn new() -> Self {
        Self {
            styled: Styled_text::ansi(""),
            layout: None,
            viewport: Size::default(),
            offset: Point::default(),
        }
    }

    pub fn set_content(&mut self, content: &str) {
        self.styled = Styled_text::ansi(content);
        self.layout = None;
    }

    pub fn prepare(
        &mut self,
        text_context: &mut crate::graphics::text::Text_context,
        viewport: Size,
    ) {
        if self.layout.is_none() {
            self.layout = Some(text_context.build_layout(&self.styled));
        }
        self.viewport = viewport;
        self.clamp_offset();
    }

    pub fn paint(&self, scene: &mut Scene<'_>, origin: Point) {
        let Some(layout) = &self.layout else {
            return;
        };
        scene.paint_layout(
            layout,
            origin,
            Some(Text_window {
                offset: self.offset,
                size: self.viewport,
            }),
            true,
        );
    }

    pub fn content_size(&self) -> Size {
        self.layout.as_ref().map_or_else(Size::default, |layout| {
            Size::new(f64::from(layout.full_width()), f64::from(layout.height()))
        })
    }

    pub fn viewport_size(&self) -> Size {
        self.viewport
    }

    pub fn offset(&self) -> Point {
        self.offset
    }

    pub fn maximum_offset(&self) -> Point {
        let content = self.content_size();
        Point::new(
            (content.width - self.viewport.width).max(0.0),
            (content.height - self.viewport.height).max(0.0),
        )
    }

    pub fn line_step(&self) -> f64 {
        self.layout
            .as_ref()
            .and_then(|layout| layout.lines().next().map(|line| line.metrics().line_height))
            .map(f64::from)
            .unwrap_or(16.0)
    }

    pub fn scroll_x(&mut self, amount: f64) {
        self.offset.x += amount;
        self.clamp_offset();
    }

    pub fn scroll_y(&mut self, amount: f64) {
        self.offset.y += amount;
        self.clamp_offset();
    }

    pub fn jump_to_top(&mut self) {
        self.offset.y = 0.0;
    }

    pub fn jump_to_bottom(&mut self) {
        self.offset.y = self.maximum_offset().y;
    }

    pub fn line_count(&self) -> usize {
        self.layout.as_ref().map_or(0, Layout::len)
    }

    pub fn current_line(&self) -> usize {
        let Some(layout) = &self.layout else {
            return 0;
        };
        layout
            .lines()
            .position(|line| f64::from(line.metrics().block_max_coord) > self.offset.y)
            .unwrap_or_else(|| layout.len().saturating_sub(1))
    }

    pub fn lines_from_end(&self) -> usize {
        let visible_lines = (self.viewport.height / self.line_step()).ceil() as usize;
        self.line_count()
            .saturating_sub(self.current_line().saturating_add(visible_lines))
    }

    fn clamp_offset(&mut self) {
        let maximum = self.maximum_offset();
        self.offset.x = self.offset.x.clamp(0.0, maximum.x);
        self.offset.y = self.offset.y.clamp(0.0, maximum.y);
    }
}

impl Default for Text_viewport {
    fn default() -> Self {
        Self::new()
    }
}
