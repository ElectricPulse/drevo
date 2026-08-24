use super::super::{Layout_input, Render_input, Widget_trait};
use crate::{
    component::Children,
    config::DEFAULT_FONT_SIZE,
    geometry::{Direction, Size},
    graphics::text::{Styled_text, Text_brush},
    state::State,
    style::{Color, Style},
    theme::Theme,
};
use async_trait::async_trait;
use color_eyre::Result;
use parley::Layout;

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
    cached_layout: Option<Layout<Text_brush>>,
}

impl Text {
    pub fn new(content: impl Into<State<String>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
            cached_layout: None,
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
        let style = self.style.get(&theme);
        let layout = text_context.build_layout(&Styled_text::styled(&*content, style));
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
        Render_input {
            render,
            theme,
            hitbox,
            scene,
            text_context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        if let Some(layout) = &self.cached_layout {
            scene.paint_layout(layout, hitbox.origin, true);
        } else {
            let content = self.content.affect(render.clone()).await?;
            let theme = theme.affect(render).await?;
            let _ = text_context.draw_text(scene, &content, hitbox.origin, self.style.get(&theme));
        }
        Ok(())
    }
}
