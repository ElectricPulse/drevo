use async_trait::async_trait;
use color_eyre::Result;
use lucide_icons::Icon as Lucide_icon;
use parley::Layout;

use super::{
    super::{Layout_input, Render_input, Widget_trait},
    text::Text_style,
};
use crate::{
    component::Children,
    geometry::{Direction, Point, Size},
    graphics::text::{Styled_text, Text_brush, icon_ink_bounds},
    state::State,
    style::Style,
};

#[derive(Clone)]
pub struct Icon {
    icon: State<Lucide_icon>,
    pub style: Style<Text_style>,
    cached_layout: Option<(Point, Layout<Text_brush>)>,
}

impl Icon {
    pub fn new(icon: impl Into<State<Lucide_icon>>) -> Self {
        Self {
            icon: icon.into(),
            style: Style::default(),
            cached_layout: None,
        }
    }
}

#[async_trait]
impl Widget_trait for Icon {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            hitbox,
            problem,
            text_context,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let icon = *self.icon.affect(render.clone()).await?;
        let theme = theme.affect(render).await?;
        let style = self.style.get(&theme);
        let layout = text_context.build_layout(&Styled_text::icon(icon, style));
        let bounds = icon_ink_bounds(&layout, icon, style.size);
        let size = bounds.map_or_else(
            || Size::new(f64::from(layout.full_width()), f64::from(layout.height())),
            |bounds| Size::new(bounds.width(), bounds.height()),
        );
        let offset = bounds.map_or(Point::default(), |bounds| {
            Point::new(-bounds.x0, -bounds.y0)
        });
        self.cached_layout = Some((offset, layout));

        hitbox
            .set_static_dimension(&problem, Direction::Horizontal, size.width)
            .await?;
        hitbox
            .set_static_dimension(&problem, Direction::Vertical, size.height)
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        Render_input {
            render,
            theme,
            hitbox,
            scene,
            text_context,
            ..
        }: Render_input<'_, '_>,
    ) -> Result<()> {
        if let Some((offset, layout)) = &self.cached_layout {
            let origin = Point::new(hitbox.origin.x + offset.x, hitbox.origin.y + offset.y);
            scene.paint_layout(layout, origin, false);
        } else {
            let icon = *self.icon.affect(render.clone()).await?;
            let theme = theme.affect(render).await?;
            let _ = text_context.draw_icon(scene, icon, hitbox.origin, self.style.get(&theme));
        }
        Ok(())
    }
}
