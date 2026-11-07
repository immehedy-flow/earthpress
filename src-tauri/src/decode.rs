//! Decoding is split out from the encoders because input and output formats
//! have different support matrices.
//!
//! * JPEG / PNG / WebP / BMP / TIFF / GIF go through the pure-Rust `image`
//!   crate.
//! * AVIF goes through `avif-decode`, which uses rav1d. It is attempted first
//!   for AVIF-branded containers and as a fallback when `image` cannot sniff
//!   the bytes.

use crate::error::{AppError, AppResult};
use image::{ImageReader, Rgba, RgbaImage};
use std::io::Cursor;
use std::path::Path;

/// Refuse absurd inputs before they can exhaust memory. A 100 MP image is
/// already 400 MB as RGBA, and four workers may run at once.
pub const MAX_PIXELS: u64 = 100_000_000;
/// Refuse files larger than this before reading them into memory.
pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

const HEIF_BRANDS: [&[u8; 4]; 7] = [
    b"avif", b"avis", b"mif1", b"msf1", b"heic", b"heix", b"heim",
];

/// True when the container looks like AVIF/HEIF rather than a plain image.
pub fn is_avif(bytes: &[u8]) -> bool {
    bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && HEIF_BRANDS.iter().any(|brand| bytes[8..12] == brand[..])
}

/// Reads a source file with the size and pixel guards applied first.
///
/// Every caller (inspect, preview, estimate, convert) goes through here so a
/// crafted "decompression bomb" cannot allocate gigabytes before it is
/// rejected.
pub fn read_source(path: &Path) -> AppResult<Vec<u8>> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(AppError::new("Not a file."));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(AppError::new(format!(
            "File is larger than {} MB.",
            MAX_FILE_BYTES / (1024 * 1024)
        )));
    }

    let bytes = std::fs::read(path)?;
    ensure_within_limits(&bytes)?;
    Ok(bytes)
}

/// Rejects images whose header advertises more pixels than we will decode.
pub fn ensure_within_limits(bytes: &[u8]) -> AppResult<()> {
    let Some((width, height)) = header_dimensions(bytes) else {
        // Unknown headers fall through to the decoder's own error handling.
        return Ok(());
    };

    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(AppError::new(format!(
            "Image is too large ({width} × {height}); the limit is {} megapixels.",
            MAX_PIXELS / 1_000_000
        )));
    }

    Ok(())
}

/// Reads dimensions from the container header without decoding pixels.
fn header_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if is_avif(bytes) {
        return avif_dimensions(bytes);
    }

    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

/// Pulls width/height out of an AVIF `ispe` box. `image` cannot parse AVIF
/// headers, and rav1d allocates from the same values, so this check has to
/// happen before handing the file over.
fn avif_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let window = &bytes[..bytes.len().min(1024 * 1024)];

    for index in 0..window.len().saturating_sub(15) {
        if window.get(index..index + 4) != Some(b"ispe") {
            continue;
        }

        let width = u32::from_be_bytes(window.get(index + 8..index + 12)?.try_into().ok()?);
        let height = u32::from_be_bytes(window.get(index + 12..index + 16)?.try_into().ok()?);

        // Real `ispe` boxes are a FullBox: 4 bytes version/flags, then two
        // 32-bit dimensions. Ignore accidental matches with absurd values.
        if width > 0 && height > 0 && width <= 1_000_000 && height <= 1_000_000 {
            return Some((width, height));
        }
    }

    None
}

/// Decodes any supported image into RGBA8.
pub fn decode(bytes: &[u8]) -> AppResult<RgbaImage> {
    if is_avif(bytes) {
        return decode_avif(bytes);
    }

    match image::load_from_memory(bytes) {
        Ok(image) => Ok(image.to_rgba8()),
        Err(primary) => {
            // An AVIF container with a brand we did not recognise still gets a
            // chance before we surface the image crate's error.
            decode_avif(bytes).map_err(|_| AppError::new(primary.to_string()))
        }
    }
}

