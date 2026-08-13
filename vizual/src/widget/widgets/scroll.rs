use async_trait::async_trait;
use color_eyre::eyre::{ContextCompat, Result};
use vello::{Scene as Vello_scene, kurbo::Affine};
use vizual_macros::display;

use crate::{
    Vizual_command, Vizual_msg,
    component::{
        Child_reference, Children, Render_context, Shared_component, context::Component_context,
    },
    event::{Key_code, Key_event},
    geometry::{Point, Rect, Size},
    graphics::{scene::Scene, text::Text_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::State,
    theme::Theme,
    widget::{Child_render_region, Focus_provider, Widget, Widget_trait},
};

const SCROLL_STEP: f64 = 32.0;

#[derive(Clone)]
pub struct Scroll {
    child: Widget,
    component: Child_reference,
    offset: Point,
    content_size: Size,
    viewport: Rect,
}

impl Scroll {
    pub fn new(child: impl Widget_trait) -> Self {
        Self {
            child: Box::new(child),
            component: Child_reference::new(),
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
}

#[async_trait]
impl Widget_trait for Scroll {
    async fn layout(
        &mut self,
        _render: crate::Render,
        _theme: State<Theme>,
        focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        focus.set_active(true);

        let child = display!(self.child.clone());
        child.use_logical_root(&problem).await?;
        child.lock().await?.r#virtual = true;
        self.component = child.as_reference();

        // Layout deliberately remains in Vizual's ordinary component tree. Future state
        // management can then relayout only invalidated branches. Rendering is expected to keep
        // walking from the root each frame, so manually rendering this isolated subtree into a
        // temporary scene does not undermine that selective-layout design.
        Ok(vec![child])
    }

    async fn render(
        &mut self,
        theme: State<Theme>,
        focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        text_context: &mut Text_context,
        context: &Render_context<'_>,
    ) -> Result<Option<Hitbox>> {
        focus.set_active(true);

        let mut child = Shared_component::new(
            self.component
                .upgrade()
                .wrap_err("scroll child was not laid out before rendering")?,
        );
        let content = child.get_hitbox().await?.get_resolved(context.solution);
        self.content_size = content.size;
        self.viewport = hitbox;
        self.clamp_offset();

        let mut virtual_scene = Vello_scene::new();
        {
            let mut scene = Scene::new(&mut virtual_scene);
            child
                .render(theme, &mut scene, text_context, context)
                .await?;
        }

        let transform = Affine::translate((
            hitbox.origin.x - self.offset.x,
            hitbox.origin.y - self.offset.y,
        ));
        scene.append_clipped(&virtual_scene, hitbox, transform);

        Ok(None)
    }

    async fn child_render_region(&self) -> Option<Child_render_region> {
        Some(Child_render_region {
            translation: Point::new(
                self.viewport.origin.x - self.offset.x,
                self.viewport.origin.y - self.offset.y,
            ),
            clip: self.viewport,
        })
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        let previous = self.offset;
        match key.code {
            Key_code::Arrow_left => self.offset.x -= SCROLL_STEP,
            Key_code::Arrow_right => self.offset.x += SCROLL_STEP,
            Key_code::Arrow_up => self.offset.y -= SCROLL_STEP,
            Key_code::Arrow_down => self.offset.y += SCROLL_STEP,
            _ => return Vizual_msg::none(),
        }
        self.clamp_offset();

        match self.offset == previous {
            true => Vizual_msg::none(),
            false => Vizual_msg::new(Vizual_command::Render),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Modifiers;

    #[derive(Clone)]
    struct Empty;

    #[async_trait]
    impl Widget_trait for Empty {
        async fn layout(
            &mut self,
            _render: crate::Render,
            _theme: State<Theme>,
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
        let mut scroll = Scroll::new(Empty);
        scroll.content_size = Size::new(300.0, 200.0);
        scroll.viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
        scroll.offset = Point::new(500.0, -20.0);

        scroll.clamp_offset();

        assert_eq!(scroll.offset, Point::new(200.0, 0.0));
    }

    #[tokio::test]
    async fn arrow_scrolling_stops_at_content_edge() -> Result<()> {
        let mut scroll = Scroll::new(Empty);
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
}
