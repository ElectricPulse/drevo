use crate::{
    component::Children,
    config::BORDER_SIZE,
    constraint,
    geometry::Direction,
    id,
    widget::{LayoutInput, RenderInput, WidgetTrait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Copy)]
pub struct Linebreak {
    direction: Direction,
}

impl Linebreak {
    pub fn new(direction: Direction) -> Self {
        Self { direction }
    }
}

#[async_trait]
impl WidgetTrait for Linebreak {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox, formula, ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        formula.constrain(
            id!(),
            constraint!(hitbox.get_dimension(self.direction.flip()) == BORDER_SIZE),
        )?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        RenderInput {
            rerender,
            theme,
            hitbox,
            scene,
            ..
        }: RenderInput<'_, '_>,
    ) -> Result<()> {
        scene.fill_rect(hitbox, theme.affect(rerender).await?.semantic.border);
        Ok(())
    }
}
