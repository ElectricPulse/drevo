use super::super::{Focus_provider, Widget_trait};
use crate::{
    component::{Child, Children, context::Component_context},
    constraint,
    display::Display,
    geometry::{Direction, Rect},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    style::{Color, Style},
    theme::Theme,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Copy, PartialEq)]
pub struct Border_style {
    pub thickness: f64,
    pub color: Color,
    pub radius: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Block_style {
    pub background: Color,
    pub border: Border_style,
    pub focused_border: Border_style,
}

#[derive(Clone)]
pub struct Block {
    child: Child,
    pub style: Style<Block_style>,
    pub highlighted: bool,
}

impl Block {
    pub fn new(child: Child) -> Self {
        Self {
            child,
            style: Style::default(),
            highlighted: false,
        }
    }
}

#[async_trait]
impl Widget_trait for Block {
    async fn layout(
        &mut self,
        _render: crate::Render,
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut crate::text::Text_context,
        _slots: &mut Slots,
    ) -> Result<Children> {
        let child_hitbox = self.child.get_hitbox().await?;
        let style = self.style.get(&theme);
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
        theme: State<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        paint_block(display, hitbox, &self.style.get(&theme), self.highlighted);
        Ok(None)
    }
}

impl From<Theme> for Block_style {
    fn from(theme: Theme) -> Self {
        theme.specific.block
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
