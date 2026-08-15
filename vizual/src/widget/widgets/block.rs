use super::{
    super::{Focus_provider, Widget_trait},
    positioning::space::Space,
};
use crate::{
    component::{Children, context::Component_context},
    geometry::Rect,
    graphics::scene::Scene,
    layouter::{hitbox::Hitbox, objective::Delta},
    slot::manager::Slots,
    state::Store,
    style::Color,
    theme::Theme,
    widget::Widget,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

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
            child: Box::new(child),
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
        _render: crate::Render,
        _theme: Store<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut crate::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(self.focusable);
        let style = self.style;
        let border_thickness = style.border.thickness.max(style.focused_border.thickness);
        let mut space = Space::uniform(self.child.clone(), style.padding + border_thickness, 1);
        space.delta = self.delta.clone();
        space.minimum = border_thickness;

        Ok(vec![display!(space)])
    }

    async fn render(
        &mut self,
        _render: crate::Render,
        _theme: Store<Theme>,
        focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        _text_context: &mut crate::graphics::text::Text_context,
        _context: &crate::component::Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        let focused = self.focusable && focus.get();
        paint_block(scene, hitbox, &self.style, focused);
        Ok(None)
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
