use super::*;

#[test]
fn source_dimensions_are_preserved() -> Result<()> {
    let image = Image::new(include_bytes!("../../../../assets/logo.png"))?;

    assert!(image.data.width > 0);
    assert!(image.data.height > 0);
    Ok(())
}

#[test]
fn invalid_image_data_is_rejected() {
    assert!(Image::new(b"not an image").is_err());
}
