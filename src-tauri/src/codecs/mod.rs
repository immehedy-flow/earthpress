//! Encoders, one module per output format.

pub mod avif;
pub mod jpeg;
pub mod png;
pub mod quantize;
pub mod webp;

use crate::error::{AppError, AppResult};
use crate::model::{ConvertOptions, ImageFormat};
use image::RgbaImage;

/// The AVIF encoder speed (rav1e effort). `10` is fastest, `0` is slowest and
/// smallest. 6 keeps AVIF batches usable on a laptop.
pub const AVIF_SPEED: u8 = 6;

/// sharp clamps quality to 1..=100 as well.
pub fn clamp_quality(quality: u8) -> u8 {
    quality.clamp(1, 100)
}

/// Encodes `image` in the format requested by `options`.
pub fn encode(image: &RgbaImage, options: &ConvertOptions) -> AppResult<Vec<u8>> {
    if image.width() == 0 || image.height() == 0 {
        return Err(AppError::new("Refusing to encode an empty image."));
    }

    match options.format {
        ImageFormat::Jpg => jpeg::encode(image, options),
        ImageFormat::Png => png::encode(image, options),
        ImageFormat::Webp => webp::encode(image, options),
        ImageFormat::Avif => avif::encode(image, options),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_is_clamped_to_the_valid_range() {
        assert_eq!(clamp_quality(0), 1);
        assert_eq!(clamp_quality(1), 1);
        assert_eq!(clamp_quality(82), 82);
        assert_eq!(clamp_quality(100), 100);
        assert_eq!(clamp_quality(255), 100);
    }

    #[test]
    fn empty_images_are_rejected() {
        let image = RgbaImage::new(0, 0);
        let error = encode(&image, &ConvertOptions::default()).expect_err("must fail");
        assert!(error.to_string().contains("empty"));
    }
}
