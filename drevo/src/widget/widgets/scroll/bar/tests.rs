use super::*;
use crate::{
    event::{Event, PointerButton, PointerEvent},
    geometry::Point,
    state::Store,
    widget::{MouseEvent, WidgetTrait},
};
use color_eyre::eyre::Result;

#[test]
fn scrollbar_style_calculates_dimensions() {
    let theme = crate::theme::dark_theme();
    let style = ScrollbarStyle::new(&theme);
    assert_eq!(style.gutter, theme.units.em * 0.75);
    assert_eq!(style.rail, theme.units.em * 0.25);
    assert_eq!(style.thumb, theme.units.em * 0.5);
}

#[test]
fn unmeasured_scrollbar_thumb_uses_a_finite_ratio() {
    let ratio = viewport_ratio(0.0, 0.0);

    assert_eq!(ratio, 1.0);
    assert!(ratio.is_finite());
}

#[tokio::test]
async fn thumb_drag_updates_the_scroll_offset() -> Result<()> {
    let offset = Store::new(Point::new(0.0, 0.0));
    let mut thumb = Thumb {
        direction: Direction::Vertical,
        offset: 0.0,
        viewport_length: 100.0,
        content_length: 300.0,
        scrollable: true,
        offset_store: offset.clone(),
        drag: Store::new(None),
        metrics: Store::new(ThumbMetrics {
            travel: 80.0,
            maximum_offset: 200.0,
        }),
    };
    let press = Event::Pointer(PointerEvent {
        position: Point::new(0.0, 10.0),
        button: PointerButton::Primary,
    });
    let _ = thumb
        .on_mouse_event(MouseEvent {
            event: &press,
            relayout: crate::render_manager::RenderManager::new().layout,
            window: None,
        })
        .await?;
    let moved = Event::PointerMoved(Point::new(0.0, 50.0));
    let _ = thumb
        .on_mouse_event(MouseEvent {
            event: &moved,
            relayout: crate::render_manager::RenderManager::new().layout,
            window: None,
        })
        .await?;

    assert_eq!(*offset.read().await?, Point::new(0.0, 100.0));
    Ok(())
}
