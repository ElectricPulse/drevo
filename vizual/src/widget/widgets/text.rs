use super::super::{Layout_input, Render_input, Widget_trait};
use async_trait::async_trait;
use color_eyre::Result;
use std::sync::Arc;

use crate::{
    component::Children,
    config::DEFAULT_FONT_SIZE,
    geometry::Direction,
    graphics::text::{Styled_text, Text_layout},
    state::{State, State_trait, memoization::Memoization},
    style::{Color, Style},
    sync::Mutex,
    theme::Theme,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Text_style {
    pub size: f32,
    pub color: Color,
    pub bold: bool,
}

impl Text_style {
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

impl Default for Text_style {
    fn default() -> Self {
        Self {
            size: DEFAULT_FONT_SIZE,
            color: Color::White,
            bold: false,
        }
    }
}

impl From<Theme> for Text_style {
    fn from(theme: Theme) -> Self {
        theme.specific.text.paragraph
    }
}

#[derive(Clone)]
pub struct Text {
    content: State<String>,
    pub style: Style<Text_style>,
    cached_layout: Arc<Mutex<Option<(Styled_text, Memoization<Text_layout>)>>>,
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
impl Widget_trait for Text {
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
        let theme = theme.affect(relayout.clone()).await?;
        let style = self.style.get(&theme);
        let text = Styled_text::styled(&*content, style);
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
        Render_input { hitbox, scene, .. }: Render_input<'_, '_>,
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
