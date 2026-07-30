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
    widget::{Control, Focus_provider, Widget_trait, widgets::full::Full},
};

pub struct Linebreak {
    pub theme: State<Theme>,
}

impl Linebreak {
    pub fn new(theme: State<Theme>) -> Self {
        Self { theme }
    }
}

impl Control for Linebreak {}

#[async_trait]
impl Widget_trait for Linebreak {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let line = Linebreak_component {
            theme: self.theme.clone(),
        };
        let full = Full::width(display!(line));

        Ok(vec![display!(full)])
    }
}

struct Linebreak_component {
    theme: State<Theme>,
}

impl Control for Linebreak_component {}

#[async_trait]
impl Widget_trait for Linebreak_component {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) == BORDER_SIZE
            ))
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        display.fill_rect(hitbox, self.theme.load().semantic.border);
        Ok(None)
    }
}
