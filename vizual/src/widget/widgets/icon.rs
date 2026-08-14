use async_trait::async_trait;
use color_eyre::Result;
use lucide_icons::Icon as Lucide_icon;

use super::{
    super::{Focus_provider, Widget_trait},
    text::Text_style,
};
use crate::{
    component::{Children, context::Component_context},
    geometry::{Direction, Rect},
    graphics::scene::Scene,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    style::Style,
    theme::Theme,
};

#[derive(Clone)]
pub struct Icon {
    icon: Box<dyn State<Output = Lucide_icon>>,
    pub style: Style<Text_style>,
}

impl Icon {
    pub fn new(icon: impl Into<Box<dyn State<Output = Lucide_icon>>>) -> Self {
        Self {
            icon: icon.into(),
            style: Style::default(),
        }
    }
}

#[async_trait]
impl Widget_trait for Icon {
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
        let icon = *self.icon.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let size = text_context.measure_icon(icon, self.style.get(&theme).size);
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
        let icon = *self.icon.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let _ = text_context.draw_icon(scene, icon, hitbox.origin, self.style.get(&theme));
        Ok(None)
    }
}
