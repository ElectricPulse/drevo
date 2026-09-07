use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, ensure};
use vello::peniko::{ImageAlphaType, ImageData, ImageFormat};

use crate::{
    component::Children,
    constraint,
    geometry::Direction,
    id,
    widget::{LayoutInput, RenderInput, WidgetTrait},
};

/// A static raster image with its source aspect ratio preserved.
#[derive(Clone)]
pub struct Image {
    data: ImageData,
}

impl Image {
    /// Decodes an image.
    ///
    /// Layout preserves the source aspect ratio. Its parent determines the actual size. PNG,
    /// JPEG, GIF, and WebP sources are supported; animated sources render their first frame.
    pub fn new(source: impl AsRef<[u8]>) -> Result<Self> {
        let image = image::load_from_memory(source.as_ref()).wrap_err("failed to decode image")?;
        let image = image.into_rgba8();
        let source_width = image.width();
        let source_height = image.height();
        ensure!(
            source_width > 0 && source_height > 0,
            "image dimensions must be nonzero"
        );

        Ok(Self {
            data: ImageData {
                data: image.into_raw().into(),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::Alpha,
                width: source_width,
                height: source_height,
            },
        })
    }
}

#[async_trait]
impl WidgetTrait for Image {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox, formula, ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        formula.constrain(
            id!(),
            constraint!(
                hitbox.get_dimension(Direction::Horizontal) * f64::from(self.data.height)
                    == hitbox.get_dimension(Direction::Vertical) * f64::from(self.data.width)
            ),
        )?;

        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        RenderInput { hitbox, scene, .. }: RenderInput<'_, '_>,
    ) -> Result<()> {
        scene.draw_image(&self.data, hitbox);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
