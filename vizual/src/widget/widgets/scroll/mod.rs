mod bar;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use vizual_macros::display;

use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, Render_context, Shared_component, context::Component_context},
    event::{Event, Key_code, Key_event, Wheel_delta},
    geometry::{Direction, Point, Rect, Size},
    graphics::{scene::Scene, text::Text_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::{Focus_provider, Layout_input, Render_input, Widget, Widget_trait},
};

use self::bar::Scrollbars;

const SCROLL_STEP: f64 = 130.0;

#[derive(Clone)]
pub struct Scroll {
    child: Widget,
    child_component: Option<Shared_component>,
    offset: Point,
    content_size: Size,
    viewport: Rect,
}

impl Scroll {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            child: Box::new(child),
            child_component: None,
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
            hitbox,
            problem,
            slots,
            mask,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        focus.set_active(true);
        *mask = true;

        let child = display!(self.child.clone());
        self.child_component = Some(child.clone());

        {
            let mut child_lock = child.lock().await?;
            let child_hitbox = &mut child_lock.hitbox;
            child_hitbox.make_independent();

            problem
                .constrain(crate::constraint!(
                    child_hitbox.get_start_position(Direction::Horizontal)
                        == hitbox.get_start_position(Direction::Horizontal) - self.offset.x
                ))
                .await?;
            problem
                .constrain(crate::constraint!(
                    child_hitbox.get_start_position(Direction::Vertical)
                        == hitbox.get_start_position(Direction::Vertical) - self.offset.y
                ))
                .await?;
        }

        Ok(vec![child])
    }

    async fn render(
        &mut self,
        Render_input {
            render,
            theme,
            focus,
            hitbox,
            scene,
            context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        focus.set_active(true);

        if let Some(child) = &self.child_component {
            let content = child.get_hitbox().await?.get_resolved(context.solution);
            self.content_size = content.size;
        }

        let loaded_theme = (*theme.affect(render).await?).clone();
        let scrollbars = Scrollbars::new(hitbox, self.content_size, &loaded_theme);
        self.viewport = scrollbars.viewport();
        self.clamp_offset();

        scrollbars.paint(scene, self.offset, &loaded_theme);

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

#[cfg(test)]
mod tests;
