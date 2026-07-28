use async_trait::async_trait;
use color_eyre::eyre::Result;
use good_lp::{Expression, constraint};

use crate::{
    backend::graphics::Paint_context,
    config::BORDER_SIZE,
    geometry::Rect,
    hitbox::{Direction, Hitbox},
    layouter::Problem_context,
    slot_manager::Slots,
    state::State,
    theme::Theme,
    widget::{Control, Focus_provider, Renderable, Widget_type},
};

pub struct Linebreak_component {
    pub theme: State<Theme>,
}

pub type Linebreak = Linebreak_component;

impl Linebreak {
    pub fn new(theme: State<Theme>) -> Self {
        Self { theme }
    }
}

impl Control for Linebreak_component {}

#[async_trait]
impl Renderable for Linebreak_component {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let horizontal_length = Expression::from(hitbox.get_dimension(Direction::Horizontal));

        problem.maximize(horizontal_length, 1).await?;
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) == BORDER_SIZE
            ))
            .await?;

        Ok(Widget_type::Visual(Vec::new()))
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        paint.fill_rect(hitbox, self.theme.load().semantic.border);
        Ok(None)
    }
}
