use crate::{
    component::{Children, context::Component_context},
    config::BORDER_SIZE,
    constraint,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::{Focus_provider, Widget_trait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Copy)]
pub struct Linebreak;

impl Linebreak {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Widget_trait for Linebreak {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        hitbox.end.point_to_variable(
            Direction::Vertical,
            problem.make_independent_variable("linebreak-end"),
        );
        problem.constrain_hitbox(hitbox.clone()).await?;
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) == BORDER_SIZE
            ))
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        display.fill_rect(hitbox, theme.load().semantic.border);
        Ok(None)
    }
}
