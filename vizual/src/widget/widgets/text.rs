use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::Children,
    component::context::Component_context,
    config::DEFAULT_FONT_SIZE,
    geometry::{Direction, Rect},
    graphics::scene::Scene,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
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
    content: Box<dyn State<Output = String>>,
    pub style: Style<Text_style>,
}

impl Text {
    pub fn new(content: impl Into<Box<dyn State<Output = String>>>) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }
}

#[async_trait]
impl Widget_trait for Text {
    async fn layout(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let content = self.content.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let size = text_context.measure(&content, self.style.get(&theme).size);
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
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        let content = self.content.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let _ = text_context.draw_text(scene, &content, hitbox.origin, self.style.get(&theme));
        Ok(None)
    }
}
