use async_trait::async_trait;
use color_eyre::eyre::Result;

use super::{
    super::{Focus_provider, Widget_trait},
    text_viewport::Text_viewport,
};
use crate::{
    Vizual_command, Vizual_msg,
    config::SCROLLBAR_SIZE,
    display::Display,
    event::{Event, Key_code, Key_event, Wheel_delta},
    geometry::{Point, Rect, Size},
    layouter::hitbox::Hitbox,
    style::Color,
};

pub struct Paragraph {
    viewport: Text_viewport,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Paragraph {
    pub fn new() -> Self {
        Self {
            viewport: Text_viewport::new(),
        }
    }

    pub fn set_content(&mut self, content: String) {
        self.viewport.set_content(&content);
    }
}

#[async_trait]
impl Widget_trait for Paragraph {
    async fn render(
        &mut self,
        _theme: crate::state::State<crate::theme::Theme>,
        focus: &mut Focus_provider,
        hitbox: Rect,
        display: &mut Display<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);
        let inner = hitbox;
        self.viewport.prepare(display, inner.size);
        let content_size = self.viewport.content_size();
        let mut vertical = false;
        let mut horizontal = false;

        loop {
            let viewport = Size::new(
                (inner.size.width - f64::from(vertical) * SCROLLBAR_SIZE).max(0.0),
                (inner.size.height - f64::from(horizontal) * SCROLLBAR_SIZE).max(0.0),
            );
            let next_vertical = content_size.height > viewport.height;
            let next_horizontal = content_size.width > viewport.width;
            if next_vertical == vertical && next_horizontal == horizontal {
                break;
            }
            vertical = next_vertical;
            horizontal = next_horizontal;
        }

        let content_hitbox = Rect {
            origin: inner.origin,
            size: Size::new(
                (inner.size.width - f64::from(vertical) * SCROLLBAR_SIZE).max(0.0),
                (inner.size.height - f64::from(horizontal) * SCROLLBAR_SIZE).max(0.0),
            ),
        };
        self.viewport.prepare(display, content_hitbox.size);
        self.viewport.paint(display, content_hitbox.origin);

        if vertical {
            paint_scrollbar(
                display,
                Rect::new(
                    inner.right() - SCROLLBAR_SIZE,
                    inner.origin.y,
                    SCROLLBAR_SIZE,
                    content_hitbox.size.height,
                ),
                self.viewport.offset().y,
                self.viewport.maximum_offset().y,
                content_hitbox.size.height,
                content_size.height,
                true,
            );
        }
        if horizontal {
            paint_scrollbar(
                display,
                Rect::new(
                    inner.origin.x,
                    inner.bottom() - SCROLLBAR_SIZE,
                    content_hitbox.size.width,
                    SCROLLBAR_SIZE,
                ),
                self.viewport.offset().x,
                self.viewport.maximum_offset().x,
                content_hitbox.size.width,
                content_size.width,
                false,
            );
        }

        Ok(None)
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        let step = self.viewport.line_step();
        match key.code {
            Key_code::Arrow_up => self.viewport.scroll_y(-step),
            Key_code::Arrow_down => self.viewport.scroll_y(step),
            Key_code::Arrow_left => self.viewport.scroll_x(-step),
            Key_code::Arrow_right => self.viewport.scroll_x(step),
            Key_code::Page_up => self
                .viewport
                .scroll_y(-self.viewport.viewport_size().height),
            Key_code::Page_down => self.viewport.scroll_y(self.viewport.viewport_size().height),
            Key_code::Home => self.viewport.jump_to_top(),
            Key_code::End => self.viewport.jump_to_bottom(),
            _ => return Vizual_msg::none(),
        }
        Vizual_msg::new(Vizual_command::Layout)
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let Event::Wheel(wheel) = event else {
            return Vizual_msg::none();
        };
        let step = self.viewport.line_step();
        match wheel.delta {
            Wheel_delta::Lines(delta) => {
                self.viewport.scroll_x(-delta.x * step);
                self.viewport.scroll_y(-delta.y * step * 3.0);
            }
            Wheel_delta::Logical_pixels(delta) => {
                self.viewport.scroll_x(-delta.x);
                self.viewport.scroll_y(-delta.y);
            }
        }
        Vizual_msg::new(Vizual_command::Layout)
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_scrollbar(
    display: &mut Display<'_>,
    track: Rect,
    position: f64,
    maximum: f64,
    viewport_length: f64,
    content_length: f64,
    vertical: bool,
) {
    display.fill_rect(track, Color::Dark_gray);
    let track_length = match vertical {
        true => track.size.height,
        false => track.size.width,
    };
    let thumb_length = (track_length * viewport_length / content_length.max(viewport_length))
        .clamp(SCROLLBAR_SIZE, track_length);
    let travel = (track_length - thumb_length).max(0.0);
    let offset = match maximum > 0.0 {
        true => travel * position / maximum,
        false => 0.0,
    };
    let thumb = match vertical {
        true => Rect {
            origin: Point::new(track.origin.x, track.origin.y + offset),
            size: Size::new(track.size.width, thumb_length),
        },
        false => Rect {
            origin: Point::new(track.origin.x + offset, track.origin.y),
            size: Size::new(thumb_length, track.size.height),
        },
    };
    display.fill_rect(thumb, Color::White);
}
