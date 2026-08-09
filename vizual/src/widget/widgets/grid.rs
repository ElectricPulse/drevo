use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::{Children, context::Component_context},
    layouter::{constraints::prohibit_overlap, hitbox::Hitbox},
    slot::manager::Slots,
    widget::{Focus_provider, General_shared_widget, Widget_trait},
};

pub struct Grid {
    children: Vec<General_shared_widget>,
    gap: f64,
}

impl Grid {
    pub fn new(children: Vec<General_shared_widget>, gap: f64) -> Self {
        Self { children, gap }
    }
}

#[async_trait]
impl Widget_trait for Grid {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: crate::state::State<crate::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let mut children = Vec::with_capacity(self.children.len());
        for (index, child) in self.children.iter().enumerate() {
            children.push(slots.set(index as u64, child.clone()).await?);
        }

        for (index, first) in children.iter().enumerate() {
            for second in children.iter().skip(index + 1) {
                let first_hitbox = first.get_hitbox().await?;
                let second_hitbox = second.get_hitbox().await?;

                prohibit_overlap(&problem, first_hitbox, second_hitbox, self.gap).await?;
            }
        }

        Ok(children)
    }
}
