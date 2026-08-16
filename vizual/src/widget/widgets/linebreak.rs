use crate::{
    component::{Children, context::Component_context},
    config::BORDER_SIZE,
    constraint,
    geometry::{Direction, Rect},
    graphics::scene::Scene,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::{Focus_provider, Layout_input, Render_input, Widget_trait},
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
            render,
            theme,
            hitbox,
            scene,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        scene.fill_rect(hitbox, theme.affect(render).await?.semantic.border);
        Ok(())
    }
}
