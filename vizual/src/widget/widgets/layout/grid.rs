use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::{
        constraints::{prohibit_overlap, shrink_wrap},
        hitbox::Hitbox,
    },
    slot::manager::Slots,
    widget::{Focus_provider, Into_widgets, Layout_input, Widget, Widget_trait},
};

#[derive(Clone)]
pub struct Grid {
    children: Vec<Widget>,
    gap: f64,
}

impl Grid {
    pub fn new(children: impl Into_widgets, gap: f64) -> Self {
        Self {
            children: children.into(),
            gap,
        }
    }
}

#[async_trait]
impl Widget_trait for Grid {
    async fn layout(
        &mut self,
        Layout_input {
            hitbox,
            problem,
            slots,
            ..
        }: Layout_input<'_>,
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

        for direction in [Direction::Horizontal, Direction::Vertical] {
            shrink_wrap(&problem, hitbox.clone(), &children, direction).await?;
        }

        Ok(children)
    }
}
