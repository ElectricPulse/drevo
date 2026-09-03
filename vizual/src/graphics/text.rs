use std::{ops::Range, sync::Arc};

use color_eyre::Result;
use lucide_icons::{Icon as LucideIcon, LUCIDE_FONT_BYTES};
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontStyle, FontWeight, GenericFamily,
    Layout, LayoutContext, PositionedLayoutItem, StyleProperty, fontique::Blob,
};
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use vello::{kurbo::Rect as KurboRect, peniko::Brush};

use crate::{
    config::DEFAULT_FONT_SIZE,
    geometry::{Point, Size},
    state::{
        Store,
        memoization::{Dependency, Memoization, dependency, memoization},
    },
    style::Color,
    sync::Mutex,
    widget::widgets::text::TextStyle,
};

use super::scene::Scene as GraphicsScene;

#[cfg(test)]
mod tests;

// TODO:
// DISCLAIMER:
// Originally this was a tui library, but later I let codex convert it to wgpu
// almost all of the resulting slop is here

#[derive(Clone, Debug, PartialEq)]
pub struct TextBrush {
    pub foreground: Brush,
    pub background: Option<Brush>,
}

impl Default for TextBrush {
    fn default() -> Self {
        Self {
            foreground: Brush::Solid(Color::White.to_peniko()),
            background: None,
        }
    }
}

#[derive(Clone)]
pub struct TextContext {
    /// This does not need to be a store today, but is one in case its provider becomes dynamic.
    font_context: Store<Mutex<FontContext>>,
    /// This does not need to be a store today, but is one in case its provider becomes dynamic.
    layout_context: Store<Mutex<LayoutContext<TextBrush>>>,
}

#[derive(Clone)]
pub(crate) struct TextLayout {
    pub(crate) layout: Layout<TextBrush>,
    pub(crate) size: Size,
}

impl TextContext {
    pub fn new() -> Self {
        let mut font_context = FontContext::new();
        let _ = font_context
            .collection
            .register_fonts(Blob::new(Arc::new(LUCIDE_FONT_BYTES)), None);

        Self {
            font_context: Store::new(Mutex::new(font_context)),
            layout_context: Store::new(Mutex::new(LayoutContext::new())),
        }
    }

    pub(crate) async fn build_layout(&self, text: &StyledText) -> Result<Layout<TextBrush>> {
        self.build_layout_with_width(text, None).await
    }

    pub(crate) fn memoize_layout(&self, text: StyledText) -> Memoization<TextLayout> {
        let context = self.clone();
        memoization(
            move || {
                let context = context.clone();
                let text = text.clone();
                async move {
                    let layout = context.build_layout(&text).await?;
                    let size =
                        Size::new(f64::from(layout.full_width()), f64::from(layout.height()));
                    Ok(TextLayout { layout, size })
                }
            },
            self.dependencies(),
        )
    }

    fn dependencies(&self) -> Vec<Dependency> {
        vec![
            dependency(self.font_context.clone()),
            dependency(self.layout_context.clone()),
        ]
    }

    pub(crate) async fn build_wrapped_layout(
        &self,
        text: &StyledText,
        width: f32,
    ) -> Result<Layout<TextBrush>> {
        self.build_layout_with_width(text, Some(width)).await
    }

