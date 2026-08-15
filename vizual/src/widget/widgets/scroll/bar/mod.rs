use crate::{
    geometry::{Direction, Point, Rect, Size},
    graphics::scene::Scene,
    theme::Theme,
};

#[cfg(test)]
mod tests;

pub(super) struct Scrollbars {
    viewport: Rect,
    horizontal: Option<Scrollbar>,
    vertical: Option<Scrollbar>,
    style: Scrollbar_style,
}

struct Scrollbar {
    direction: Direction,
    track: Rect,
    viewport_length: f64,
    content_length: f64,
}

#[derive(Clone, Copy)]
struct Scrollbar_style {
    gutter: f64,
    rail: f64,
    thumb: f64,
    minimum_thumb_length: f64,
}

impl Scrollbars {
    pub(super) fn new(bounds: Rect, content: Size, theme: &Theme) -> Self {
        let style = Scrollbar_style::new(theme);
        let mut horizontal = false;
        let mut vertical = false;

        loop {
            let viewport = viewport(bounds, horizontal, vertical, style.gutter);
            let next_horizontal = content.width > viewport.size.width;
            let next_vertical = content.height > viewport.size.height;
            if next_horizontal == horizontal && next_vertical == vertical {
                break;
            }
            horizontal = next_horizontal;
            vertical = next_vertical;
        }

        let viewport = viewport(bounds, horizontal, vertical, style.gutter);
        Self {
            horizontal: horizontal.then_some(Scrollbar {
                direction: Direction::Horizontal,
                track: Rect::new(
                    bounds.origin.x,
                    viewport.bottom(),
                    viewport.size.width,
                    (bounds.size.height - viewport.size.height).max(0.0),
                ),
                viewport_length: viewport.size.width,
                content_length: content.width,
            }),
            vertical: vertical.then_some(Scrollbar {
                direction: Direction::Vertical,
                track: Rect::new(
                    viewport.right(),
                    bounds.origin.y,
                    (bounds.size.width - viewport.size.width).max(0.0),
                    viewport.size.height,
                ),
                viewport_length: viewport.size.height,
                content_length: content.height,
            }),
            viewport,
            style,
        }
    }

    pub(super) fn viewport(&self) -> Rect {
        self.viewport
    }

    pub(super) fn paint(&self, scene: &mut Scene<'_>, offset: Point, theme: &Theme) {
        for scrollbar in [&self.horizontal, &self.vertical].into_iter().flatten() {
            scrollbar.paint(scene, offset, theme, self.style);
        }
    }
}

impl Scrollbar {
    fn paint(&self, scene: &mut Scene<'_>, position: Point, theme: &Theme, style: Scrollbar_style) {
        let track_length = match self.direction {
            Direction::Horizontal => self.track.size.width,
            Direction::Vertical => self.track.size.height,
        };
        if track_length <= 0.0 {
            return;
        }
        let rail_size = style.rail.min(self.cross_axis_length());
        let rail = self.with_cross_axis_size(rail_size);
        scene.fill_rounded_rect(rail, theme.semantic.border, rail_size / 2.0);

        let thumb_length = (track_length * self.viewport_length
            / self.content_length.max(self.viewport_length))
        .clamp(style.minimum_thumb_length.min(track_length), track_length);
        let travel = (track_length - thumb_length).max(0.0);
        let maximum = (self.content_length - self.viewport_length).max(0.0);
        let position = match self.direction {
            Direction::Horizontal => position.x,
            Direction::Vertical => position.y,
        };
        let thumb_offset = match maximum > 0.0 {
            true => travel * position / maximum,
            false => 0.0,
        };
        let thumb_size = style.thumb.min(self.cross_axis_length());
        let thumb = match self.direction {
            Direction::Horizontal => Rect {
                origin: Point::new(
                    self.track.origin.x + thumb_offset,
                    self.track.origin.y + (self.track.size.height - thumb_size) / 2.0,
                ),
                size: Size::new(thumb_length, thumb_size),
            },
            Direction::Vertical => Rect {
                origin: Point::new(
                    self.track.origin.x + (self.track.size.width - thumb_size) / 2.0,
                    self.track.origin.y + thumb_offset,
                ),
                size: Size::new(thumb_size, thumb_length),
            },
        };
        scene.fill_rounded_rect(thumb, theme.semantic.text.muted, thumb_size / 2.0);
    }

    fn with_cross_axis_size(&self, size: f64) -> Rect {
        match self.direction {
            Direction::Horizontal => Rect::new(
                self.track.origin.x,
                self.track.origin.y + (self.track.size.height - size) / 2.0,
                self.track.size.width,
                size,
            ),
            Direction::Vertical => Rect::new(
                self.track.origin.x + (self.track.size.width - size) / 2.0,
                self.track.origin.y,
                size,
                self.track.size.height,
            ),
        }
    }

    fn cross_axis_length(&self) -> f64 {
        match self.direction {
            Direction::Horizontal => self.track.size.height,
            Direction::Vertical => self.track.size.width,
        }
    }
}

impl Scrollbar_style {
    fn new(theme: &Theme) -> Self {
        Self {
            gutter: theme.units.em * 0.75,
            rail: theme.units.em * 0.25,
            thumb: theme.units.em * 0.5,
            minimum_thumb_length: theme.units.em * 1.5,
        }
    }
}

fn viewport(bounds: Rect, horizontal: bool, vertical: bool, gutter: f64) -> Rect {
    Rect {
        origin: bounds.origin,
        size: Size::new(
            (bounds.size.width - f64::from(vertical) * gutter).max(0.0),
            (bounds.size.height - f64::from(horizontal) * gutter).max(0.0),
        ),
    }
}
