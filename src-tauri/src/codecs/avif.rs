//! AVIF encoding.
//!
//! AVIF is built on AV1, and the only pure-Rust encoder that produces AV1 is
//! rav1e, reached here through `ravif`. The manifest disables ravif's `asm`
//! feature so no NASM install is needed; the trade-off is a slower encode.
//!
//! Licence note: `ravif` is BSD-3-Clause and `rav1e` is BSD-2-Clause, both
//! permissive. `zenavif` is not used anywhere in this crate precisely because
//! it is AGPL-3.0-or-commercial.
//!
//! Deviation from the Electron build: libaom has a true lossless AV1 mode and
//! rav1e does not expose one through ravif, so `lossless` falls back to quality
//! 100. The result is visually lossless, not bit-exact.

use super::{clamp_quality, AVIF_SPEED};
use crate::error::{AppError, AppResult};
use crate::model::ConvertOptions;
use image::RgbaImage;
use ravif::{Encoder, Img, RGBA8};

pub fn encode(image: &RgbaImage, options: &ConvertOptions) -> AppResult<Vec<u8>> {
    let width = image.width() as usize;
    let height = image.height() as usize;

    // ravif takes its own pixel type, so build it once up front.
    let pixels: Vec<RGBA8> = image
        .pixels()
        .map(|pixel| RGBA8::new(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect();

    let quality = if options.lossless {
        if options.quality < 100 {
            crate::logging::warn(
                "AVIF has no bit-exact lossless mode in rav1e; encoding at quality 100",
                Some(serde_json::json!({ "requestedQuality": options.quality })),
            );
        }
        100.0
    } else {
        f32::from(clamp_quality(options.quality))
    };

    let encoded = Encoder::new()
        .with_quality(quality)
        .with_speed(AVIF_SPEED)
        .encode_rgba(Img::new(&pixels, width, height))
        .map_err(|error| AppError::new(format!("AVIF encoding failed: {error}")))?;

    Ok(encoded.avif_file)
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
                ((x * 13) % 256) as u8,
                ((y * 17) % 256) as u8,
                (((x + y) * 7) % 256) as u8,
                255,
            ]);
        }
        image
    }

    fn options(quality: u8) -> ConvertOptions {
        ConvertOptions {
            format: ImageFormat::Avif,
            quality,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn encodes_an_avif_container() {
        let bytes = encode(&sample(24, 24), &options(60)).expect("avif should encode");

        // ISO-BMFF: a `ftyp` box with an `avif` brand.
        assert_eq!(&bytes[4..8], b"ftyp");
        assert!(
            bytes.windows(4).any(|window| window == b"avif"),
            "expected the avif brand"
        );
    }

    #[test]
    fn output_round_trips_through_the_avif_decoder() {
        let image = sample(16, 16);
        let bytes = encode(&image, &options(70)).expect("avif should encode");
        let decoded = crate::decode::decode(&bytes).expect("avif should decode");
        assert_eq!(decoded.dimensions(), image.dimensions());
    }
}
