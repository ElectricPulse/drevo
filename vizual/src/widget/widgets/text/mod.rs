pub mod ansi;

use super::super::{LayoutInput, RenderInput, WidgetTrait};
use async_trait::async_trait;
use color_eyre::Result;
use std::sync::Arc;

use crate::{
    component::Children,
    config::DEFAULT_FONT_SIZE,
    geometry::Direction,
    graphics::text::{StyledText, TextLayout},
    state::{State, StateTrait, memoization::Memoization},
    style::{Color, Style},
    sync::Mutex,
    theme::Theme,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    pub bold: bool,
}

impl TextStyle {
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: DEFAULT_FONT_SIZE,
            color: Color::White,
            bold: false,
        }
    }
}

impl From<Theme> for TextStyle {
    fn from(theme: Theme) -> Self {
        theme.specific.text.paragraph
    }
}

#[derive(Clone)]
pub struct Text {
    content: State<String>,
    pub style: Style<TextStyle>,
    cached_layout: Arc<Mutex<Option<(StyledText, Memoization<TextLayout>)>>>,
}

impl Text {
    pub fn new(content: impl Into<State<String>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            cached_layout: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl WidgetTrait for Text {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            hitbox,
            formula: problem,
            text_context,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let content = self.content.affect(relayout.clone()).await?;
        let theme = theme.affect(relayout.clone()).await?;
        let style = self.style.get(&theme);
        let text = StyledText::styled(&*content, style);
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
            .set_static_dimension(problem, Direction::Horizontal, size.width)
            .await?;
        hitbox
            .set_static_dimension(problem, Direction::Vertical, size.height)
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
            .expect("Text must be laid out before rendering")
            .1
            .clone();
        let layout = memoization.read().await?;
        scene.paint_layout(&layout.layout, hitbox.origin, true);
        Ok(())
    }
}
