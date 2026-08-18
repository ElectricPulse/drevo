pub mod bar;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, Shared_component},
    constraint,
    event::{Event, Key_code, Key_event, Wheel_delta},
    geometry::{Direction, Point, Rect, Size},
    state::State,
    widget::{
        Layout_input, Render_input, Widget, Widget_trait,
        widgets::layout::axis::{Axis, Axis_style},
    },
};

const SCROLL_STEP: f64 = 130.0;

#[derive(Clone)]
pub struct Scroll_content {
    child: Widget,
    offset: Point,
}

impl Scroll_content {
    pub fn new(child: impl Widget_trait, offset: Point) -> Self {
        Self {
            child: child.as_any(),
            offset,
        }
    }
}

#[async_trait]
impl Widget_trait for Scroll_content {
    async fn layout(
        &mut self,
        Layout_input {
            hitbox,
            problem,
            mask,
            slots,
            render,
            theme,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        *mask = true;

        let theme = theme.affect(render).await?;

        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Horizontal) >= theme.units.em * 5.0
            ))
            .await?;
        problem
            .constrain(constraint!(
                hitbox.get_dimension(Direction::Vertical) >= theme.units.em * 5.0
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
    root_component: Option<Shared_component>,
    offset: Point,
    content_size: Size,
    viewport: Rect,
}

impl Scroll {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            child: child.as_any(),
            root_component: None,
            offset: Point::default(),
            content_size: Size::default(),
            viewport: Rect::default(),
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
impl Widget_trait for Scroll {
    async fn layout(
        &mut self,
        Layout_input {
            focus,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        focus.set_interactive(true);

        let content_widget = Scroll_content::new(self.child.clone(), self.offset);

        let has_vertical = self.content_size.height > self.viewport.size.height && self.viewport.size.height > 0.0;
        let has_horizontal = self.content_size.width > self.viewport.size.width && self.viewport.size.width > 0.0;

        let content_column: Widget = match has_horizontal {
            true => {
                let h_bar = bar::Scrollbar::new(
                    Direction::Horizontal,
                    self.offset.x,
                    self.viewport.size.width,
                    self.content_size.width,
                );
                let mut v_axis = Axis::new(Direction::Vertical, (content_widget, h_bar));
                v_axis.style.set(Axis_style::Gap(0.0));
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
                h_axis.style.set(Axis_style::Gap(0.0));
                Box::new(h_axis)
            }
            false => content_column,
        };

        let component = display!(root_widget);
        self.root_component = Some(component.clone());

        Ok(vec![component])
    }

    async fn render(
        &mut self,
        Render_input {
            focus,
            context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        focus.set_interactive(true);

        if let Some(root_comp) = &self.root_component {
            if let Some((content_comp, child_comp)) = find_scroll_content_and_child(root_comp).await? {
                let viewport_rect = content_comp.get_hitbox().await?.get_resolved(context.solution);
                let content_rect = child_comp.get_hitbox().await?.get_resolved(context.solution);
                self.viewport = viewport_rect;
                self.content_size = content_rect.size;
            }
        }

        self.clamp_offset();
        Ok(())
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        let delta = match key.code {
            Key_code::Arrow_left => Point::new(-SCROLL_STEP, 0.0),
            Key_code::Arrow_right => Point::new(SCROLL_STEP, 0.0),
            Key_code::Arrow_up => Point::new(0.0, -SCROLL_STEP),
            Key_code::Arrow_down => Point::new(0.0, SCROLL_STEP),
            _ => return Vizual_msg::none(),
        };

        match self.scroll_by(delta) {
            true => Vizual_msg::new(Vizual_command::Layout),
            false => Vizual_msg::none(),
        }
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        let Event::Wheel(wheel) = event else {
            return Vizual_msg::none();
        };
        if !self.viewport.contains(wheel.position) {
            return Vizual_msg::none();
        }

        let delta = match wheel.delta {
            Wheel_delta::Lines(delta) => Point::new(-delta.x * SCROLL_STEP, -delta.y * SCROLL_STEP),
            Wheel_delta::Logical_pixels(delta) => Point::new(-delta.x, -delta.y),
        };
        let delta = match wheel.modifiers.shift {
            true => Point::new(delta.y, 0.0),
            false => delta,
        };

        match self.scroll_by(delta) {
            true => Vizual_msg::new(Vizual_command::Layout),
            false => Vizual_msg::none(),
        }
    }
}

async fn find_scroll_content_and_child(
    root: &Shared_component,
) -> Result<Option<(Shared_component, Shared_component)>> {
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
