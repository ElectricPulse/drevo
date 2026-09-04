use super::*;
use crate::event::{KeyEvent, Modifiers, WheelEvent};

#[derive(Clone)]
struct Empty;

#[async_trait]
impl WidgetTrait for Empty {}

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
                relayout: manager.rerender.clone(),
                window: None,
            })
            .await?;
    }

    assert_eq!(scroll.offset, Point::new(60.0, 0.0));
    Ok(())
}

#[tokio::test]
async fn wheel_scrolls_vertically_and_shift_wheel_scrolls_horizontally() -> Result<()> {
    let manager = crate::render_manager::RenderManager::new();
    let mut scroll = Scroll::new(Empty);
    scroll.content_size = Size::new(400.0, 400.0);
    scroll.viewport = Rect::new(0.0, 0.0, 100.0, 100.0);
    let mut wheel = WheelEvent {
        position: Point::new(50.0, 50.0),
        delta: Point::new(0.0, -SCROLL_STEP),
        modifiers: Modifiers::default(),
    };

    let event = Event::Wheel(wheel);
    let message = scroll
        .on_other_event(crate::widget::OtherEvent {
            event: &event,
            relayout: manager.rerender.clone(),
            window: None,
        })
        .await?;
    assert!(!message.has_command());
    assert_eq!(scroll.offset, Point::new(0.0, SCROLL_STEP));

    wheel.modifiers.shift = true;
    let event = Event::Wheel(wheel);
    let message = scroll
        .on_other_event(crate::widget::OtherEvent {
            event: &event,
            relayout: manager.rerender.clone(),
            window: None,
        })
        .await?;
    assert!(!message.has_command());
    assert_eq!(scroll.offset, Point::new(SCROLL_STEP, SCROLL_STEP));

    Ok(())
}