    async fn build_layout_with_width(
        &self,
        text: &StyledText,
        width: Option<f32>,
    ) -> Result<Layout<TextBrush>> {
        let font_context = self.font_context.read().await?;
        let mut font_context = font_context.lock().await?;
        let layout_context = self.layout_context.read().await?;
        let mut layout_context = layout_context.lock().await?;
        let mut builder =
            layout_context.ranged_builder(&mut font_context, &text.content, 1.0, false);
        match text.font {
            TextFont::SansSerif => builder.push_default(GenericFamily::SansSerif),
            TextFont::Lucide => builder.push_default(FontFamily::named("lucide")),
        }
        builder.push_default(StyleProperty::FontSize(text.size));
        builder.push_default(StyleProperty::Brush(TextBrush::default()));

        for span in &text.spans {
            if span.range.is_empty() {
                continue;
            }

            let (foreground, background) = resolved_colors(span.style);
            builder.push(
                StyleProperty::Brush(TextBrush {
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
        Ok(layout)
    }

    pub async fn draw_text(
        &self,
        scene: &mut GraphicsScene<'_>,
        text: &StyledText,
        origin: Point,
    ) -> Result<Size> {
        let layout = self.build_layout(text).await?;
        let size = Size::new(f64::from(layout.full_width()), f64::from(layout.height()));
        scene.paint_layout(&layout, origin, true);
        Ok(size)
    }

    pub(crate) async fn draw_icon(
        &self,
        scene: &mut GraphicsScene<'_>,
        icon: LucideIcon,
        origin: Point,
        style: TextStyle,
    ) -> Result<Size> {
        let layout = self.build_layout(&StyledText::icon(icon, style)).await?;
        let bounds = icon_ink_bounds(&layout, icon, style.size);
        let size = bounds.map_or_else(
            || Size::new(f64::from(layout.full_width()), f64::from(layout.height())),
            |bounds| Size::new(bounds.width(), bounds.height()),
        );
        let origin = bounds.map_or(origin, |bounds| {
            Point::new(origin.x - bounds.x0, origin.y - bounds.y0)
        });
        scene.paint_layout(&layout, origin, false);
        Ok(size)
    }

    pub async fn measure_text(&self, content: &str) -> Result<Size> {
        let layout = self
            .build_layout(&StyledText::plain(content, Color::White))
            .await?;
        Ok(Size::new(
            f64::from(layout.full_width()),
            f64::from(layout.height()),
        ))
    }

    pub async fn measure(&self, content: &str, font_size: f32) -> Result<Size> {
        let layout = self
            .build_layout(&StyledText::styled(
                content,
                TextStyle {
                    size: font_size,
                    color: Color::White,
                    bold: false,
                },
            ))
            .await?;
        Ok(Size::new(
            f64::from(layout.full_width()),
            f64::from(layout.height()),
        ))
    }

    pub async fn measure_ansi(&self, content: &str, font_size: f32) -> Result<Size> {
        let mut text = StyledText::ansi(content);
        text.size = font_size;
        let layout = self.build_layout(&text).await?;
        Ok(Size::new(
            f64::from(layout.full_width()),
            f64::from(layout.height()),
        ))
    }

    #[allow(dead_code)]
    pub(crate) async fn measure_icon(&self, icon: LucideIcon, font_size: f32) -> Result<Size> {
        let layout = self
            .build_layout(&StyledText::icon(
                icon,
                TextStyle {
                    size: font_size,
                    color: Color::White,
                    bold: false,
                },
            ))
            .await?;
        Ok(icon_ink_bounds(&layout, icon, font_size).map_or_else(
            || Size::new(f64::from(layout.full_width()), f64::from(layout.height())),
            |bounds| Size::new(bounds.width(), bounds.height()),
        ))
    }
}

impl Default for TextContext {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn icon_ink_bounds(
    layout: &Layout<TextBrush>,
    icon: LucideIcon,
    font_size: f32,
) -> Option<KurboRect> {
    let font = FontRef::new(LUCIDE_FONT_BYTES).ok()?;
    let glyph_id = font.charmap().map(icon.unicode())?;
    let bounds = font
        .glyph_metrics(FontSize::new(font_size), LocationRef::default())
        .bounds(glyph_id)?;
    let glyph = layout.lines().find_map(|line| {
        line.items().find_map(|item| match item {
            PositionedLayoutItem::GlyphRun(glyph_run) => glyph_run.positioned_glyphs().next(),
            _ => None,
        })
    })?;

    Some(KurboRect::new(
        f64::from(glyph.x + bounds.x_min),
        f64::from(glyph.y - bounds.y_max),
        f64::from(glyph.x + bounds.x_max),
        f64::from(glyph.y - bounds.y_min),
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyledText {
    pub content: String,
    pub size: f32,
    pub(crate) font: TextFont,
    pub(crate) spans: Vec<StyledSpan>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextFont {
    SansSerif,
    Lucide,
}

impl StyledText {
    pub fn plain(content: impl Into<String>, color: Color) -> Self {
        Self::styled(
            content,
            TextStyle {
                size: DEFAULT_FONT_SIZE,
                color,
                bold: false,
            },
        )
    }

    pub fn styled(content: impl Into<String>, style: TextStyle) -> Self {
        let content = content.into();
        let length = content.len();
        Self {
            content,
            size: style.size,
            font: TextFont::SansSerif,
            spans: vec![StyledSpan {
                range: 0..length,
                style: AnsiStyle {
                    foreground: style.color,
                    bold: style.bold,
                    ..AnsiStyle::default()
                },
                hyperlink: None,
            }],
        }
    }

    pub fn icon(icon: LucideIcon, style: TextStyle) -> Self {
        let mut text = Self::styled(icon.unicode().to_string(), style);
        text.font = TextFont::Lucide;
        text
    }

    pub fn ansi(content: &str) -> Self {
        let mut text = Self::empty();
        text.append_ansi(content, &mut AnsiParser::default());
        text
    }

    /// Returns the OSC 8 hyperlinks contained in this text.
    pub fn hyperlinks(&self) -> impl Iterator<Item = Hyperlink> + '_ {
        self.spans.iter().filter_map(|span| {
            span.hyperlink.as_ref().map(|url| Hyperlink {
                range: span.range.clone(),
                url: url.clone(),
            })
        })
    }

    fn empty() -> Self {
        Self {
            content: String::new(),
            size: DEFAULT_FONT_SIZE,
            font: TextFont::SansSerif,
            spans: Vec::new(),
        }
    }

    pub(crate) fn append_ansi(&mut self, input: &str, parser: &mut AnsiParser) {
        let mut segment_start = self.content.len();
        let mut index = 0;

        while index < input.len() {
            let remaining = &input[index..];
            if let Some(sequence_end) = remaining.strip_prefix("\u{1b}[").and_then(csi_end) {
                let sequence_end = 2 + sequence_end;
                let terminator = remaining.as_bytes()[sequence_end - 1];
                if terminator == b'm' {
                    push_span(
                        &mut self.spans,
                        segment_start,
                        self.content.len(),
                        parser.style,
                        &parser.hyperlink,
                    );
                    apply_sgr(&mut parser.style, &remaining[2..sequence_end - 1]);
                    segment_start = self.content.len();
                }
                index += sequence_end;
                continue;
            }

            let osc = remaining
                .strip_prefix("\u{1b}]")
                .map(|sequence| (sequence, 2))
                .or_else(|| {
                    remaining
                        .strip_prefix('\u{9d}')
                        .map(|sequence| (sequence, '\u{9d}'.len_utf8()))
                });
            if let Some((osc, prefix_length)) = osc.and_then(|(sequence, prefix_length)| {
                osc_end(sequence).map(|osc| (osc, prefix_length))
            }) {
                if let Some(url) = osc8_url(osc.payload) {
                    push_span(
                        &mut self.spans,
                        segment_start,
                        self.content.len(),
                        parser.style,
                        &parser.hyperlink,
                    );
                    parser.hyperlink = url.map(str::to_owned);
                    segment_start = self.content.len();
                }
                index += prefix_length + osc.length;
                continue;
            }

            let character = remaining.chars().next().unwrap();
            self.content.push(character);
            index += character.len_utf8();
        }

        push_span(
            &mut self.spans,
            segment_start,
            self.content.len(),
            parser.style,
            &parser.hyperlink,
        );
    }
}

/// A hyperlink extracted from an OSC 8 escape sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hyperlink {
    pub range: Range<usize>,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StyledSpan {
    pub(crate) range: Range<usize>,
    pub(crate) style: AnsiStyle,
    hyperlink: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct AnsiStyle {
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

#[derive(Clone, Default)]
pub(crate) struct AnsiParser {
    style: AnsiStyle,
    hyperlink: Option<String>,
}

fn resolved_colors(style: AnsiStyle) -> (Color, Option<Color>) {
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

fn push_span(
    spans: &mut Vec<StyledSpan>,
    start: usize,
    end: usize,
    style: AnsiStyle,
    hyperlink: &Option<String>,
) {
    if start < end {
        spans.push(StyledSpan {
            range: start..end,
            style,
            hyperlink: hyperlink.clone(),
        });
    }
}

fn csi_end(sequence: &str) -> Option<usize> {
    sequence
        .bytes()
        .position(|byte| (0x40..=0x7e).contains(&byte))
        .map(|index| index + 1)
}

struct Osc<'a> {
    payload: &'a str,
    length: usize,
}

fn osc_end(sequence: &str) -> Option<Osc<'_>> {
    let mut characters = sequence.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        match character {
            '\u{7}' | '\u{9c}' => {
                return Some(Osc {
                    payload: &sequence[..index],
                    length: index + character.len_utf8(),
                });
            }
            '\u{1b}' if matches!(characters.peek(), Some((_, '\\'))) => {
                let (_, terminator) = characters.next().unwrap();
                return Some(Osc {
                    payload: &sequence[..index],
                    length: index + character.len_utf8() + terminator.len_utf8(),
                });
            }
            _ => {}
        }
    }
    None
}

fn osc8_url(payload: &str) -> Option<Option<&str>> {
    let mut fields = payload.splitn(3, ';');
    (fields.next() == Some("8")).then(|| fields.nth(1).filter(|url| !url.is_empty()))
}

fn apply_sgr(style: &mut AnsiStyle, sequence: &str) {
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
            0 => *style = AnsiStyle::default(),
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
