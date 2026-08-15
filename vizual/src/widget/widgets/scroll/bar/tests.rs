use super::*;

#[test]
fn one_scrollbar_can_make_the_other_axis_overflow() {
    let scrollbars = Scrollbars::new(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        Size::new(100.0, 101.0),
        &crate::theme::dark_theme(),
    );

    assert!(scrollbars.horizontal.is_some());
    assert!(scrollbars.vertical.is_some());
    assert_eq!(scrollbars.viewport.size, Size::new(88.0, 88.0));
}
