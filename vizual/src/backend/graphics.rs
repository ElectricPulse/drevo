use std::ops::Range;

use parley::{
    Alignment, AlignmentOptions, FontContext, FontStyle, FontWeight, GenericFamily, Layout,
    LayoutContext, PositionedLayoutItem, StyleProperty,
};
use vello::{
    Glyph, Scene,
    kurbo::{Affine, Line, Rect as Kurbo_rect, RoundedRect, Stroke},
    peniko::{Brush, Fill},
};

use crate::{
    config::DEFAULT_FONT_SIZE,
    geometry::{Point, Rect, Size},
    style::Color,
    widget::widgets::text::Text_style,
};

// TODO:
// DISCLAIMER:
// Originally this was a tui library, but later I let codex convert it to wgpu
// almost all of the resulting slop is here

#[derive(Clone, Debug, PartialEq)]
pub struct Text_brush {
    pub foreground: Brush,
    pub background: Option<Brush>,
}

impl Default for Text_brush {
    fn default() -> Self {
        Self {
            foreground: Brush::Solid(Color::White.to_peniko()),
            background: None,
        }
    }
}

pub struct Text_resources {
    pub font_context: FontContext,
    pub layout_context: LayoutContext<Text_brush>,
}

impl Text_resources {
    pub fn new() -> Self {
        Self {
            font_context: FontContext::new(),
            layout_context: LayoutContext::new(),
        }
    }

    pub(crate) fn build_layout(&mut self, text: &Styled_text) -> Layout<Text_brush> {
        build_layout(&mut self.font_context, &mut self.layout_context, text)
    }

    pub(crate) fn measure(&mut self, content: &str, font_size: f32) -> Size {
        let layout = self.build_layout(&Styled_text::styled(
            content,
            Text_style {
                size: font_size,
                color: Color::White,
            },
        ));
        Size::new(f64::from(layout.full_width()), f64::from(layout.height()))
    }
}

impl Default for Text_resources {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Paint_context<'a> {
    pub scene: &'a mut Scene,
    pub font_context: &'a mut FontContext,
    pub layout_context: &'a mut LayoutContext<Text_brush>,
}

