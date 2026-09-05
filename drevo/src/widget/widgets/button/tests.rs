use super::*;
use crate::theme::dark_theme;

#[test]
fn highlighted_button_uses_the_nested_highlight_token() {
    let theme = dark_theme();
    let style = resolve_block_style(&theme, true);

    assert_eq!(style.background, theme.specific.button.highlight);
}
