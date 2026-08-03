use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::{Child, Children, context::Component_context},
    layouter::{constraints::prohibit_overlap, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Focus_provider, Widget_trait},
};

pub struct Grid {
    children: Vec<Child>,
    gap: f64,
}

impl Grid {
    pub fn new(children: Vec<Child>, gap: f64) -> Self {
        Self { children, gap }
    }
}

#[async_trait]
impl Widget_trait for Grid {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        for (index, first) in self.children.iter().enumerate() {
            for second in self.children.iter().skip(index + 1) {
                let first_hitbox = first.get_hitbox().await?;
                let second_hitbox = second.get_hitbox().await?;

                prohibit_overlap(&problem, first_hitbox, second_hitbox, self.gap).await?;
            }
        }

        Ok(self.children.clone())
    }
}
