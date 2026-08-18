use async_trait::async_trait;
use color_eyre::Result;

use super::super::{Layout_input, Render_input, Widget_trait};
use super::text::Text_style;
use crate::{
    component::Children,
    geometry::Direction,
    state::State,
    style::Style,
};

#[derive(Clone)]
pub struct Ansi {
    content: State<String>,
    pub style: Style<Text_style>,
}

impl Ansi {
    pub fn new(content: impl Into<State<String>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
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
        let size = text_context.measure_ansi(&content, font_size);
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
        let _ = text_context.draw_ansi_text(scene, &content, hitbox.origin, style.size);
        Ok(())
    }
}
