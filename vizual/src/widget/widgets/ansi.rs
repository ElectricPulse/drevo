use async_trait::async_trait;
use color_eyre::Result;
use std::sync::Arc;

use super::super::{LayoutInput, RenderInput, WidgetTrait};
use super::text::TextStyle;
use crate::{
    component::Children,
    geometry::Direction,
    graphics::text::{AnsiParser, StyledText, TextLayout},
    state::{State, StateTrait, memoization::Memoization},
    style::Style,
    sync::Mutex,
};

#[cfg(test)]
mod tests;

/// Parsed ANSI text that can be extended without retaining earlier escape sequences.
#[derive(Clone)]
pub struct Content(ParsedContent);

#[derive(Clone)]
struct ParsedContent {
    text: StyledText,
    parser: AnsiParser,
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

    pub fn text(&self) -> &StyledText {
        &self.0.text
    }
}

impl Default for Content {
    fn default() -> Self {
        Self(ParsedContent {
            text: StyledText::ansi(""),
            parser: AnsiParser::default(),
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
    pub style: Style<TextStyle>,
    cached_layout: Arc<Mutex<Option<(StyledText, Memoization<TextLayout>)>>>,
}

impl Ansi {
    pub fn new(content: impl Into<Content>) -> Self {
        Self::from_state(content.into())
    }

    pub fn from_state(content: impl Into<State<Content>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            cached_layout: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl WidgetTrait for Ansi {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            hitbox,
            problem,
            text_context,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let content = self.content.affect(relayout.clone()).await?;
        let theme = theme.affect(relayout.clone()).await?;
        let font_size = self.style.get(&theme).size;
        let mut text = content.text().clone();
        text.size = font_size;
        let memoization = {
            let mut cached_layout = self.cached_layout.lock().await?;
            match &*cached_layout {
                Some((cached_text, memoization)) if cached_text == &text => memoization.clone(),
                _ => {
                    let memoization = text_context.memoize_layout(text.clone());
                    *cached_layout = Some((text, memoization.clone()));
                    memoization
                }
            }
        };
        let layout = memoization.affect(relayout).await?;
        let size = layout.size;

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
        RenderInput { hitbox, scene, .. }: RenderInput<'_, '_>,
    ) -> Result<()> {
        let memoization = self
            .cached_layout
            .lock()
            .await?
            .as_ref()
            .expect("Ansi must be laid out before rendering")
            .1
            .clone();
        let layout = memoization.read().await?;
        scene.paint_layout(&layout.layout, hitbox.origin, true);
        Ok(())
    }
}
