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
    state::State,
    style::Style,
    theme::Theme,
};

#[derive(Clone)]
pub struct Icon {
    icon: Lucide_icon,
    pub style: Style<Text_style>,
}

impl Icon {
    pub fn new(icon: Lucide_icon) -> Self {
        Self {
            icon,
            style: Style::default(),
        }
    }
}

#[async_trait]
impl Widget_trait for Icon {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let size = text_context.measure_icon(self.icon, self.style.get(&theme).size);
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
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        let _ = text_context.draw_icon(scene, self.icon, hitbox.origin, self.style.get(&theme));
        Ok(None)
    }
}
