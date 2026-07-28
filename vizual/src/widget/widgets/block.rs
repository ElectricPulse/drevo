use async_trait::async_trait;
use color_eyre::eyre::Result;
use good_lp::constraint;

use super::super::{Control, Focus_provider, Renderable, Widget_type};
use crate::{
    backend::graphics::Paint_context,
    component::Child,
    config::BORDER_SIZE,
    geometry::Rect,
    hitbox::{Direction, Hitbox},
    layouter::Problem_context,
    slot_manager::Slots,
    state::State,
    style::Color,
    theme::Theme,
};

#[derive(Clone)]
pub struct Block_style {
    pub background: Color,
    pub color: Color,
    pub focused_color: Color,
    pub border_radius: f64,
}

pub struct Block {
    child: Child,
    pub theme: State<Block_style>,
    pub highlighted: bool,
}

impl Block {
    pub fn new(child: Child, theme: State<Theme>) -> Self {
        let theme = theme.project(|theme| &theme.specific.block);
        Self {
            child,
            theme,
            highlighted: false,
        }
    }
}

impl Control for Block {}

#[async_trait]
impl Renderable for Block {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Problem_context,
        _slots: &mut Slots,
    ) -> Result<Widget_type> {
        let child_hitbox = self.child.get_hitbox().await?;

        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(constraint!(
                    hitbox.get_dimension(direction)
                        == child_hitbox.get_dimension(direction) + 2.0 * BORDER_SIZE
                ))
                .await?;

            problem
                .constrain(constraint!(
                    child_hitbox.get_start_position(direction)
                        == hitbox.get_start_position(direction) + BORDER_SIZE
                ))
                .await?;
        }

        Ok(Widget_type::Virtual(Box::new(self.child.clone())))
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        paint: &mut Paint_context<'_>,
    ) -> Result<Option<Hitbox>> {
        paint_block(paint, hitbox, &self.theme.load(), self.highlighted);
        Ok(None)
    }
}

fn paint_block(paint: &mut Paint_context<'_>, hitbox: Rect, style: &Block_style, focused: bool) {
    paint.fill_rounded_rect(hitbox, style.background, style.border_radius);
    let color = match focused {
        true => style.focused_color,
        false => style.color,
    };
    let radius = (style.border_radius - BORDER_SIZE / 2.0).max(0.0);
    paint.stroke_rounded_rect(hitbox.inset(BORDER_SIZE / 2.0), color, BORDER_SIZE, radius);
}
