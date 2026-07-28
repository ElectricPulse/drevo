use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::Child,
    hitbox::Hitbox,
    layouter::{Problem_context, constraints::prohibit_overlap},
    slot_manager::Slots,
    widget::{Control, Focus_provider, Renderable, Widget_type},
};

pub struct Grid {
    items: Vec<Child>,
    gap: f64,
}

impl Grid {
    pub fn new(items: Vec<Child>, gap: f64) -> Self {
        Self { items, gap }
    }
}

impl Control for Grid {}

#[async_trait]
impl Renderable for Grid {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: Hitbox,
        problem: Problem_context,
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
