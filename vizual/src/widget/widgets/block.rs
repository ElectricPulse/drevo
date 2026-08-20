use super::{
    super::{Layout_input, Render_input, Widget_trait},
    positioning::space::Space,
};
use crate::{
    component::Children,
    geometry::Rect,
    graphics::scene::Scene,
    layouter::objective::Delta,
    style::Color,
    widget::Widget,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use crate::macros::display;

#[derive(Clone, Copy, PartialEq)]
pub struct Border_style {
    pub thickness: f64,
    pub color: Color,
    pub radius: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Block_style {
    pub padding: f64,
    pub background: Color,
    pub border: Border_style,
    pub focused_border: Border_style,
}

#[derive(Clone)]
pub struct Block {
    child: Widget,
    pub style: Block_style,
    pub focusable: bool,
    pub delta: Option<Delta>,
}

impl Block {
    pub fn new(child: impl Widget_trait, style: Block_style) -> Self {
        Self {
            child: child.as_any(),
            style,
            focusable: false,
            delta: None,
        }
    }
}

#[async_trait]
impl Widget_trait for Block {
    async fn layout(
        &mut self,
        Layout_input {
            focus, slots, ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        focus.set_interactive(self.focusable);
        let style = self.style;
        let border_thickness = style.border.thickness.max(style.focused_border.thickness);
        let mut space = Space::uniform(self.child.clone(), style.padding + border_thickness, 1);
        space.delta = self.delta.clone();
        space.minimum = border_thickness;

        Ok(vec![display!(space)])
    }

    async fn render(
        &mut self,
        Render_input {
            focus,
            hitbox,
            scene,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        let focused = self.focusable && focus.get();
        paint_block(scene, hitbox, &self.style, focused);
        Ok(())
    }
}

fn paint_block(scene: &mut Scene<'_>, hitbox: Rect, style: &Block_style, focused: bool) {
    let border = match focused {
        true => style.focused_border,
        false => style.border,
    };

    scene.fill_rounded_rect(hitbox, style.background, border.radius);
    if border.thickness > 0.0 {
        let radius = (border.radius - border.thickness / 2.0).max(0.0);
        scene.stroke_rounded_rect(
            hitbox.inset(border.thickness / 2.0),
            border.color,
            border.thickness,
            radius,
        );
    }
}
