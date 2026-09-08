use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    layouter::constraints::prohibit_overlap,
    widget::{Components, IntoComponents, LayoutInput, WidgetTrait},
};

#[derive(Clone)]
/// Bounds each child within this grid.
///
/// By default, children receive independent hitboxes. [`Grid::prohibit_overlap`] instead
/// preserves each child's existing hitbox sharing so its positioning widget controls which edges
/// may move while resolving overlap.
pub struct Grid {
    children: Components,
    gap: f64,
    prohibit_overlap: bool,
}

impl Grid {
    pub fn new(children: impl IntoComponents + 'static, gap: f64) -> Self {
        Self {
            children: Box::new(children),
            gap,
            prohibit_overlap: false,
        }
    }

    pub fn prohibit_overlap(mut self) -> Self {
        self.prohibit_overlap = true;
        self
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
        let children = self.children.into_components(slots).await?;
        if !self.prohibit_overlap {
            for child in &children {
                child.lock().await?.hitbox.make_independent();
            }
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

        if self.prohibit_overlap {
            for (index, first) in children.iter().enumerate() {
                for second in &children[index + 1..] {
                    prohibit_overlap(
                        formula,
                        first.get_hitbox().await?,
                        second.get_hitbox().await?,
                        self.gap,
                    )?;
                }
            }
        }

        Ok(children)
    }
}
