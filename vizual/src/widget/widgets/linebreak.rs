use crate::{
    component::Children,
    config::BORDER_SIZE,
    constraint,
    geometry::Direction,
    widget::{Layout_input, Render_input, Widget_trait},
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
impl Widget_trait for Linebreak {
    async fn layout(
        &mut self,
        Layout_input {
            hitbox, problem, ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        problem
            .constrain(constraint!(
                hitbox.get_dimension(self.direction.flip()) == BORDER_SIZE
            ))
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        Render_input {
            rerender,
            theme,
            hitbox,
            scene,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        scene.fill_rect(hitbox, theme.affect(rerender).await?.semantic.border);
        Ok(())
    }
}
