use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::{Shared_component, context::Component_context},
    layouter::{constraints::prohibit_overlap, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Control, Focus_provider, Widget_trait, Widget_type},
};

pub struct Grid {
    items: Vec<Shared_component>,
    gap: f64,
}

impl Grid {
    pub fn new(items: Vec<Shared_component>, gap: f64) -> Self {
        Self { items, gap }
    }
}

impl Control for Grid {}

#[async_trait]
impl Widget_trait for Grid {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        for (index, first) in self.items.iter().enumerate() {
            for second in self.items.iter().skip(index + 1) {
                let first_hitbox = first.get_hitbox().await?;
                let second_hitbox = second.get_hitbox().await?;

                prohibit_overlap(&problem, first_hitbox, second_hitbox, self.gap).await?;
            }
        }

        Ok(Widget_type::Visual(self.items.clone()))
    }
}
