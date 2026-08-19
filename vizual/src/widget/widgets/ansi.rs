use async_trait::async_trait;
use color_eyre::Result;
use parley::Layout;

use super::super::{Layout_input, Render_input, Widget_trait};
use super::text::Text_style;
use crate::{
    component::Children,
    geometry::{Direction, Size},
    graphics::text::{Styled_text, Text_brush},
    state::State,
    style::Style,
};

#[derive(Clone)]
pub struct Ansi {
    content: State<String>,
    pub style: Style<Text_style>,
    cached_layout: Option<Layout<Text_brush>>,
}

impl Ansi {
    pub fn new(content: impl Into<State<String>>) -> Self {
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
        let mut text = Styled_text::ansi(&content);
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
            let style = self.style.get(&theme);
            let _ = text_context.draw_ansi_text(scene, &content, hitbox.origin, style.size);
        }
        Ok(())
    }
}
