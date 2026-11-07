//! PNG encoding, including optional palette (indexed) output.
//!
//! The `png` crate only exposes three compression levels, so the UI's 0-9
//! slider is mapped onto them:
//!
//! | slider | png::Compression |
//! |--------|------------------|
//! | 0-3    | Fast             |
//! | 4-7    | Default          |
//! | 8-9    | Best             |

use super::quantize::quantize;
use crate::error::{AppError, AppResult};
use crate::model::ConvertOptions;
use image::RgbaImage;
use png::{AdaptiveFilterType, BitDepth, ColorType, Compression, Encoder};

/// Above this many colours a palette would visibly band, so we refuse.
const MAX_PALETTE_COLORS: usize = 256;

fn compression_for(level: u8) -> Compression {
    match level {
        0..=3 => Compression::Fast,
        4..=7 => Compression::Default,
        _ => Compression::Best,
    }
}

pub fn encode(image: &RgbaImage, options: &ConvertOptions) -> AppResult<Vec<u8>> {
    let width = image.width();
    let height = image.height();
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut encoder = Encoder::new(&mut buffer, width, height);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(compression_for(options.png_compression));
        // Pick a filter per row rather than a fixed one: it wins on mixed
        // content and is what libpng/sharp do by default.
        encoder.set_adaptive_filter(AdaptiveFilterType::Adaptive);

        if options.png_palette {
            let quantized = quantize(image, MAX_PALETTE_COLORS);

            encoder.set_color(ColorType::Indexed);
            encoder.set_palette(quantized.entries);
            if !quantized.transparency.is_empty() {
                encoder.set_trns(quantized.transparency);
            }

            let mut writer = encoder
                .write_header()
                .map_err(|error| AppError::new(format!("PNG header failed: {error}")))?;
            writer
                .write_image_data(&quantized.indices)
                .map_err(|error| AppError::new(format!("PNG encoding failed: {error}")))?;
        } else {
            encoder.set_color(ColorType::Rgba);
            let mut writer = encoder
                .write_header()
                .map_err(|error| AppError::new(format!("PNG header failed: {error}")))?;
            writer
                .write_image_data(image.as_raw())
                .map_err(|error| AppError::new(format!("PNG encoding failed: {error}")))?;
        }
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ImageFormat;
    use image::Rgba;

    fn gradient(width: u32, height: u32) -> RgbaImage {
        let mut image = RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([
                ((x * 255) / width.max(1)) as u8,
                ((y * 255) / height.max(1)) as u8,
                128,
                255,
            ]);
        }
        image
    }

    fn options(palette: bool) -> ConvertOptions {
        ConvertOptions {
            format: ImageFormat::Png,
            png_palette: palette,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn encodes_truecolour_png_with_alpha() {
        let bytes = encode(&gradient(16, 16), &options(false)).expect("png should encode");
        assert_eq!(
            &bytes[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );

        let decoded = image::load_from_memory(&bytes).expect("png should decode");
        assert_eq!((decoded.width(), decoded.height()), (16, 16));
        assert!(decoded.color().has_alpha());
    }

    #[test]
    fn encodes_indexed_png() {
        let bytes = encode(&gradient(16, 16), &options(true)).expect("png should encode");
        let decoded = image::load_from_memory(&bytes).expect("png should decode");
        assert_eq!((decoded.width(), decoded.height()), (16, 16));
    }

    #[test]
    fn palette_mode_is_smaller_for_flat_art() {
        let mut image = RgbaImage::new(64, 64);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = if (x / 8 + y / 8) % 2 == 0 {
                Rgba([20, 40, 60, 255])
            } else {
                Rgba([240, 230, 220, 255])
            };
        }

        let truecolour = encode(&image, &options(false)).expect("truecolour");
        let indexed = encode(&image, &options(true)).expect("indexed");
        assert!(
            indexed.len() < truecolour.len(),
            "indexed ({}) should beat truecolour ({})",
            indexed.len(),
            truecolour.len()
        );
    }

    #[test]
    fn compression_slider_maps_onto_the_three_available_levels() {
        assert!(matches!(compression_for(0), Compression::Fast));
        assert!(matches!(compression_for(3), Compression::Fast));
        assert!(matches!(compression_for(5), Compression::Default));
        assert!(matches!(compression_for(8), Compression::Best));
        assert!(matches!(compression_for(9), Compression::Best));
    }
}
