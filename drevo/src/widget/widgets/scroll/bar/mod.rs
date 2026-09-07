//! Scrollbar layout deliberately takes two passes when overflow changes because the
//! viewport-to-content ratio is nonlinear and cannot be expressed directly in the MILP model.
//!
//! The first solve lays out the content without any newly needed bars. After rendering, `Scroll`
//! measures the resolved content and viewport, updates scrollbar visibility, and requests another
//! layout. That second solve reserves the bar space and computes the thumb from the measured
//! viewport-to-content ratio.

use crate::macros::display;
use crate::{
    DrevoMsg, constraint,
    event::{Event, PointerButton},
    geometry::{Direction, Point, Rect},
    id,
    layouter::priorities::INTRINSIC_CONTENT,
    state::Store,
    theme::Theme,
    widget::{Children, LayoutInput, MouseEvent, RenderInput, WidgetTrait},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default)]
pub struct ThumbMetrics {
    travel: f64,
    maximum_offset: f64,
}

#[derive(Clone, Copy)]
pub struct ThumbDrag {
    pointer_position: f64,
    offset: f64,
}

#[derive(Clone)]
pub struct Scrollbar {
    pub direction: Direction,
    pub offset: f64,
    pub viewport_length: f64,
    pub content_length: f64,
    pub scrollable: bool,
    offset_store: Store<Point>,
    thumb_drag: Store<Option<ThumbDrag>>,
    thumb_metrics: Store<ThumbMetrics>,
}

#[derive(Clone, Copy)]
pub struct ScrollbarStyle {
    pub gutter: f64,
    pub rail: f64,
    pub thumb: f64,
    pub minimum_thumb_length: f64,
}

fn viewport_ratio(viewport_length: f64, content_length: f64) -> f64 {
    let visible_length = content_length.max(viewport_length);
    match visible_length > 0.0 {
        true => viewport_length / visible_length,
        false => 1.0,
    }
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
        offset_store: Store<Point>,
        thumb_drag: Store<Option<ThumbDrag>>,
        thumb_metrics: Store<ThumbMetrics>,
    ) -> Self {
        Self {
            direction,
            offset,
            viewport_length,
            content_length,
            scrollable,
            offset_store,
            thumb_drag,
            thumb_metrics,
        }
    }

    fn track_length(&self, hitbox: Rect) -> f64 {
        match self.direction {
            Direction::Horizontal => hitbox.size.width,
            Direction::Vertical => hitbox.size.height,
        }
    }

    fn cross_axis_length(&self, hitbox: Rect) -> f64 {
        match self.direction {
            Direction::Horizontal => hitbox.size.height,
            Direction::Vertical => hitbox.size.width,
        }
    }

    fn thumb_length(&self, track_length: f64, style: ScrollbarStyle) -> f64 {
        (track_length * viewport_ratio(self.viewport_length, self.content_length))
            .clamp(style.minimum_thumb_length.min(track_length), track_length)
    }

    fn rail(&self, hitbox: Rect, size: f64) -> Rect {
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
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let theme = theme.affect(relayout).await?;
        let style = ScrollbarStyle::new(&theme);
        hitbox
            .set_static_dimension(formula, self.direction.flip(), style.gutter)
            .await?;

        let thumb = Thumb {
            direction: self.direction,
            offset: self.offset,
            viewport_length: self.viewport_length,
            content_length: self.content_length,
            scrollable: self.scrollable,
            offset_store: self.offset_store.clone(),
            drag: self.thumb_drag.clone(),
            metrics: self.thumb_metrics.clone(),
        };
        Ok(vec![display!(thumb)])
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
        let theme = theme.affect(rerender).await?;
        let style = ScrollbarStyle::new(&theme);
        let track_length = self.track_length(hitbox);
        if track_length <= 0.0 {
            return Ok(());
        }

        let rail_size = style.rail.min(self.cross_axis_length(hitbox));
        scene.fill_rounded_rect(
            self.rail(hitbox, rail_size),
            theme.semantic.border,
            rail_size / 2.0,
        );

        let thumb_length = self.thumb_length(track_length, style);
        self.thumb_metrics
            .set(ThumbMetrics {
                travel: (track_length - thumb_length).max(0.0),
                maximum_offset: (self.content_length - self.viewport_length).max(0.0),
            })
            .await?;
        Ok(())
    }
}

