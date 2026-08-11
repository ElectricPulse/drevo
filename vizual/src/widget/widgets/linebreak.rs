use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

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
    widget::{Focus_provider, Widget_trait, widgets::full::Full},
};

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
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let line = Linebreak_unsized;
        let full = Full::width(line);

        Ok(vec![display!(full)])
    }
}

#[derive(Clone, Copy)]
struct Linebreak_unsized;

#[async_trait]
impl Widget_trait for Linebreak_unsized {
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
        problem.constrain_hitbox(*hitbox).await?;
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
