//! WebP encoding.
//!
//! `image`/`image-webp` can only *losslessly* encode WebP, and lossy WebP is
//! the app's default output format, so this goes through `zenwebp`.
//!
//! Licence note: `zenwebp` 0.1.x and 0.2.x ship under MIT OR Apache-2.0. From
//! 0.3.0 onwards the crate is AGPL-3.0-or-commercial, which is why the manifest
//! pins `zenwebp = "0.2"`. Do not bump it without re-checking the licence.
//!
//! API note: if `zenwebp` changes its builder between 0.2.x point releases,
//! this file is the only one that needs touching. The 0.2 surface used here is
//! `WebPEncoder::new(Write)` + `set_params(EncoderParams)` + `encode`.

use super::clamp_quality;
use crate::error::{AppError, AppResult};
use crate::model::ConvertOptions;
use image::RgbaImage;
use zenwebp::{ColorType, EncoderParams, WebPEncoder};

pub fn encode(image: &RgbaImage, options: &ConvertOptions) -> AppResult<Vec<u8>> {
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut encoder = WebPEncoder::new(&mut buffer);

        if options.lossless {
            encoder.set_params(EncoderParams::lossless());
        } else {
            // libwebp's quality scale is 0-100, same as the UI slider.
            encoder.set_params(EncoderParams::lossy(clamp_quality(options.quality)));
        }

        encoder
            .encode(
                image.as_raw(),
                image.width(),
                image.height(),
                ColorType::Rgba8,
            )
            .map_err(|error| AppError::new(format!("WebP encoding failed: {error}")))?;
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ImageFormat;
    use image::Rgba;

    fn sample(width: u32, height: u32) -> RgbaImage {
        let mut image = RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([
                ((x * 9) % 256) as u8,
                ((y * 5) % 256) as u8,
                (((x * y) % 251) as u8),
                255,
            ]);
        }
        image
    }

    fn options(lossless: bool, quality: u8) -> ConvertOptions {
        ConvertOptions {
            format: ImageFormat::Webp,
            lossless,
            quality,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn encodes_a_riff_webp_container() {
        let bytes = encode(&sample(32, 32), &options(false, 80)).expect("webp should encode");
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
    }

    #[test]
    fn lossy_output_decodes_back_to_the_same_size() {
        let bytes = encode(&sample(40, 24), &options(false, 70)).expect("webp should encode");
        let decoded = image::load_from_memory(&bytes).expect("webp should decode");
        assert_eq!((decoded.width(), decoded.height()), (40, 24));
    }

    #[test]
    fn lossless_round_trip_is_pixel_perfect() {
        let image = sample(16, 16);
        let bytes = encode(&image, &options(true, 90)).expect("webp should encode");
        let decoded = image::load_from_memory(&bytes)
            .expect("webp should decode")
            .to_rgba8();
        assert_eq!(decoded.dimensions(), image.dimensions());
        assert_eq!(decoded.as_raw(), image.as_raw());
    }
}
