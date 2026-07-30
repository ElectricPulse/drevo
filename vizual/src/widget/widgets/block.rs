use super::super::{Control, Focus_provider, Widget, Widget_trait, Widget_type};
use crate::{
    component::{Child, context::Component_context},
    config::BORDER_SIZE,
    constraint,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    style::Color,
    theme::Theme,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

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
impl Widget_trait for Block {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
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

        let child: Widget = Box::new(self.child.clone());
        Ok(Widget_type::Virtual(child))
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        paint_block(display, hitbox, &self.theme.load(), self.highlighted);
        Ok(None)
    }
}

fn paint_block(display: &mut Display<'_>, hitbox: Rect, style: &Block_style, focused: bool) {
    display.fill_rounded_rect(hitbox, style.background, style.border_radius);
    let color = match focused {
        true => style.focused_color,
        false => style.color,
    };
    let radius = (style.border_radius - BORDER_SIZE / 2.0).max(0.0);
    display.stroke_rounded_rect(hitbox.inset(BORDER_SIZE / 2.0), color, BORDER_SIZE, radius);
}