/// The separately hit-tested movable part of a [`Scrollbar`].
#[derive(Clone)]
struct Thumb {
    direction: Direction,
    offset: f64,
    viewport_length: f64,
    content_length: f64,
    scrollable: bool,
    offset_store: Store<Point>,
    drag: Store<Option<ThumbDrag>>,
    metrics: Store<ThumbMetrics>,
}

impl Thumb {
    fn pointer_position(&self, point: Point) -> f64 {
        match self.direction {
            Direction::Horizontal => point.x,
            Direction::Vertical => point.y,
        }
    }

    fn offset_in_direction(&self, offset: Point) -> f64 {
        match self.direction {
            Direction::Horizontal => offset.x,
            Direction::Vertical => offset.y,
        }
    }

    fn set_offset_in_direction(&self, offset: &mut Point, value: f64) {
        match self.direction {
            Direction::Horizontal => offset.x = value,
            Direction::Vertical => offset.y = value,
        }
    }
}

#[async_trait]
impl WidgetTrait for Thumb {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            parent,
            formula,
            relayout,
            theme,
            focus,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(self.scrollable);
        hitbox.make_independent();

        let theme = theme.affect(relayout).await?;
        let style = ScrollbarStyle::new(&theme);
        let parent_dimension = parent.get_dimension(self.direction);
        let thumb_dimension = hitbox.get_dimension(self.direction);
        let ratio = viewport_ratio(self.viewport_length, self.content_length);
        formula.constrain(
            id!(),
            constraint!(thumb_dimension.clone() >= parent_dimension.clone() * ratio),
        )?;
        formula.constrain(
            id!(),
            constraint!(thumb_dimension.clone() >= style.minimum_thumb_length),
        )?;
        formula.constrain(
            id!(),
            constraint!(thumb_dimension.clone() <= parent_dimension.clone()),
        )?;
        formula.minimize(id!(), thumb_dimension.clone(), INTRINSIC_CONTENT)?;

        hitbox
            .set_static_dimension(formula, self.direction.flip(), style.thumb)
            .await?;

        let cross_direction = self.direction.flip();
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_start_position(cross_direction)
                    == parent.get_start_position(cross_direction)
                        + (parent.get_dimension(cross_direction)
                            - hitbox.get_dimension(cross_direction))
                            / 2.0
            ),
        )?;

        let maximum_offset = (self.content_length - self.viewport_length).max(0.0);
        let progress = match maximum_offset > 0.0 {
            true => self.offset / maximum_offset,
            false => 0.0,
        };
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_start_position(self.direction)
                    == parent.get_start_position(self.direction)
                        + (parent_dimension - thumb_dimension) * progress
            ),
        )?;

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
        let theme = theme.affect(rerender).await?;
        let color = match self.scrollable {
            true => theme.semantic.text.muted,
            false => theme.semantic.text.muted.darken(10),
        };
        let size = match self.direction {
            Direction::Horizontal => hitbox.size.height,
            Direction::Vertical => hitbox.size.width,
        };
        scene.fill_rounded_rect(hitbox, color, size / 2.0);
        Ok(())
    }

    async fn on_mouse_event(&mut self, input: MouseEvent<'_>) -> Result<DrevoMsg> {
        match input.event {
            Event::Pointer(pointer)
                if pointer.button == PointerButton::Primary && self.scrollable =>
            {
                let offset = *self.offset_store.read().await?;
                self.drag
                    .set(Some(ThumbDrag {
                        pointer_position: self.pointer_position(pointer.position),
                        offset: self.offset_in_direction(offset),
                    }))
                    .await?;
            }
            Event::PointerMoved(pointer) => {
                let Some(drag) = *self.drag.read().await? else {
                    return DrevoMsg::none();
                };
                let metrics = *self.metrics.read().await?;
                if metrics.travel <= 0.0 || metrics.maximum_offset <= 0.0 {
                    return DrevoMsg::none();
                }
                let offset = (drag.offset
                    + (self.pointer_position(*pointer) - drag.pointer_position)
                        * metrics.maximum_offset
                        / metrics.travel)
                    .clamp(0.0, metrics.maximum_offset);
                let mut current = *self.offset_store.read().await?;
                if self.offset_in_direction(current) != offset {
                    self.set_offset_in_direction(&mut current, offset);
                    self.offset_store.set(current).await?;
                }
            }
            Event::PointerReleased(pointer) if pointer.button == PointerButton::Primary => {
                self.drag.set(None).await?;
            }
            _ => {}
        }

        DrevoMsg::none()
    }
}
