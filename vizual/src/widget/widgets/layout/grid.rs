use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::Children,
    geometry::Direction,
    layouter::constraints::{prohibit_overlap, shrink_wrap},
    widget::{IntoWidgets, LayoutInput, Widget, WidgetTrait},
};

#[derive(Clone)]
/// Lays out children without overlap.
///
/// Warning: every pair of children uses binary variables to select their relative position.
/// Those variables make the layout a MIP and can substantially slow solving as the grid grows.
pub struct Grid {
    children: Vec<Widget>,
    gap: f64,
}

impl Grid {
    pub fn new(children: impl IntoWidgets, gap: f64) -> Self {
        Self {
            children: children.into(),
            gap,
        }
    }
}

#[async_trait]
impl WidgetTrait for Grid {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            formula: problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let mut children = Vec::with_capacity(self.children.len());
        for (index, child) in self.children.iter().enumerate() {
            children.push(slots.set(index as u64, child.clone()).await?);
        }

        for (index, first) in children.iter().enumerate() {
            for second in children.iter().skip(index + 1) {
                let first_hitbox = first.get_hitbox().await?;
                let second_hitbox = second.get_hitbox().await?;

                prohibit_overlap(problem, first_hitbox, second_hitbox, self.gap)?;
            }
        }

        for direction in [Direction::Horizontal, Direction::Vertical] {
            shrink_wrap(problem, hitbox.clone(), &children, direction).await?;
        }

        Ok(children)
    }
}
