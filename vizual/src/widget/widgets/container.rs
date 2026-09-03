use crate::macros::display;
use async_trait::async_trait;
use color_eyre::Result;

use super::super::{LayoutInput, Widget, WidgetTrait};
use crate::{
    component::Children,
    geometry::{Direction, Size},
};

/// Gives a child an intermediate hitbox with an optional static size.
#[derive(Clone)]
pub struct Container {
    child: Widget,
    fixed_size: Option<Size>,
}

impl Container {
    pub fn new(child: impl WidgetTrait) -> Self {
        Self {
            child: child.as_any(),
            fixed_size: None,
        }
    }

    pub fn fixed_size(mut self, size: Size) -> Self {
        self.fixed_size = Some(size);
        self
    }
}

#[async_trait]
impl WidgetTrait for Container {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            problem,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        if let Some(size) = self.fixed_size {
            hitbox
                .set_static_dimension(&problem, Direction::Horizontal, size.width)
                .await?;
            hitbox
                .set_static_dimension(&problem, Direction::Vertical, size.height)
                .await?;
        }

        let child = display!(self.child.clone());

        Ok(vec![child])
    }
}
