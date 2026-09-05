use crate::macros::display;
use async_trait::async_trait;
use color_eyre::Result;

use super::super::{LayoutInput, Widget, WidgetTrait};
use crate::{
    component::Children,
    geometry::{Direction, Size},
};

/// Gives a child an intermediate hitbox with a static size.
#[derive(Clone)]
pub struct Container {
    child: Widget,
    size: Size,
}

impl Container {
    pub fn new(child: impl WidgetTrait, size: Size) -> Self {
        Self {
            child: child.as_any(),
            size,
        }
    }
}

#[async_trait]
impl WidgetTrait for Container {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            formula,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        hitbox
            .set_static_dimension(formula, Direction::Horizontal, self.size.width)
            .await?;
        hitbox
            .set_static_dimension(formula, Direction::Vertical, self.size.height)
            .await?;

        let child = display!(self.child.clone());

        Ok(vec![child])
    }
}
