use super::{
    super::{LayoutInput, RenderInput, WidgetTrait},
    positioning::space::Space,
};
use crate::macros::display;
use crate::{
    component::Children,
    geometry::Rect,
    graphics::scene::Scene,
    layouter::{objective::Delta, priorities::INTRINSIC_SPACING},
    style::Color,
    widget::Widget,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[derive(Clone, Copy, PartialEq)]
pub struct BorderStyle {
    pub thickness: f64,
    pub color: Color,
    pub radius: f64,
}

impl BorderStyle {
    /// Returns a square border that does not paint or reserve space.
    pub fn none() -> Self {
        Self {
            thickness: 0.0,
            color: Color::Black,
            radius: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct BlockStyle {
    pub padding: f64,
    pub background: Color,
    pub border: BorderStyle,
    pub focused_border: BorderStyle,
}

/// A styled container with no default style.
///
/// Callers must provide a `BlockStyle` when constructing a block.
#[derive(Clone)]
pub struct Block {
    child: Widget,
    pub style: BlockStyle,
    pub focusable: bool,
    pub delta: Option<Delta>,
}

impl Block {
    pub fn new(child: impl WidgetTrait, style: BlockStyle) -> Self {
        Self {
            child: child.as_any(),
            style,
            focusable: false,
            delta: None,
        }
    }
}

#[async_trait]
impl WidgetTrait for Block {
    async fn layout(
        &mut self,
        LayoutInput { focus, slots, .. }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(self.focusable);
        let style = self.style;
        let border_thickness = style.border.thickness.max(style.focused_border.thickness);
        let mut space = Space::uniform(
            self.child.clone(),
            style.padding + border_thickness,
            INTRINSIC_SPACING,
        );
        space.delta = self.delta.clone();
        space.minimum = border_thickness;

        Ok(vec![display!(space)])
    }

    async fn render(
        &mut self,
        RenderInput {
            focus,
            hitbox,
            scene,
            ..
        }: RenderInput<'_, '_>,
    ) -> Result<()> {
        paint_block(scene, hitbox, &self.style, self.focusable && focus.get());
        Ok(())
    }
}

fn paint_block(scene: &mut Scene<'_>, hitbox: Rect, style: &BlockStyle, focused: bool) {
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
