use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    widget::{IntoWidgets, LayoutInput, Widget, WidgetTrait},
};

#[derive(Clone)]
/// Gives each child an independent hitbox bounded by this grid.
///
/// This widget deliberately does not choose how its children relate to one another. Callers can
/// add their own constraints when they need a tiled grid, overlapping layers, or another
/// arrangement.
pub struct Grid {
    children: Vec<Widget>,
}

impl Grid {
    /// `gap` is retained for source compatibility; spacing is now a caller-owned constraint.
    pub fn new(children: impl IntoWidgets, _gap: f64) -> Self {
        Self {
            children: children.into(),
        }
    }
}

#[async_trait]
impl WidgetTrait for Grid {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let mut children = Vec::with_capacity(self.children.len());
        for (index, child) in self.children.iter().enumerate() {
            let child = slots.set(index as u64, child.clone()).await?;
            child.lock().await?.hitbox.make_independent();
            children.push(child);
        }

        for child in &children {
            let child_hitbox = child.get_hitbox().await?;
            for direction in [Direction::Horizontal, Direction::Vertical] {
                formula.constrain(
                    id!(),
                    constraint!(
                        child_hitbox.get_start_position(direction)
                            >= hitbox.get_start_position(direction)
                    ),
                )?;
                formula.constrain(
                    id!(),
                    constraint!(
                        child_hitbox.get_end_position(direction)
                            <= hitbox.get_end_position(direction)
                    ),
                )?;
            }
        }

        Ok(children)
    }
}
