use super::*;

#[test]
fn scrollbar_style_calculates_dimensions() {
    let theme = crate::theme::dark_theme();
    let style = ScrollbarStyle::new(&theme);
    assert_eq!(style.gutter, theme.units.em * 0.75);
    assert_eq!(style.rail, theme.units.em * 0.25);
    assert_eq!(style.thumb, theme.units.em * 0.5);
}
