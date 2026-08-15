mod bar;
mod frame;

use async_trait::async_trait;
use color_eyre::eyre::{ContextCompat, Result};
use vello::{Scene as Vello_scene, kurbo::Affine};
use vizual_macros::display;

use crate::{
    Vizual_command, Vizual_msg,
    component::{Children, Render_context, Shared_component, context::Component_context},
    event::{Event, Key_code, Key_event, Pointer_event, Wheel_delta},
    geometry::{Point, Rect, Size},
    graphics::{scene::Scene, text::Text_context},
    layouter::{Solution, hitbox::Hitbox},
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::{Focus_provider, Shared_widget, Widget, Widget_trait, widgets::block::Block},
};

use self::bar::Scrollbars;
use self::frame::Frame;

const SCROLL_STEP: f64 = 130.0;

#[derive(Clone)]
pub struct Scroll {
    content: Shared_widget<Scroll_content>,
}

impl Scroll {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            content: Scroll_content::new(child).into_shared(),
        }
    }
}

#[derive(Clone)]
struct Scroll_content {
    child: Widget,
    frame: Option<Shared_component>,
    solution: Option<Solution>,
    offset: Point,
    content_size: Size,
    viewport: Rect,
}

impl Scroll_content {
    fn new(child: impl Widget_trait) -> Self {
        Self {
            child: Box::new(child),
            frame: None,
            solution: None,
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
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let mut style = theme.affect(render).await?.specific.paper.block;
        style.padding = 0.0;
        let mut block = Block::new(self.content.clone(), style);
        block.focusable = true;

        Ok(vec![display!(block)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        self.content.lock().await?.on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        self.content.lock().await?.on_other_event(event).await
    }
}

#[async_trait]
impl Widget_trait for Scroll_content {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let frame = display!(Frame::new(self.child.clone()));
        frame.lock().await?.logical = true;
        self.frame = Some(frame.clone());
        self.solution = None;

        // Layout deliberately remains in Vizual's ordinary component tree. Future state
        // management can then relayout only invalidated branches. Rendering is expected to keep
        // walking from the root each frame, so manually rendering this isolated subtree into a
        // temporary scene does not undermine that selective-layout design.
        Ok(vec![frame])
    }

    async fn render(
        &mut self,
        render: crate::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        text_context: &mut Text_context,
        context: &Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        let mut frame = self
            .frame
            .clone()
            .wrap_err("scroll frame was not laid out before rendering")?;
        self.solution = Some(context.solution.clone());
        let content = frame.get_hitbox().await?.get_resolved(context.solution);
        self.content_size = content.size;
        let loaded_theme = (*theme.affect(render.clone()).await?).clone();
        let scrollbars = Scrollbars::new(hitbox, self.content_size, &loaded_theme);
        self.viewport = scrollbars.viewport();
        self.clamp_offset();

        let mut logical_scene = Vello_scene::new();
        {
            let mut scene = Scene::new(&mut logical_scene);
            let previous_logical = {
                let mut frame = frame.lock().await?;
                std::mem::replace(&mut frame.logical, false)
            };
            let render_result = frame
                .render(render, theme.clone(), &mut scene, text_context, context)
                .await;
            frame.lock().await?.logical = previous_logical;
            render_result?;
        }

        let transform = Affine::translate((
            hitbox.origin.x - self.offset.x,
            hitbox.origin.y - self.offset.y,
        ));
        scene.append_clipped(&logical_scene, self.viewport, transform);
        scrollbars.paint(scene, self.offset, &loaded_theme);

        Ok(None)
    }

    async fn on_mouse_click(&mut self, pointer: &Pointer_event) -> Result<Vizual_msg> {
        let frame = self
            .frame
            .as_ref()
            .wrap_err("scroll frame was not laid out before handling a pointer event")?;
        let solution = self
            .solution
            .as_ref()
            .wrap_err("scroll was not rendered before handling a pointer event")?;
        let pointer = Pointer_event {
            position: Point::new(
                pointer.position.x - self.viewport.origin.x + self.offset.x,
                pointer.position.y - self.viewport.origin.y + self.offset.y,
            ),
            button: pointer.button,
        };

        frame::forward_pointer(frame, &pointer, solution).await
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
            true => Vizual_msg::new(Vizual_command::Render),
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
            true => Vizual_msg::new(Vizual_command::Render),
            false => Vizual_msg::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Modifiers, Wheel_event};

    #[derive(Clone)]
    struct Empty;

    #[async_trait]
    impl Widget_trait for Empty {
        async fn layout(
            &mut self,
            _render: crate::Render,
            _theme: Store<Theme>,
            _focus: &mut Focus_provider,
            _hitbox: &mut Hitbox,
            _parent: Hitbox,
            _problem: Component_context,
            _text_context: &mut Text_context,
            _slots: &mut Slots,
        ) -> Result<Children> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn offset_is_clamped_to_content_edge() {
        let mut scroll = Scroll_content::new(Empty);
        scroll.content_size = Size::new(300.0, 200.0);
        scroll.viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
        scroll.offset = Point::new(500.0, -20.0);

        scroll.clamp_offset();

        assert_eq!(scroll.offset, Point::new(200.0, 0.0));
    }

    #[tokio::test]
    async fn arrow_scrolling_stops_at_content_edge() -> Result<()> {
        let mut scroll = Scroll_content::new(Empty);
        scroll.content_size = Size::new(100.0, 70.0);
        scroll.viewport = Rect::new(0.0, 0.0, 40.0, 30.0);
        let right = Key_event {
            code: Key_code::Arrow_right,
            modifiers: Modifiers::default(),
            text: None,
            repeat: false,
        };

        for _ in 0..10 {
            let _ = scroll.on_key_press(&right).await?;
        }

        assert_eq!(scroll.offset, Point::new(60.0, 0.0));
        Ok(())
    }

    #[tokio::test]
    async fn wheel_scrolls_vertically_and_shift_wheel_scrolls_horizontally() -> Result<()> {
        let mut scroll = Scroll_content::new(Empty);
        scroll.content_size = Size::new(200.0, 200.0);
        scroll.viewport = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut wheel = Wheel_event {
            position: Point::new(50.0, 50.0),
            delta: Wheel_delta::Lines(Point::new(0.0, -1.0)),
            modifiers: Modifiers::default(),
        };

        let message = scroll.on_other_event(&Event::Wheel(wheel)).await?;
        assert!(message.has_command());
        assert_eq!(scroll.offset, Point::new(0.0, SCROLL_STEP));

        wheel.modifiers.shift = true;
        let message = scroll.on_other_event(&Event::Wheel(wheel)).await?;
        assert!(message.has_command());
        assert_eq!(scroll.offset, Point::new(SCROLL_STEP, SCROLL_STEP));

        Ok(())
    }
}
