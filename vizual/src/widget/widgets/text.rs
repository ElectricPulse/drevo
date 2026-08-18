use super::super::{Layout_input, Render_input, Widget_trait};
use crate::{
    component::Children,
    config::DEFAULT_FONT_SIZE,
    geometry::Direction,
    state::State,
    style::{Color, Style},
    theme::Theme,
};
use async_trait::async_trait;
use color_eyre::Result;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Text_style {
    pub size: f32,
    pub color: Color,
}

impl Default for Text_style {
    fn default() -> Self {
        Self {
            size: DEFAULT_FONT_SIZE,
            color: Color::White,
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
    pub ansi: bool,
}

impl Text {
    pub fn new(content: impl Into<State<String>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            ansi: false,
        }
    }

    pub fn ansi(content: impl Into<State<String>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            ansi: true,
        }
    }
}

#[async_trait]
impl Widget_trait for Text {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            hitbox,
            problem,
            text_context,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let content = self.content.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let font_size = self.style.get(&theme).size;
        let size = match self.ansi {
            true => text_context.measure_ansi(&content, font_size),
            false => text_context.measure(&content, font_size),
        };
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
        Render_input {
            render,
            theme,
            hitbox,
            scene,
            text_context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        let content = self.content.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let style = self.style.get(&theme);
        match self.ansi {
            true => {
                let _ = text_context.draw_ansi_text(scene, &content, hitbox.origin, style.size);
            }
            false => {
                let _ = text_context.draw_text(scene, &content, hitbox.origin, style);
            }
        }
        Ok(())
    }
}
