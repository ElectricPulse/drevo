use crate::{
    geometry::{Direction, Point, Rect, Size},
    graphics::scene::Scene,
    theme::Theme,
    widget::{Children, LayoutInput, RenderInput, WidgetTrait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Scrollbar {
    pub direction: Direction,
    pub offset: f64,
    pub viewport_length: f64,
    pub content_length: f64,
    pub scrollable: bool,
}

#[derive(Clone, Copy)]
pub struct ScrollbarStyle {
    pub gutter: f64,
    pub rail: f64,
    pub thumb: f64,
    pub minimum_thumb_length: f64,
}

impl ScrollbarStyle {
    pub fn new(theme: &Theme) -> Self {
        Self {
            gutter: theme.units.em * 0.75,
            rail: theme.units.em * 0.25,
            thumb: theme.units.em * 0.5,
            minimum_thumb_length: theme.units.em * 1.5,
        }
    }
}

impl Scrollbar {
    pub fn new(
        direction: Direction,
        offset: f64,
        viewport_length: f64,
        content_length: f64,
        scrollable: bool,
    ) -> Self {
        Self {
            direction,
            offset,
            viewport_length,
            content_length,
            scrollable,
        }
    }

    fn paint(&self, scene: &mut Scene<'_>, hitbox: Rect, theme: &Theme, style: ScrollbarStyle) {
        let track_length = match self.direction {
            Direction::Horizontal => hitbox.size.width,
            Direction::Vertical => hitbox.size.height,
        };
        if track_length <= 0.0 {
            return;
        }

        let rail_size = style.rail.min(self.cross_axis_length(hitbox));
        let rail = self.with_cross_axis_size(hitbox, rail_size);
        scene.fill_rounded_rect(rail, theme.semantic.border, rail_size / 2.0);

        let thumb_length = (track_length * self.viewport_length
            / self.content_length.max(self.viewport_length))
        .clamp(style.minimum_thumb_length.min(track_length), track_length);
        let travel = (track_length - thumb_length).max(0.0);
        let maximum = (self.content_length - self.viewport_length).max(0.0);
        let thumb_offset = match maximum > 0.0 {
            true => travel * self.offset / maximum,
            false => 0.0,
        };
        let thumb_size = style.thumb.min(self.cross_axis_length(hitbox));
        let thumb = match self.direction {
            Direction::Horizontal => Rect {
                origin: Point::new(
                    hitbox.origin.x + thumb_offset,
                    hitbox.origin.y + (hitbox.size.height - thumb_size) / 2.0,
                ),
                size: Size::new(thumb_length, thumb_size),
            },
            Direction::Vertical => Rect {
                origin: Point::new(
                    hitbox.origin.x + (hitbox.size.width - thumb_size) / 2.0,
                    hitbox.origin.y + thumb_offset,
                ),
                size: Size::new(thumb_size, thumb_length),
            },
        };
        let thumb_color = match self.scrollable {
            true => theme.semantic.text.muted,
            false => theme.semantic.text.muted.darken(10),
        };
        scene.fill_rounded_rect(thumb, thumb_color, thumb_size / 2.0);
    }

    fn with_cross_axis_size(&self, hitbox: Rect, size: f64) -> Rect {
        match self.direction {
            Direction::Horizontal => Rect::new(
                hitbox.origin.x,
                hitbox.origin.y + (hitbox.size.height - size) / 2.0,
                hitbox.size.width,
                size,
            ),
            Direction::Vertical => Rect::new(
                hitbox.origin.x + (hitbox.size.width - size) / 2.0,
                hitbox.origin.y,
                size,
                hitbox.size.height,
            ),
        }
    }

    fn cross_axis_length(&self, hitbox: Rect) -> f64 {
        match self.direction {
            Direction::Horizontal => hitbox.size.height,
            Direction::Vertical => hitbox.size.width,
        }
    }
}

#[async_trait]
impl WidgetTrait for Scrollbar {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            formula,
            relayout,
            theme,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let style = ScrollbarStyle::new(&theme);
        match self.direction {
            Direction::Horizontal => {
                hitbox
                    .set_static_dimension(formula, Direction::Vertical, style.gutter)
                    .await?;
            }
            Direction::Vertical => {
                hitbox
                    .set_static_dimension(formula, Direction::Horizontal, style.gutter)
                    .await?;
            }
        }
        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        RenderInput {
            rerender,
            theme,
            hitbox,
            scene,
            ..
        }: RenderInput<'_, '_>,
    ) -> Result<()> {
        let loaded_theme = (*theme.affect(rerender).await?).clone();
        let style = ScrollbarStyle::new(&loaded_theme);
        self.paint(scene, hitbox, &loaded_theme, style);
        Ok(())
    }
}
