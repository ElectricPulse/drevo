pub mod bar;

use crate::{
    layouter::{EXCESS_SPACE, INTRINSIC_CONTENT},
    macros::display,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    DrevoMsg,
    component::{Children, SharedComponent},
    config::SCROLL_SENSITIVITY,
    constraint,
    event::{Event, KeyCode},
    geometry::{Direction, Point, Rect, Size},
    id,
    state::Store,
    widget::{
        LayoutInput, RenderInput, Widget, WidgetTrait,
        widgets::{
            block::{Block, BlockStyle},
            layout::axis::{Axis, AxisStyle},
        },
    },
};

#[derive(Clone)]
pub struct ScrollContent {
    child: Widget,
    offset: Point,
}

impl ScrollContent {
    pub fn new(child: impl WidgetTrait, offset: Point) -> Self {
        Self {
            child: child.as_any(),
            offset,
        }
    }
}

#[async_trait]
impl WidgetTrait for ScrollContent {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox,
            formula,
            mask,
            slots,
            relayout,
            theme,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        *mask = true;

        let theme = theme.affect(relayout).await?;

        formula.constrain(
            id!(),
            constraint!(hitbox.get_dimension(Direction::Horizontal) >= theme.units.em * 3.0),
        )?;
        formula.constrain(
            id!(),
            constraint!(hitbox.get_dimension(Direction::Vertical) >= theme.units.em * 3.0),
        )?;

        let child = display!(self.child.clone());

        {
            let mut child_lock = child.lock().await?;
            let child_hitbox = &mut child_lock.hitbox;
            child_hitbox.make_independent();

            formula.constrain(
                id!(),
                constraint!(
                    child_hitbox.get_start_position(Direction::Horizontal)
                        == hitbox.get_start_position(Direction::Horizontal) - self.offset.x
                ),
            )?;
            formula.constrain(
                id!(),
                constraint!(
                    child_hitbox.get_start_position(Direction::Vertical)
                        == hitbox.get_start_position(Direction::Vertical) - self.offset.y
                ),
            )?;

            for direction in [Direction::Horizontal, Direction::Vertical] {
                let child_dim = child_hitbox.get_dimension(direction);
                let parent_dim = hitbox.get_dimension(direction);

                formula.maximize(id!(), parent_dim.clone() - child_dim.clone(), EXCESS_SPACE)?;

                let content_growth = formula.variable(format!("content-growth.{direction:?}"))?;
                formula.constrain(
                    format!("{}:{direction:?}:content-growth-ge-0", id!()),
                    constraint!(content_growth >= 0.0),
                )?;
                formula.constrain(
                    format!("{}:{direction:?}:content-growth-ge-child-sub-parent", id!()),
                    constraint!(content_growth >= child_dim - parent_dim),
                )?;
                formula.minimize(id!(), content_growth, INTRINSIC_CONTENT)?;
            }
        }

        Ok(vec![child])
    }
}

#[derive(Clone)]
pub struct Scroll {
    child: Widget,
    root_component: Option<SharedComponent>,
    offset: Store<Point>,
    content_size: Size,
    viewport: Rect,
    scrollbars: Store<ScrollbarVisibility>,
    horizontal_thumb_drag: Store<Option<bar::ThumbDrag>>,
    vertical_thumb_drag: Store<Option<bar::ThumbDrag>>,
    horizontal_thumb_metrics: Store<bar::ThumbMetrics>,
    vertical_thumb_metrics: Store<bar::ThumbMetrics>,
    pub style: Option<BlockStyle>,
    pub block: bool,
}

impl Scroll {
    pub fn new(child: impl WidgetTrait) -> Self {
        Self {
            child: child.as_any(),
            root_component: None,
            offset: Store::new(Point::default()),
            content_size: Size::default(),
            viewport: Rect::default(),
            scrollbars: Store::new(ScrollbarVisibility::default()),
            horizontal_thumb_drag: Store::new(None),
            vertical_thumb_drag: Store::new(None),
            horizontal_thumb_metrics: Store::new(bar::ThumbMetrics::default()),
            vertical_thumb_metrics: Store::new(bar::ThumbMetrics::default()),
            style: None,
            block: true,
        }
    }

    fn maximum_offset(&self) -> Point {
        Point::new(
            (self.content_size.width - self.viewport.size.width).max(0.0),
            (self.content_size.height - self.viewport.size.height).max(0.0),
        )
    }

    async fn clamp_offset(&self) -> Result<bool> {
        let maximum = self.maximum_offset();
        let previous = *self.offset.read().await?;
        let offset = Point::new(
            previous.x.clamp(0.0, maximum.x),
            previous.y.clamp(0.0, maximum.y),
        );
        if offset == previous {
            return Ok(false);
        }
        self.offset.set(offset).await?;
        Ok(true)
    }

