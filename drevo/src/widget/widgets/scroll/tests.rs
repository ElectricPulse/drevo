use super::*;
use crate::{
    config::SCROLL_SENSITIVITY,
    event::{KeyEvent, Modifiers, WheelEvent},
};

#[derive(Clone)]
struct Empty;

#[async_trait]
impl WidgetTrait for Empty {}

#[test]
fn scrollbars_are_hidden_when_content_fits() {
    let scrollbars =
        ScrollbarVisibility::for_content(Size::new(100.0, 80.0), Rect::new(0.0, 0.0, 100.0, 80.0));

    assert_eq!(scrollbars, ScrollbarVisibility::default());
}

#[test]
fn scrollbars_appear_only_on_overflowing_axes() {
    let scrollbars =
        ScrollbarVisibility::for_content(Size::new(120.0, 80.0), Rect::new(0.0, 0.0, 100.0, 80.0));

    assert!(scrollbars.horizontal);
    assert!(!scrollbars.vertical);
}

#[tokio::test]
async fn offset_is_clamped_to_content_edge() -> Result<()> {
    let mut scroll = Scroll::new(Empty);
    scroll.content_size = Size::new(300.0, 200.0);
    scroll.viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
    scroll.offset.set(Point::new(500.0, -20.0)).await?;

    let _ = scroll.clamp_offset().await?;

    assert_eq!(*scroll.offset.read().await?, Point::new(200.0, 0.0));
    Ok(())
}

#[tokio::test]
async fn arrow_scrolling_stops_at_content_edge() -> Result<()> {
    let manager = crate::render_manager::RenderManager::new();
    let mut scroll = Scroll::new(Empty);
    scroll.content_size = Size::new(100.0, 70.0);
    scroll.viewport = Rect::new(0.0, 0.0, 40.0, 30.0);
    let right = KeyEvent {
        code: KeyCode::ArrowRight,
        modifiers: Modifiers::default(),
        text: None,
        repeat: false,
    };

    for _ in 0..10 {
        let _ = scroll
            .on_key_press(crate::widget::KeyPress {
                key: &right,
                relayout: manager.layout.clone(),
                window: None,
            })
            .await?;
    }

    assert_eq!(*scroll.offset.read().await?, Point::new(60.0, 0.0));
    Ok(())
}

#[tokio::test]
async fn wheel_scrolls_vertically_and_shift_wheel_scrolls_horizontally() -> Result<()> {
    let mut manager = crate::render_manager::RenderManager::new();
    let mut scroll = Scroll::new(Empty);
    scroll.content_size = Size::new(400.0, 400.0);
    scroll.viewport = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut wheel = WheelEvent {
        position: Point::new(50.0, 50.0),
        delta: Point::new(0.0, -SCROLL_SENSITIVITY),
        modifiers: Modifiers::default(),
    };

    let _ = scroll.offset.affect(manager.layout.clone()).await?;
    let event = Event::Wheel(wheel);
    let message = scroll
        .on_mouse_event(crate::widget::MouseEvent {
            event: &event,
            relayout: manager.layout.clone(),
            window: None,
        })
        .await?;
    assert!(!message.has_command());
    assert_eq!(
        *scroll.offset.read().await?,
        Point::new(0.0, SCROLL_SENSITIVITY)
    );
    assert_eq!(
        manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Layout)
    );

    wheel.modifiers.shift = true;
    let event = Event::Wheel(wheel);
    let message = scroll
        .on_mouse_event(crate::widget::MouseEvent {
            event: &event,
            relayout: manager.layout.clone(),
            window: None,
        })
        .await?;
    assert!(!message.has_command());
    assert_eq!(
        *scroll.offset.read().await?,
        Point::new(SCROLL_SENSITIVITY, SCROLL_SENSITIVITY)
    );
    assert_eq!(
        manager.receiver.0.recv().await,
        Some(crate::RenderRequest::Layout)
    );

    Ok(())
}
