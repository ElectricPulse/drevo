use super::Color;

#[test]
fn rgba_preserves_all_four_channels() {
    let rgba = Color::Rgba(12, 34, 56, 78).to_peniko().to_rgba8();

    assert_eq!((rgba.r, rgba.g, rgba.b, rgba.a), (12, 34, 56, 78));
}

#[test]
fn lighten_and_darken_saturate_color_channels() {
    assert_eq!(Color::Rgb(250, 10, 0).lighten(10), Color::Rgb(255, 20, 10));
    assert_eq!(
        Color::Rgba(5, 20, 255, 78).darken(10),
        Color::Rgba(0, 10, 245, 78)
    );
}
