use parley::{Layout, PositionedLayoutItem};
use vello::{
    Glyph, Scene as Vello_scene,
    kurbo::{Affine, Line, Rect as Kurbo_rect, RoundedRect, Stroke},
    peniko::Fill,
};

use crate::{
    geometry::{Point, Rect},
    style::Color,
};

use super::text::{Text_brush, Text_window};

enum Decoration {
    Underline,
    Strikethrough,
}

pub struct Scene<'a> {
    pub scene: &'a mut Vello_scene,
}

impl<'a> Scene<'a> {
    pub(crate) fn new(scene: &'a mut Vello_scene) -> Self {
        Self { scene }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.size.is_empty() {
            return;
        }

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &to_kurbo_rect(rect),
        );
    }

    pub fn fill_rounded_rect(&mut self, rect: Rect, color: Color, radius: f64) {
        if rect.size.is_empty() {
            return;
        }

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &RoundedRect::from_rect(to_kurbo_rect(rect), radius),
        );
    }

    pub fn stroke_rounded_rect(&mut self, rect: Rect, color: Color, width: f64, radius: f64) {
        if rect.size.is_empty() || width <= 0.0 {
            return;
        }

        self.scene.stroke(
            &Stroke::new(width),
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &RoundedRect::from_rect(to_kurbo_rect(rect), radius),
        );
    }

    pub(crate) fn paint_layout(
        &mut self,
        layout: &Layout<Text_brush>,
        origin: Point,
        viewport: Option<Text_window>,
        hint: bool,
    ) {
        let scroll = viewport.map(|window| window.offset).unwrap_or_default();
        let transform = Affine::translate((origin.x - scroll.x, origin.y - scroll.y));

        for line in layout.lines() {
            let metrics = line.metrics();
            if let Some(window) = viewport
                && (f64::from(metrics.block_max_coord) < window.offset.y
                    || f64::from(metrics.block_min_coord) > window.offset.y + window.size.height)
            {
                continue;
            }

            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();
                let visible_x =
                    viewport.map(|window| (window.offset.x, window.offset.x + window.size.width));

                if let Some(background) = &style.brush.background {
                    let start = f64::from(glyph_run.offset());
                    let end = f64::from(glyph_run.offset() + glyph_run.advance());
                    let (start, end) = match visible_x {
                        Some((visible_start, visible_end)) => {
                            (start.max(visible_start), end.min(visible_end))
                        }
                        None => (start, end),
                    };

                    if end > start {
                        self.scene.fill(
                            Fill::NonZero,
                            transform,
                            background,
                            None,
                            &Kurbo_rect::new(
                                start,
                                f64::from(metrics.block_min_coord),
                                end,
                                f64::from(metrics.block_max_coord),
                            ),
                        );
                    }
                }

                self.paint_decoration(transform, &glyph_run, visible_x, Decoration::Underline);

                let run = glyph_run.run();
                let synthesis = run.synthesis();
                let glyph_transform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let glyphs = glyph_run.positioned_glyphs().filter_map(|glyph| {
                    let visible = match visible_x {
                        Some((start, end)) => {
                            let glyph_start = f64::from(glyph.x);
                            let glyph_end = glyph_start + f64::from(glyph.advance);
                            glyph_end >= start && glyph_start <= end
                        }
                        None => true,
                    };

                    visible.then_some(Glyph {
                        id: glyph.id,
                        x: glyph.x,
                        y: glyph.y,
                    })
                });

                self.scene
                    .draw_glyphs(run.font())
                    .brush(&style.brush.foreground)
                    .hint(hint)
                    .transform(transform)
                    .glyph_transform(glyph_transform)
                    .font_size(run.font_size())
                    .normalized_coords(run.normalized_coords())
                    .draw(Fill::NonZero, glyphs);

                self.paint_decoration(transform, &glyph_run, visible_x, Decoration::Strikethrough);
            }
        }
    }

    fn paint_decoration(
        &mut self,
        transform: Affine,
        glyph_run: &parley::layout::GlyphRun<'_, Text_brush>,
        visible_x: Option<(f64, f64)>,
        decoration: Decoration,
    ) {
        let style = glyph_run.style();
        let (decoration, default_offset, default_size) = match decoration {
            Decoration::Underline => (
                style.underline.as_ref(),
                glyph_run.run().metrics().underline_offset,
                glyph_run.run().metrics().underline_size,
            ),
            Decoration::Strikethrough => (
                style.strikethrough.as_ref(),
                glyph_run.run().metrics().strikethrough_offset,
                glyph_run.run().metrics().strikethrough_size,
            ),
        };
        let Some(decoration) = decoration else {
            return;
        };
        let offset = decoration.offset.unwrap_or(default_offset);
        let width = decoration.size.unwrap_or(default_size);
        let y = glyph_run.baseline() - offset + width / 2.0;
        let mut start = f64::from(glyph_run.offset());
        let mut end = f64::from(glyph_run.offset() + glyph_run.advance());
        if let Some((visible_start, visible_end)) = visible_x {
            start = start.max(visible_start);
            end = end.min(visible_end);
        }
        if end <= start {
            return;
        }

        self.scene.stroke(
            &Stroke::new(f64::from(width)),
            transform,
            &decoration.brush.foreground,
            None,
            &Line::new((start, f64::from(y)), (end, f64::from(y))),
        );
    }
}

fn to_kurbo_rect(rect: Rect) -> Kurbo_rect {
    Kurbo_rect::new(rect.origin.x, rect.origin.y, rect.right(), rect.bottom())
}
