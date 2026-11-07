//! JPEG encoding.
//!
//! Uses `jpeg-encoder` rather than the `image` crate for two reasons that
//! matter for file size:
//!
//! * progressive encoding (sharp's default `progressive: true`)
//! * optimised Huffman tables
//!
//! Deviation from the Electron build: mozjpeg's trellis quantisation is not
//! available, so files land roughly 5-15% larger at the same quality setting.

use super::clamp_quality;
use crate::error::{AppError, AppResult};
use crate::model::ConvertOptions;
use image::RgbaImage;
use jpeg_encoder::{ColorType, Encoder, SamplingFactor};

/// jpeg-encoder works in 16-bit dimensions.
const MAX_DIMENSION: u32 = u16::MAX as u32;

fn check_dimensions(width: u32, height: u32) -> AppResult<()> {
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(AppError::new(format!(
            "JPEG cannot store images larger than {MAX_DIMENSION}px on a side ({width}x{height} received)."
        )));
    }
    Ok(())
}

pub fn encode(image: &RgbaImage, options: &ConvertOptions) -> AppResult<Vec<u8>> {
    let width = image.width();
    let height = image.height();
    check_dimensions(width, height)?;

    let quality = clamp_quality(options.quality);
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut encoder = Encoder::new(&mut buffer, quality);
        encoder.set_progressive(true);
        encoder.set_optimized_huffman_tables(true);
        // 4:2:0 chroma subsampling, matching sharp's default for photos.
        encoder.set_sampling_factor(SamplingFactor::F_2_2);

        encoder
            .encode(image.as_raw(), width as u16, height as u16, ColorType::Rgba)
            .map_err(|error| AppError::new(format!("JPEG encoding failed: {error}")))?;
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
                ((x * 7) % 256) as u8,
                ((y * 11) % 256) as u8,
                (((x + y) * 3) % 256) as u8,
                255,
            ]);
        }
        image
    }

    fn options(quality: u8) -> ConvertOptions {
        ConvertOptions {
            format: ImageFormat::Jpg,
            quality,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn encodes_a_valid_progressive_jpeg() {
        let bytes = encode(&sample(32, 24), &options(80)).expect("jpeg should encode");

        // SOI marker.
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
        // EOI marker.
        assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xD9]);
        // Progressive JPEGs carry a DAC or SOF2 marker; SOF2 specifically.
        assert!(
            bytes.windows(2).any(|pair| pair == [0xFF, 0xC2]),
            "expected a SOF2 marker for a progressive JPEG"
        );
    }

    #[test]
    fn lower_quality_produces_smaller_files() {
        let image = sample(128, 128);
        let high = encode(&image, &options(95)).expect("high quality");
        let low = encode(&image, &options(20)).expect("low quality");
        assert!(
            low.len() < high.len(),
            "expected q20 ({}) to beat q95 ({})",
            low.len(),
            high.len()
        );
    }

    #[test]
    fn output_is_decodable_by_the_image_crate() {
        let bytes = encode(&sample(40, 30), &options(75)).expect("jpeg should encode");
        let decoded = image::load_from_memory(&bytes).expect("jpeg should decode");
        assert_eq!((decoded.width(), decoded.height()), (40, 30));
    }

    #[test]
    fn oversized_images_are_rejected_with_a_clear_message() {
        assert!(check_dimensions(4000, 3000).is_ok());

        let error = check_dimensions(70_000, 100).expect_err("must reject");
        assert!(error.to_string().contains("65535"));
    }
}
