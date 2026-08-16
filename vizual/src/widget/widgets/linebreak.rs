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
    widget::{Focus_provider, Widget_trait},
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
        _render: crate::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        _slots: &mut Slots,
        _logical: &mut bool,
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
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        _text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        scene.fill_rect(hitbox, theme.affect(render).await?.semantic.border);
        Ok(None)
    }
}
