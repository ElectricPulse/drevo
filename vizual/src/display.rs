use vello::{
    Scene,
    kurbo::{Affine, Rect as Kurbo_rect, RoundedRect, Stroke},
    peniko::Fill,
};

use crate::{geometry::Rect, style::Color, text::Text_context};

pub struct Display<'a> {
    pub(crate) scene: &'a mut Scene,
    pub(crate) text_context: &'a mut Text_context,
}

impl<'a> Display<'a> {
    pub(crate) fn new(scene: &'a mut Scene, text_context: &'a mut Text_context) -> Self {
        Self {
            scene,
            text_context,
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if rect.size.is_empty() {
            return;
        }

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &to_kurbo_rect(rect),
        );
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f64) {
        if rect.size.is_empty() || width <= 0.0 {
            return;
        }

        self.scene.stroke(
            &Stroke::new(width),
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &to_kurbo_rect(rect),
        );
    }

    pub fn fill_rounded_rect(&mut self, rect: Rect, color: Color, radius: f64) {
        if rect.size.is_empty() {
            return;
        }

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &RoundedRect::from_rect(to_kurbo_rect(rect), radius),
        );
    }

    pub fn stroke_rounded_rect(&mut self, rect: Rect, color: Color, width: f64, radius: f64) {
        if rect.size.is_empty() || width <= 0.0 {
            return;
        }

        self.scene.stroke(
            &Stroke::new(width),
            Affine::IDENTITY,
            color.to_peniko(),
            None,
            &RoundedRect::from_rect(to_kurbo_rect(rect), radius),
        );
    }
}

fn to_kurbo_rect(rect: Rect) -> Kurbo_rect {
    Kurbo_rect::new(rect.origin.x, rect.origin.y, rect.right(), rect.bottom())
}
