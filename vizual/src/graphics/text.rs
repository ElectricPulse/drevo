use std::{ops::Range, sync::Arc};

use lucide_icons::{Icon as Lucide_icon, LUCIDE_FONT_BYTES};
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontStyle, FontWeight, GenericFamily,
    Layout, LayoutContext, PositionedLayoutItem, StyleProperty, fontique::Blob,
};
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size as Font_size},
};
use vello::{kurbo::Rect as Kurbo_rect, peniko::Brush};

use crate::{
    config::DEFAULT_FONT_SIZE,
    geometry::{Point, Size},
    style::Color,
    widget::widgets::text::Text_style,
};

use super::scene::Scene as Graphics_scene;

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

pub struct Text_context {
    font_context: FontContext,
    layout_context: LayoutContext<Text_brush>,
}

impl Text_context {
    pub fn new() -> Self {
        let mut font_context = FontContext::new();
        let _ = font_context
            .collection
            .register_fonts(Blob::new(Arc::new(LUCIDE_FONT_BYTES)), None);

        Self {
            font_context,
            layout_context: LayoutContext::new(),
        }
    }

    pub(crate) fn build_layout(&mut self, text: &Styled_text) -> Layout<Text_brush> {
        self.build_layout_with_width(text, None)
    }

    pub(crate) fn build_wrapped_layout(
        &mut self,
        text: &Styled_text,
        width: f32,
    ) -> Layout<Text_brush> {
        self.build_layout_with_width(text, Some(width))
    }

    fn build_layout_with_width(
        &mut self,
        text: &Styled_text,
        width: Option<f32>,
    ) -> Layout<Text_brush> {
        let mut builder =
            self.layout_context
                .ranged_builder(&mut self.font_context, &text.content, 1.0, false);
        match text.font {
            Text_font::Sans_serif => builder.push_default(GenericFamily::SansSerif),
            Text_font::Lucide => builder.push_default(FontFamily::named("lucide")),
        }
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
        layout.break_all_lines(width);
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout
    }

    pub fn draw_text(
        &mut self,
        scene: &mut Graphics_scene<'_>,
        content: &str,
        origin: Point,
        style: Text_style,
    ) -> Size {
        let layout = self.build_layout(&Styled_text::styled(content, style));
        let size = Size::new(f64::from(layout.full_width()), f64::from(layout.height()));
        scene.paint_layout(&layout, origin, true);
        size
    }

    pub fn draw_ansi_text(
        &mut self,
        scene: &mut Graphics_scene<'_>,
        content: &str,
        origin: Point,
        font_size: f32,
    ) -> Size {
        let mut text = Styled_text::ansi(content);
        text.size = font_size;
        let layout = self.build_layout(&text);
        let size = Size::new(f64::from(layout.full_width()), f64::from(layout.height()));
        scene.paint_layout(&layout, origin, true);
        size
    }

    pub(crate) fn draw_icon(
        &mut self,
        scene: &mut Graphics_scene<'_>,
        icon: Lucide_icon,
        origin: Point,
        style: Text_style,
    ) -> Size {
        let layout = self.build_layout(&Styled_text::icon(icon, style));
        let bounds = icon_ink_bounds(&layout, icon, style.size);
        let size = bounds.map_or_else(
            || Size::new(f64::from(layout.full_width()), f64::from(layout.height())),
            |bounds| Size::new(bounds.width(), bounds.height()),
        );
        let origin = bounds.map_or(origin, |bounds| {
            Point::new(origin.x - bounds.x0, origin.y - bounds.y0)
        });
        scene.paint_layout(&layout, origin, false);
        size
    }

    pub fn measure_text(&mut self, content: &str) -> Size {
        let layout = self.build_layout(&Styled_text::plain(content, Color::White));
        Size::new(f64::from(layout.full_width()), f64::from(layout.height()))
    }

    pub fn measure(&mut self, content: &str, font_size: f32) -> Size {
        let layout = self.build_layout(&Styled_text::styled(
            content,
            Text_style {
                size: font_size,
                color: Color::White,
            },
        ));
        Size::new(f64::from(layout.full_width()), f64::from(layout.height()))
    }

    pub fn measure_ansi(&mut self, content: &str, font_size: f32) -> Size {
        let mut text = Styled_text::ansi(content);
        text.size = font_size;
        let layout = self.build_layout(&text);
        Size::new(f64::from(layout.full_width()), f64::from(layout.height()))
    }

    pub(crate) fn measure_icon(&mut self, icon: Lucide_icon, font_size: f32) -> Size {
        let layout = self.build_layout(&Styled_text::icon(
            icon,
            Text_style {
                size: font_size,
                color: Color::White,
            },
        ));
        icon_ink_bounds(&layout, icon, font_size).map_or_else(
            || Size::new(f64::from(layout.full_width()), f64::from(layout.height())),
            |bounds| Size::new(bounds.width(), bounds.height()),
        )
    }
}

impl Default for Text_context {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn icon_ink_bounds(
    layout: &Layout<Text_brush>,
    icon: Lucide_icon,
    font_size: f32,
) -> Option<Kurbo_rect> {
    let font = FontRef::new(LUCIDE_FONT_BYTES).ok()?;
    let glyph_id = font.charmap().map(icon.unicode())?;
    let bounds = font
        .glyph_metrics(Font_size::new(font_size), LocationRef::default())
        .bounds(glyph_id)?;
    let glyph = layout.lines().find_map(|line| {
        line.items().find_map(|item| match item {
            PositionedLayoutItem::GlyphRun(glyph_run) => glyph_run.positioned_glyphs().next(),
            _ => None,
        })
    })?;

    Some(Kurbo_rect::new(
        f64::from(glyph.x + bounds.x_min),
        f64::from(glyph.y - bounds.y_max),
        f64::from(glyph.x + bounds.x_max),
        f64::from(glyph.y - bounds.y_min),
    ))
}

#[derive(Clone)]
pub(crate) struct Styled_text {
    pub content: String,
    size: f32,
    font: Text_font,
    spans: Vec<Styled_span>,
}

#[derive(Clone, Copy)]
enum Text_font {
    Sans_serif,
    Lucide,
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
            font: Text_font::Sans_serif,
            spans: vec![Styled_span {
                range: 0..length,
                style: Ansi_style {
                    foreground: style.color,
                    ..Ansi_style::default()
                },
            }],
        }
    }

    pub(crate) fn icon(icon: Lucide_icon, style: Text_style) -> Self {
        let mut text = Self::styled(icon.unicode().to_string(), style);
        text.font = Text_font::Lucide;
        text
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
        font: Text_font::Sans_serif,
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
