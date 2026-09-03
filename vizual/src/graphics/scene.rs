use parley::{Layout, PositionedLayoutItem};
use vello::{
    Glyph, Scene as VelloScene,
    kurbo::{Affine, Line, Rect as KurboRect, RoundedRect, Stroke},
    peniko::Fill,
};

use crate::{
    geometry::{Point, Rect},
    style::Color,
};

use super::text::TextBrush;

enum Decoration {
    Underline,
    Strikethrough,
}

pub struct Scene<'a> {
    pub scene: &'a mut VelloScene,
}

impl<'a> Scene<'a> {
    pub(crate) fn new(scene: &'a mut VelloScene) -> Self {
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

    pub(crate) fn append_clipped(
        &mut self,
        scene: &VelloScene,
        viewport: Rect,
        transform: Affine,
    ) {
        let viewport = to_kurbo_rect(viewport);
        self.scene
            .push_clip_layer(Fill::NonZero, Affine::IDENTITY, &viewport);
        self.scene.append(scene, Some(transform));
        self.scene.pop_layer();
    }

    pub(crate) fn paint_layout(&mut self, layout: &Layout<TextBrush>, origin: Point, hint: bool) {
        let transform = Affine::translate((origin.x, origin.y));

        for line in layout.lines() {
            let metrics = line.metrics();

            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();

                if let Some(background) = &style.brush.background {
                    let start = f64::from(glyph_run.offset());
                    let end = f64::from(glyph_run.offset() + glyph_run.advance());

                    if end > start {
                        self.scene.fill(
                            Fill::NonZero,
                            transform,
                            background,
                            None,
                            &KurboRect::new(
                                start,
                                f64::from(metrics.block_min_coord),
                                end,
                                f64::from(metrics.block_max_coord),
                            ),
                        );
                    }
                }

                self.paint_decoration(transform, &glyph_run, None, Decoration::Underline);

                let run = glyph_run.run();
                let synthesis = run.synthesis();
                let glyph_transform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));
                let glyphs = glyph_run.positioned_glyphs().map(|glyph| Glyph {
                    id: glyph.id,
                    x: glyph.x,
                    y: glyph.y,
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

                self.paint_decoration(transform, &glyph_run, None, Decoration::Strikethrough);
            }
        }
    }

    pub(crate) fn paint_layout_clipped(
        &mut self,
        layout: &Layout<TextBrush>,
        origin: Point,
        viewport: Rect,
        hint: bool,
    ) {
        if viewport.size.is_empty() {
            return;
        }

        let viewport = to_kurbo_rect(viewport);
        self.scene
            .push_clip_layer(Fill::NonZero, Affine::IDENTITY, &viewport);
        self.paint_layout(layout, origin, hint);
        self.scene.pop_layer();
    }

    fn paint_decoration(
        &mut self,
        transform: Affine,
        glyph_run: &parley::layout::GlyphRun<'_, TextBrush>,
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

fn to_kurbo_rect(rect: Rect) -> KurboRect {
    KurboRect::new(rect.origin.x, rect.origin.y, rect.right(), rect.bottom())
}
