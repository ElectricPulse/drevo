use super::*;

#[test]
fn system_theme_tracks_system_changes() {
    let theme = system_theme(SystemTheme::Dark).set_system(SystemTheme::Light);

    assert_eq!(theme.mode(), SystemTheme::Light);
    assert_eq!(theme.system(), SystemTheme::Light);
    assert!(theme.follows_system());
}

#[test]
fn explicit_theme_keeps_its_mode_when_the_system_changes() {
    let theme = system_theme(SystemTheme::Dark)
        .select(SystemTheme::Dark)
        .set_system(SystemTheme::Light);

    assert_eq!(theme.mode(), SystemTheme::Dark);
    assert_eq!(theme.system(), SystemTheme::Light);
    assert!(!theme.follows_system());
}

#[test]
fn fluent_neutral_surfaces_progress_inward() {
    let dark = dark_theme();
    assert_eq!(
        dark.specific.body.block.background,
        dark.semantic.background.lighten(DARK_STEP)
    );
    assert_eq!(
        dark.specific.paper.block.background,
        dark.specific.body.block.background.lighten(DARK_STEP)
    );
    assert_eq!(
        dark.specific.button.block.background,
        dark.specific.paper.block.background.lighten(10)
    );
    assert_eq!(
        dark.specific.button.highlight,
        dark.specific.button.block.background.darken(15)
    );

    let light = light_theme();
    assert_eq!(
        light.specific.body.block.background,
        light.semantic.background.darken(LIGHT_STEP)
    );
    assert_eq!(
        light.specific.paper.block.background,
        light.specific.body.block.background.lighten(LIGHT_STEP)
    );
    assert_eq!(
        light.specific.button.block.background,
        light.specific.paper.block.background.lighten(10)
    );
    assert_eq!(
        light.specific.button.highlight,
        light.specific.button.block.background.darken(15)
    );
}

#[test]
fn header_uses_the_base_background_without_a_border() {
    for theme in [dark_theme(), light_theme()] {
        assert_eq!(theme.specific.header.background, theme.semantic.background);
        assert_eq!(theme.specific.header.border.thickness, 0.0);
        assert_eq!(theme.specific.header.focused_border.thickness, 0.0);
    }
}

#[test]
fn focus_accent_is_separate_from_selected_button_backgrounds() {
    for theme in [dark_theme(), light_theme()] {
        assert_eq!(
            theme.specific.button.block.focused_border.color,
            theme.semantic.focus
        );
        assert_ne!(theme.specific.button.highlight, theme.semantic.focus);
    }
}