impl<'a> Paint_context<'a> {
    pub(crate) fn new(scene: &'a mut Scene, resources: &'a mut Text_resources) -> Self {
        Self {
            scene,
            font_context: &mut resources.font_context,
            layout_context: &mut resources.layout_context,
        }
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

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        if rect.size.is_empty() || width <= 0.0 {
            return;
        }

        self.scene.stroke(
            &Stroke::new(width),
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

    pub fn draw_text(&mut self, content: &str, origin: Point, style: Text_style) -> Size {
        let styled = Styled_text::styled(content, style);
        let layout = build_layout(self.font_context, self.layout_context, &styled);
        let size = Size::new(f64::from(layout.full_width()), f64::from(layout.height()));
        self.paint_layout(&layout, origin, None);
        size
    }

    pub fn measure_text(&mut self, content: &str) -> Size {
        let styled = Styled_text::plain(content, Color::White);
        let layout = build_layout(self.font_context, self.layout_context, &styled);
        Size::new(f64::from(layout.full_width()), f64::from(layout.height()))
    }

    pub(crate) fn build_layout(&mut self, text: &Styled_text) -> Layout<Text_brush> {
        build_layout(self.font_context, self.layout_context, text)
    }

    pub(crate) fn paint_layout(
        &mut self,
        layout: &Layout<Text_brush>,
        origin: Point,
        viewport: Option<Text_window>,
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

                paint_decoration(
                    self.scene,
                    transform,
                    &glyph_run,
                    visible_x,
                    Decoration::Underline,
                );

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
                    .hint(true)
                    .transform(transform)
                    .glyph_transform(glyph_transform)
                    .font_size(run.font_size())
                    .normalized_coords(run.normalized_coords())
                    .draw(Fill::NonZero, glyphs);

                paint_decoration(
                    self.scene,
                    transform,
                    &glyph_run,
                    visible_x,
                    Decoration::Strikethrough,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Text_window {
    pub offset: Point,
    pub size: Size,
}

#[derive(Clone)]
pub(crate) struct Styled_text {
    pub content: String,
    size: f32,
    spans: Vec<Styled_span>,
}

impl Styled_text {
    pub(crate) fn plain(content: impl Into<String>, color: Color) -> Self {
        Self::styled(
            content,
            Text_style {
                size: DEFAULT_FONT_SIZE,
                color,
            },
        )
    }

    pub(crate) fn styled(content: impl Into<String>, style: Text_style) -> Self {
        let content = content.into();
        let length = content.len();
        Self {
            content,
            size: style.size,
            spans: vec![Styled_span {
                range: 0..length,
                style: Ansi_style {
                    foreground: style.color,
                    ..Ansi_style::default()
                },
            }],
        }
    }

    pub(crate) fn ansi(content: &str) -> Self {
        parse_ansi(content)
    }
}

#[derive(Clone)]
struct Styled_span {
    range: Range<usize>,
    style: Ansi_style,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ansi_style {
    foreground: Color,
    background: Option<Color>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    reverse: bool,
    hidden: bool,
}

impl Default for Ansi_style {
    fn default() -> Self {
        Self {
            foreground: Color::White,
            background: None,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            strikethrough: false,
            reverse: false,
            hidden: false,
        }
    }
}

fn build_layout(
    font_context: &mut FontContext,
    layout_context: &mut LayoutContext<Text_brush>,
    text: &Styled_text,
) -> Layout<Text_brush> {
    let mut builder = layout_context.ranged_builder(font_context, &text.content, 1.0, false);
    builder.push_default(GenericFamily::SansSerif);
    builder.push_default(StyleProperty::FontSize(text.size));
    builder.push_default(StyleProperty::Brush(Text_brush::default()));

    for span in &text.spans {
        if span.range.is_empty() {
            continue;
        }

        let (foreground, background) = resolved_colors(span.style);
        builder.push(
            StyleProperty::Brush(Text_brush {
                foreground: Brush::Solid(foreground.to_peniko()),
                background: background.map(|color| Brush::Solid(color.to_peniko())),
            }),
            span.range.clone(),
        );
        if span.style.bold {
            builder.push(
                StyleProperty::FontWeight(FontWeight::new(700.0)),
                span.range.clone(),
            );
        }
        if span.style.italic {
            builder.push(
                StyleProperty::FontStyle(FontStyle::Italic),
                span.range.clone(),
            );
        }
        if span.style.underline {
            builder.push(StyleProperty::Underline(true), span.range.clone());
        }
        if span.style.strikethrough {
            builder.push(StyleProperty::Strikethrough(true), span.range.clone());
        }
    }

    let mut layout = builder.build(&text.content);
    layout.break_all_lines(None);
    layout.align(Alignment::Start, AlignmentOptions::default());
    layout
}

fn resolved_colors(style: Ansi_style) -> (Color, Option<Color>) {
    let mut foreground = match style.hidden {
        true => style.background.unwrap_or(Color::Black),
        false => style.foreground,
    };
    let mut background = style.background;

    if style.reverse {
        let prior_foreground = foreground;
        foreground = background.unwrap_or(Color::Black);
        background = Some(prior_foreground);
    }

    if style.dim && foreground == Color::White {
        foreground = Color::Gray;
    }

    (foreground, background)
}

fn parse_ansi(input: &str) -> Styled_text {
    let mut content = String::new();
    let mut spans = Vec::new();
    let mut style = Ansi_style::default();
    let mut segment_start = 0;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        if character != '\u{1b}' || characters.peek() != Some(&'[') {
            content.push(character);
            continue;
        }

        let _ = characters.next();
        let mut sequence = String::new();
        let mut terminator = None;
        for character in characters.by_ref() {
            if character.is_ascii_alphabetic() {
                terminator = Some(character);
                break;
            }
            sequence.push(character);
        }

        if terminator != Some('m') {
            continue;
        }

        let end = content.len();
        if end > segment_start {
            spans.push(Styled_span {
                range: segment_start..end,
                style,
            });
        }
        apply_sgr(&mut style, &sequence);
        segment_start = end;
    }

    if content.len() > segment_start {
        spans.push(Styled_span {
            range: segment_start..content.len(),
            style,
        });
    }

    Styled_text {
        content,
        size: DEFAULT_FONT_SIZE,
        spans,
    }
}

fn apply_sgr(style: &mut Ansi_style, sequence: &str) {
    let values = match sequence.is_empty() {
        true => vec![0],
        false => sequence
            .split(';')
            .map(|value| value.parse::<u16>().unwrap_or_default())
            .collect(),
    };
    let mut index = 0;

    while let Some(value) = values.get(index).copied() {
        match value {
            0 => *style = Ansi_style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            8 => style.hidden = true,
            9 => style.strikethrough = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            28 => style.hidden = false,
            29 => style.strikethrough = false,
            30..=37 => style.foreground = Color::Indexed((value - 30) as u8),
            38 => index += apply_extended_color(&values[index..], &mut style.foreground),
            39 => style.foreground = Color::White,
            40..=47 => style.background = Some(Color::Indexed((value - 40) as u8)),
            48 => {
                let mut background = style.background.unwrap_or(Color::Black);
                index += apply_extended_color(&values[index..], &mut background);
                style.background = Some(background);
            }
            49 => style.background = None,
            90..=97 => style.foreground = Color::Indexed((value - 90 + 8) as u8),
            100..=107 => style.background = Some(Color::Indexed((value - 100 + 8) as u8)),
            _ => {}
        }
        index += 1;
    }
}

fn apply_extended_color(values: &[u16], color: &mut Color) -> usize {
    match values {
        [_, 5, index, ..] => {
            *color = Color::Indexed((*index).min(255) as u8);
            2
        }
        [_, 2, red, green, blue, ..] => {
            *color = Color::Rgb(
                (*red).min(255) as u8,
                (*green).min(255) as u8,
                (*blue).min(255) as u8,
            );
            4
        }
        _ => 0,
    }
}

enum Decoration {
    Underline,
    Strikethrough,
}

fn paint_decoration(
    scene: &mut Scene,
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

    scene.stroke(
        &Stroke::new(f64::from(width)),
        transform,
        &decoration.brush.foreground,
        None,
        &Line::new((start, f64::from(y)), (end, f64::from(y))),
    );
}

fn to_kurbo_rect(rect: Rect) -> Kurbo_rect {
    Kurbo_rect::new(rect.origin.x, rect.origin.y, rect.right(), rect.bottom())
}
