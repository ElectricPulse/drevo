pub mod bar;

use crate::macros::display;
use async_trait::async_trait;
use color_eyre::eyre::Result;

use crate::{
    VizualMsg,
    component::{Children, SharedComponent},
    constraint,
    event::{Event, KeyCode, WheelDelta},
    geometry::{Direction, Point, Rect, Size},
    widget::{
        LayoutInput, RenderInput, Widget, WidgetTrait,
        widgets::{
            block::{Block, BlockStyle},
            layout::axis::{Axis, AxisStyle},
        },
    },
};

const SCROLL_STEP: f64 = 115.0;

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
            problem,
            mask,
            slots,
            relayout,
            theme,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        *mask = true;

        let theme = theme.affect(relayout).await?;

        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Horizontal) >= theme.units.em * 3.0
            ))
            .await?;
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) >= theme.units.em * 3.0
            ))
            .await?;

        let child = display!(self.child.clone());

        {
            let mut child_lock = child.lock().await?;
            let child_hitbox = &mut child_lock.hitbox;
            child_hitbox.make_independent();

            problem
                .constrain(constraint!(
                    child_hitbox.get_start_position(Direction::Horizontal)
                        == hitbox.get_start_position(Direction::Horizontal) - self.offset.x
                ))
                .await?;
            problem
                .constrain(constraint!(
                    child_hitbox.get_start_position(Direction::Vertical)
                        == hitbox.get_start_position(Direction::Vertical) - self.offset.y
                ))
                .await?;
        }

        Ok(vec![child])
    }
}

#[derive(Clone)]
pub struct Scroll {
    child: Widget,
    root_component: Option<SharedComponent>,
    offset: Point,
    content_size: Size,
    viewport: Rect,
    pub style: Option<BlockStyle>,
    pub block: bool,
}

impl Scroll {
    pub fn new(child: impl WidgetTrait) -> Self {
        Self {
            child: child.as_any(),
            root_component: None,
            offset: Point::default(),
            content_size: Size::default(),
            viewport: Rect::default(),
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

    fn clamp_offset(&mut self) {
        let maximum = self.maximum_offset();
        self.offset.x = self.offset.x.clamp(0.0, maximum.x);
        self.offset.y = self.offset.y.clamp(0.0, maximum.y);
    }

    fn scroll_by(&mut self, delta: Point) -> bool {
        let previous = self.offset;
        self.offset.x += delta.x;
        self.offset.y += delta.y;
        self.clamp_offset();
        self.offset != previous
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

        let content_widget = ScrollContent::new(self.child.clone(), self.offset);

        let has_vertical =
            self.content_size.height > self.viewport.size.height && self.viewport.size.height > 0.0;
        let has_horizontal =
            self.content_size.width > self.viewport.size.width && self.viewport.size.width > 0.0;

        let content_column: Widget = match has_horizontal {
            true => {
                let h_bar = bar::Scrollbar::new(
                    Direction::Horizontal,
                    self.offset.x,
                    self.viewport.size.width,
                    self.content_size.width,
                );
                let mut v_axis = Axis::new(Direction::Vertical, (content_widget, h_bar));
                v_axis.style.set(AxisStyle::Gap(0.0));
                Box::new(v_axis)
            }
            false => Box::new(content_widget),
        };

        let root_widget: Widget = match has_vertical {
            true => {
                let v_bar = bar::Scrollbar::new(
                    Direction::Vertical,
                    self.offset.y,
                    self.viewport.size.height,
                    self.content_size.height,
                );
                let mut h_axis = Axis::new(Direction::Horizontal, (content_column, v_bar));
                h_axis.style.set(AxisStyle::Gap(0.0));
                Box::new(h_axis)
            }
            false => content_column,
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

    async fn render(
        &mut self,
        RenderInput { context, .. }: RenderInput<'_, '_>,
    ) -> Result<()> {
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
            }
        }

        Ok(())
    }

    async fn on_key_press(&mut self, input: crate::widget::KeyPress<'_>) -> Result<VizualMsg> {
        let key = input.key;
        let relayout = input.relayout;
        let delta = match key.code {
            KeyCode::ArrowLeft => Point::new(-SCROLL_STEP, 0.0),
            KeyCode::ArrowRight => Point::new(SCROLL_STEP, 0.0),
            KeyCode::ArrowUp => Point::new(0.0, -SCROLL_STEP),
            KeyCode::ArrowDown => Point::new(0.0, SCROLL_STEP),
            _ => return VizualMsg::none(),
        };

        match self.scroll_by(delta) {
            true => {
                relayout.send();
                VizualMsg::none()
            }
            false => VizualMsg::none(),
        }
    }

    async fn on_other_event(
        &mut self,
        input: crate::widget::OtherEvent<'_>,
    ) -> Result<VizualMsg> {
        let event = input.event;
        let relayout = input.relayout;
        let Event::Wheel(wheel) = event else {
            return VizualMsg::none();
        };
        if !self.viewport.contains(wheel.position) {
            return VizualMsg::none();
        }

        let delta = match wheel.delta {
            WheelDelta::Lines(delta) => Point::new(-delta.x * SCROLL_STEP, -delta.y * SCROLL_STEP),
            WheelDelta::LogicalPixels(delta) => Point::new(-delta.x, -delta.y),
        };
        let delta = match wheel.modifiers.shift {
            true => Point::new(delta.y, 0.0),
            false => delta,
        };

        match self.scroll_by(delta) {
            true => {
                relayout.send();
                VizualMsg::none()
            }
            false => VizualMsg::none(),
        }
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