/// Copies an AV1-decoded buffer into an `RgbaImage`, converting whatever
/// channel layout the decoder chose.
macro_rules! into_rgba {
    ($buffer:expr, |$pixel:ident| $convert:expr) => {{
        let buffer = $buffer;
        let width = buffer.width() as u32;
        let height = buffer.height() as u32;
        let mut output = RgbaImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let $pixel = buffer[(x, y)];
                output.put_pixel(x, y, $convert);
            }
        }

        output
    }};
}

/// AVIF decoding via rav1d.
///
/// `avif_decode` hands back its own `Image` enum rather than an `image` crate
/// type, so each pixel layout is mapped explicitly. 16-bit sources are reduced
/// to 8-bit for the same reason the rest of the pipeline works in 8-bit: the UI
/// only ever reports 8-bit sizes and every encoder here is 8-bit.
fn decode_avif(bytes: &[u8]) -> AppResult<RgbaImage> {
    let decoded = avif_decode::Decoder::from_avif(bytes)
        .map_err(|error| AppError::new(format!("Could not decode AVIF: {error}")))?
        .to_image()
        .map_err(|error| AppError::new(format!("Could not decode AVIF: {error}")))?;

    Ok(match decoded {
        avif_decode::Image::Rgba8(buffer) => {
            into_rgba!(buffer, |pixel| Rgba([pixel.r, pixel.g, pixel.b, pixel.a]))
        }
        avif_decode::Image::Rgb8(buffer) => {
            into_rgba!(buffer, |pixel| Rgba([pixel.r, pixel.g, pixel.b, 255]))
        }
        avif_decode::Image::Gray8(buffer) => {
            into_rgba!(buffer, |pixel| {
                let value = pixel.value();
                Rgba([value, value, value, 255])
            })
        }
        avif_decode::Image::Rgba16(buffer) => into_rgba!(buffer, |pixel| Rgba([
            (pixel.r >> 8) as u8,
            (pixel.g >> 8) as u8,
            (pixel.b >> 8) as u8,
            (pixel.a >> 8) as u8,
        ])),
        avif_decode::Image::Rgb16(buffer) => into_rgba!(buffer, |pixel| Rgba([
            (pixel.r >> 8) as u8,
            (pixel.g >> 8) as u8,
            (pixel.b >> 8) as u8,
            255,
        ])),
        avif_decode::Image::Gray16(buffer) => into_rgba!(buffer, |pixel| {
            let value = (pixel.value() >> 8) as u8;
            Rgba([value, value, value, 255])
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, Rgba};

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut image = RgbaImage::from_pixel(width, height, Rgba([12, 34, 56, 255]));
        image.put_pixel(0, 0, Rgba([200, 10, 10, 255]));

        image::codecs::png::PngEncoder::new(&mut buffer)
            .write_image(
                image.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .expect("png encode");
        buffer
    }

    #[test]
    fn decodes_png_to_rgba() {
        let bytes = png_bytes(7, 5);
        let image = decode(&bytes).expect("png should decode");
        assert_eq!((image.width(), image.height()), (7, 5));
        assert_eq!(image.get_pixel(0, 0), &Rgba([200, 10, 10, 255]));
        assert_eq!(image.get_pixel(1, 0), &Rgba([12, 34, 56, 255]));
    }

    #[test]
    fn garbage_is_rejected_with_a_message() {
        let error = decode(b"this is definitely not an image").expect_err("must fail");
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn avif_sniffing_looks_at_the_ftyp_brand() {
        let mut avif = vec![0u8, 0, 0, 24];
        avif.extend_from_slice(b"ftypavif");
        avif.extend_from_slice(&[0; 8]);
        assert!(is_avif(&avif));

        let png = png_bytes(1, 1);
        assert!(!is_avif(&png));
    }
}
