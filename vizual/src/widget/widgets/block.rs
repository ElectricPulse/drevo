use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::{Child, Children, context::Component_context},
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

#[derive(Clone, Copy)]
pub struct Border_style {
    pub thickness: f64,
    pub color: Color,
    pub radius: f64,
}

#[derive(Clone, Copy)]
pub struct Block_style {
    pub background: Color,
    pub border: Border_style,
    pub focused_border: Border_style,
}

pub struct Block {
    child: Child,
    pub style: State<Block_style>,
    pub highlighted: bool,
}

impl Block {
    pub fn new(child: Child, style: State<Block_style>) -> Self {
        Self {
            child,
            style,
            highlighted: false,
        }
    }
}

#[async_trait]
impl Widget_trait for Block {
    async fn layout(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let child_hitbox = self.child.get_hitbox().await?;
        let style = self.style.load();
        let border_thickness = style.border.thickness.max(style.focused_border.thickness);

        for direction in [Direction::Horizontal, Direction::Vertical] {
            problem
                .constrain(constraint!(
                    hitbox.get_dimension(direction)
                        == child_hitbox.get_dimension(direction) + 2.0 * border_thickness
                ))
                .await?;

            problem
                .constrain(constraint!(
                    child_hitbox.get_start_position(direction)
                        == hitbox.get_start_position(direction) + border_thickness
                ))
                .await?;
        }

        Ok(vec![self.child.clone()])
    }

    async fn render(
        &mut self,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        paint_block(display, hitbox, &self.style.load(), self.highlighted);
        Ok(None)
    }
}

impl From<&State<Theme>> for State<Block_style> {
    fn from(theme: &State<Theme>) -> Self {
        theme.project(|theme| &theme.specific.block)
    }
}

fn paint_block(display: &mut Display<'_>, hitbox: Rect, style: &Block_style, focused: bool) {
    let border = match focused {
        true => style.focused_border,
        false => style.border,
    };

    display.fill_rounded_rect(hitbox, style.background, border.radius);
    if border.thickness > 0.0 {
        let radius = (border.radius - border.thickness / 2.0).max(0.0);
        display.stroke_rounded_rect(
            hitbox.inset(border.thickness / 2.0),
            border.color,
            border.thickness,
            radius,
        );
    }
}