    async fn scroll_by(&self, delta: Point) -> Result<bool> {
        let previous = *self.offset.read().await?;
        let maximum = self.maximum_offset();
        let offset = Point::new(
            (previous.x + delta.x).clamp(0.0, maximum.x),
            (previous.y + delta.y).clamp(0.0, maximum.y),
        );
        if offset == previous {
            return Ok(false);
        }
        self.offset.set(offset).await?;
        Ok(true)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ScrollbarVisibility {
    horizontal: bool,
    vertical: bool,
}

impl ScrollbarVisibility {
    fn for_content(content: Size, viewport: Rect) -> Self {
        Self {
            horizontal: content.width > viewport.size.width,
            vertical: content.height > viewport.size.height,
        }
    }
}

#[async_trait]
impl WidgetTrait for Scroll {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            focus,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);

        let offset = *self.offset.affect(relayout.clone()).await?;
        let scrollbars = *self.scrollbars.affect(relayout.clone()).await?;
        let content_widget = ScrollContent::new(self.child.clone(), offset);

        let content_column = match scrollbars.horizontal {
            true => Axis::new(
                Direction::Vertical,
                (
                    content_widget,
                    bar::Scrollbar::new(
                        Direction::Horizontal,
                        offset.x,
                        self.viewport.size.width,
                        self.content_size.width,
                        true,
                        self.offset.clone(),
                        self.horizontal_thumb_drag.clone(),
                        self.horizontal_thumb_metrics.clone(),
                    ),
                ),
            )
            .style(AxisStyle::Gap(0.0)),
            false => Axis::new(Direction::Vertical, (content_widget,)),
        };

        let root_widget = match scrollbars.vertical {
            true => Axis::new(
                Direction::Horizontal,
                (
                    content_column,
                    bar::Scrollbar::new(
                        Direction::Vertical,
                        offset.y,
                        self.viewport.size.height,
                        self.content_size.height,
                        true,
                        self.offset.clone(),
                        self.vertical_thumb_drag.clone(),
                        self.vertical_thumb_metrics.clone(),
                    ),
                ),
            )
            .style(AxisStyle::Gap(0.0)),
            false => Axis::new(Direction::Horizontal, (content_column,)),
        };

        let component = match self.block {
            true => {
                let theme = theme.affect(relayout).await?;
                let block_style = self.style.unwrap_or(theme.specific.paper.block);
                let mut block = Block::new(root_widget, block_style);
                block.focusable = true;
                display!(block)
            }
            false => {
                display!(root_widget)
            }
        };

        self.root_component = Some(component.clone());

        Ok(vec![component])
    }

    async fn render(&mut self, RenderInput { context, .. }: RenderInput<'_, '_>) -> Result<()> {
        if let Some(root_comp) = &self.root_component {
            if let Some((content_comp, child_comp)) =
                find_scroll_content_and_child(root_comp).await?
            {
                let viewport_rect = content_comp
                    .get_hitbox()
                    .await?
                    .get_resolved(context.solution);
                let content_rect = child_comp
                    .get_hitbox()
                    .await?
                    .get_resolved(context.solution);
                self.viewport = viewport_rect;
                self.content_size = content_rect.size;
                let scrollbars = ScrollbarVisibility::for_content(content_rect.size, viewport_rect);
                if *self.scrollbars.read().await? != scrollbars {
                    self.scrollbars.set(scrollbars).await?;
                }
                let _ = self.clamp_offset().await?;
            }
        }

        Ok(())
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<DrevoMsg> {
        let key = input.key;
        let delta = match key.code {
            KeyCode::ArrowLeft => Point::new(-SCROLL_SENSITIVITY, 0.0),
            KeyCode::ArrowRight => Point::new(SCROLL_SENSITIVITY, 0.0),
            KeyCode::ArrowUp => Point::new(0.0, -SCROLL_SENSITIVITY),
            KeyCode::ArrowDown => Point::new(0.0, SCROLL_SENSITIVITY),
            _ => return DrevoMsg::none(),
        };

        let _ = self.scroll_by(delta).await?;
        DrevoMsg::none()
    }

    async fn on_mouse_event(&mut self, input: crate::widget::MouseEvent<'_>) -> Result<DrevoMsg> {
        let event = input.event;
        let Event::Wheel(wheel) = event else {
            return DrevoMsg::none();
        };
        if !self.viewport.contains(wheel.position) {
            return DrevoMsg::none();
        }

        let delta = Point::new(-wheel.delta.x, -wheel.delta.y);
        let delta = match wheel.modifiers.shift {
            true => Point::new(delta.y, 0.0),
            false => delta,
        };

        let _ = self.scroll_by(delta).await?;
        DrevoMsg::none()
    }
}

pub(crate) async fn find_scroll_content_and_child(
    root: &SharedComponent,
) -> Result<Option<(SharedComponent, SharedComponent)>> {
    let mut stack = vec![root.clone()];
    while let Some(current) = stack.pop() {
        let lock = current.lock().await?;
        if lock.mask {
            if let Some(child) = lock.children.first() {
                return Ok(Some((current.clone(), child.clone())));
            }
        }
        for child in &lock.children {
            stack.push(child.clone());
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;
