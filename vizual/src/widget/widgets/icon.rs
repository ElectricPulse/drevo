use async_trait::async_trait;
use color_eyre::Result;
use lucide_icons::Icon as Lucide_icon;

use super::{
    super::{Focus_provider, Widget_trait},
    text::Text_style,
};
use crate::{
    component::{Children, context::Component_context},
    constraint,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
};

pub struct Icon {
    icon: Lucide_icon,
    pub style: State<Text_style>,
}

impl Icon {
    pub fn new(icon: Lucide_icon, style: State<Text_style>) -> Self {
        Self { icon, style }
    }
}

#[async_trait]
impl Widget_trait for Icon {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let size = text_context.measure_icon(self.icon, self.style.load().size);
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Horizontal) == size.width
            ))
            .await?;
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) == size.height
            ))
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        let _ = display.draw_icon(self.icon, hitbox.origin, *self.style.load());
        Ok(None)
    }
}
