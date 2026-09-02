use async_trait::async_trait;
use color_eyre::Result;
use parley::Layout;

use super::super::{Layout_input, Render_input, Widget_trait};
use super::text::Text_style;
use crate::{
    component::Children,
    geometry::{Direction, Size},
    graphics::text::{Ansi_parser, Styled_text, Text_brush},
    state::State,
    style::Style,
};

#[cfg(test)]
mod tests;

/// Parsed ANSI text that can be extended without retaining earlier escape sequences.
#[derive(Clone)]
pub struct Content(Parsed_content);

#[derive(Clone)]
struct Parsed_content {
    text: Styled_text,
    parser: Ansi_parser,
}

impl Content {
    pub fn new(sequence: impl AsRef<str>) -> Self {
        let mut content = Self::default();
        content.append(sequence);
        content
    }

    /// Parses and appends only `sequence`; previously appended ANSI escapes are not retained.
    pub fn append(&mut self, sequence: impl AsRef<str>) {
        self.0
            .text
            .append_ansi(sequence.as_ref(), &mut self.0.parser);
    }

    pub fn text(&self) -> &Styled_text {
        &self.0.text
    }
}

impl Default for Content {
    fn default() -> Self {
        Self(Parsed_content {
            text: Styled_text::ansi(""),
            parser: Ansi_parser::default(),
        })
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Clone)]
pub struct Ansi {
    pub content: State<Content>,
    pub style: Style<Text_style>,
    cached_layout: Option<Layout<Text_brush>>,
}

impl Ansi {
    pub fn new(content: impl Into<Content>) -> Self {
        Self::from_state(content.into())
    }

    pub fn from_state(content: impl Into<State<Content>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            cached_layout: None,
        }
    }
}

#[async_trait]
impl Widget_trait for Ansi {
    async fn layout(
        &mut self,
        Layout_input {
            relayout,
            theme,
            hitbox,
            problem,
            text_context,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let content = self.content.affect(relayout.clone()).await?;
        let theme = theme.affect(relayout).await?;
        let font_size = self.style.get(&theme).size;
        let mut text = content.text().clone();
        text.size = font_size;
        let layout = text_context.build_layout(&text);
        let size = Size::new(f64::from(layout.full_width()), f64::from(layout.height()));
        self.cached_layout = Some(layout);

        hitbox
            .set_static_dimension(&problem, Direction::Horizontal, size.width)
            .await?;
        hitbox
            .set_static_dimension(&problem, Direction::Vertical, size.height)
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        Render_input { hitbox, scene, .. }: Render_input<'_, '_>,
    ) -> Result<()> {
        let layout = self
            .cached_layout
            .as_ref()
            .expect("Ansi must be laid out before rendering");
        scene.paint_layout(layout, hitbox.origin, true);
        Ok(())
    }
}
