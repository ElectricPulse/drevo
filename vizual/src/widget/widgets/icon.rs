use async_trait::async_trait;
use color_eyre::Result;
use lucide_icons::Icon as LucideIcon;
use parley::Layout;

use super::{
    super::{LayoutInput, RenderInput, WidgetTrait},
    text::TextStyle,
};
use crate::{
    component::Children,
    geometry::{Direction, Point, Size},
    graphics::text::{StyledText, TextBrush, icon_ink_bounds},
    state::State,
    style::Style,
};

#[derive(Clone)]
pub struct Icon {
    icon: State<LucideIcon>,
    pub style: Style<TextStyle>,
    cached_layout: Option<(Point, Layout<TextBrush>)>,
}

impl Icon {
    pub fn new(icon: impl Into<State<LucideIcon>>) -> Self {
        Self {
            icon: icon.into(),
            style: Style::default(),
            cached_layout: None,
        }
    }
}

#[async_trait]
impl WidgetTrait for Icon {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            hitbox,
            formula: problem,
            text_context,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let icon = *self.icon.affect(relayout.clone()).await?;
        let theme = theme.affect(relayout).await?;
        let style = self.style.get(&theme);
        let layout = text_context
            .build_layout(&StyledText::icon(icon, style))
            .await?;
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
            .set_static_dimension(problem, Direction::Horizontal, size.width)
            .await?;
        hitbox
            .set_static_dimension(problem, Direction::Vertical, size.height)
            .await?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        RenderInput {
            rerender,
            theme,
            hitbox,
            scene,
            text_context,
            ..
        }: RenderInput<'_, '_>,
    ) -> Result<()> {
        if let Some((offset, layout)) = &self.cached_layout {
            let origin = Point::new(hitbox.origin.x + offset.x, hitbox.origin.y + offset.y);
            scene.paint_layout(layout, origin, false);
        } else {
            let icon = *self.icon.affect(rerender.clone()).await?;
            let theme = theme.affect(rerender).await?;
            let _ = text_context
                .draw_icon(scene, icon, hitbox.origin, self.style.get(&theme))
                .await?;
        }
        Ok(())
    }
}
