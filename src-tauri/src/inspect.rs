//! File inspection: read metadata, decode once, and hand the UI a thumbnail.
//!
//! Mirrors `inspectImages` from `electron/main.cjs`, but builds the preview
//! with a JPEG data URI instead of a WebP one. The previews are small and the
//! payload has to travel over IPC for every file in the list, so keeping it
//! compact matters more than matching the old MIME type.

use crate::base64;
use crate::codecs;
use crate::decode;
use crate::error::AppResult;
use crate::estimate;
use crate::logging;
use crate::model::{ConvertOptions, ImageFormat, ImageItem, PreviewPayload, QualityMode};
use crate::orientation;
use crate::parallel;
use crate::paths;
use crate::resize;
use crate::resize::DEFAULT_BACKGROUND;
use image::{imageops, ImageEncoder, Rgb, RgbImage, RgbaImage};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Preview bounding box, matching the CSS thumbnail size with headroom.
const PREVIEW_MAX_WIDTH: u32 = 220;
const PREVIEW_MAX_HEIGHT: u32 = 160;
const PREVIEW_QUALITY: u8 = 72;

/// The before/after card is roughly 700 CSS px wide. 1600 keeps it sharp on a
/// high-DPI window without shipping a full-size photo over IPC.
pub const DISPLAY_PREVIEW_EDGE: u32 = 1600;
const DISPLAY_PREVIEW_QUALITY: u8 = 88;

/// Inspection is dominated by decode time, so four workers is plenty.
const INSPECT_WORKERS: usize = 4;

/// Reads one file. Errors describe what went wrong for that specific file.
pub fn inspect_image(path: &str) -> AppResult<ImageItem> {
    let file_path = Path::new(path);
    let metadata = std::fs::metadata(file_path)?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    let bytes = decode::read_source(file_path)?;
    let decoded = decode::decode(&bytes)?;
    let orientation = orientation::read_orientation(&bytes);

    // Report the dimensions the user actually sees, which are post-rotation:
    // a portrait phone photo stores landscape pixels plus an EXIF tag.
    let (mut width, mut height) = (decoded.width(), decoded.height());
    if orientation::swaps_axes(orientation) {
        std::mem::swap(&mut width, &mut height);
    }

    Ok(ImageItem {
        // File path plus mtime keeps ids stable across reloads but unique when a
        // file is replaced, the same trick the Electron build used.
        id: format!("{path}::{modified_ms}"),
        path: path.to_string(),
        name: file_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string()),
        directory: file_path
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned())
            .unwrap_or_default(),
        format: paths::format_from_extension(file_path),
        width,
        height,
        size: metadata.len(),
        preview: preview_data_url(
            &decoded,
            orientation,
            PREVIEW_MAX_WIDTH,
            PREVIEW_MAX_HEIGHT,
            PREVIEW_QUALITY,
            imageops::FilterType::Triangle,
        ),
        error: None,
    })
}

/// Inspects a batch, deduplicating paths and preserving the caller's order.
pub fn inspect_files(file_paths: Vec<String>) -> Vec<ImageItem> {
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<String> = file_paths
        .into_iter()
        .filter(|path| !path.is_empty() && seen.insert(path.clone()))
        .collect();

    parallel::map_ordered(&unique, INSPECT_WORKERS, |path| {
        if !paths::is_supported(Path::new(path)) {
            return ImageItem::failed(path, "Unsupported file type.".to_string());
        }

        match inspect_image(path) {
            Ok(item) => item,
            Err(error) => {
                logging::warn(
                    format!(
                        "Could not read {}",
                        Path::new(path)
                            .file_name()
                            .map(|name| name.to_string_lossy())
                            .unwrap_or_else(|| "image".into())
                    ),
                    Some(serde_json::json!({ "error": error.to_string() })),
                );
                ImageItem::failed(path, error.to_string())
            }
        }
    })
}

/// Downscales, applies the EXIF rotation, flattens onto the app background,
/// then encodes a JPEG data URI.
///
/// Rotation happens *after* downscaling: for a 90/270 tag that is the same
/// result as rotating first, at a fraction of the cost on a large photo.
fn preview_data_url(
    image: &RgbaImage,
    orientation: u16,
    max_width: u32,
    max_height: u32,
    quality: u8,
    filter: imageops::FilterType,
) -> String {
    if image.width() == 0 || image.height() == 0 {
        return String::new();
    }

    let swaps = orientation::swaps_axes(orientation);
    let (source_width, source_height) = if swaps {
        (image.height(), image.width())
    } else {
        (image.width(), image.height())
    };

    let scale = f64::min(
        f64::from(max_width) / f64::from(source_width),
        f64::from(max_height) / f64::from(source_height),
    )
    .min(1.0);

    let target_width = ((f64::from(source_width) * scale).round() as u32).max(1);
    let target_height = ((f64::from(source_height) * scale).round() as u32).max(1);

    // The box is expressed in display axes, so swap back before resizing.
    let (resize_width, resize_height) = if swaps {
        (target_height, target_width)
    } else {
        (target_width, target_height)
    };

    let resized = if resize_width == image.width() && resize_height == image.height() {
        image.clone()
    } else {
        imageops::resize(image, resize_width, resize_height, filter)
    };

    let oriented = orientation::apply(resized, orientation);

    // Flat onto the app background by hand: `image`'s JPEG encoder only takes
    // RGB, and transparent pixels would otherwise come out black. The
    // background is opaque, so a plain source-over blend is all that is needed.
    let flat = RgbImage::from_fn(oriented.width(), oriented.height(), |x, y| {
        let pixel = oriented.get_pixel(x, y);
        let alpha = u32::from(pixel[3]);
        let over = |channel: u8, behind: u8| -> u8 {
            let front = u32::from(channel) * alpha;
            let back = u32::from(behind) * (255 - alpha);
            ((front + back + 127) / 255) as u8
        };

        Rgb([
            over(pixel[0], DEFAULT_BACKGROUND[0]),
            over(pixel[1], DEFAULT_BACKGROUND[1]),
            over(pixel[2], DEFAULT_BACKGROUND[2]),
        ])
    });

    let mut buffer: Vec<u8> = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);

    if encoder
        .write_image(
            flat.as_raw(),
            oriented.width(),
            oriented.height(),
            image::ExtendedColorType::Rgb8,
        )
        .is_err()
    {
        return String::new();
    }

    format!("data:image/jpeg;base64,{}", base64::encode(&buffer))
}

/// Renders both halves of the preview card: the source as it looks today, and
/// the same photo pushed through the real geometry and encoder at display size.
///
/// Encoding at display size rather than full size keeps the card responsive
/// while still showing exactly what the chosen quality does to the pixels.
pub fn preview_pair(path: &str, options: &ConvertOptions) -> AppResult<PreviewPayload> {
    paths::validate_source(Path::new(path))?;
    let bytes = decode::read_source(Path::new(path))?;
    let decoded = decode::decode(&bytes)?;
    let orientation = orientation::read_orientation(&bytes);

    let before = preview_data_url(
        &decoded,
        orientation,
        DISPLAY_PREVIEW_EDGE,
        DISPLAY_PREVIEW_EDGE,
        DISPLAY_PREVIEW_QUALITY,
        imageops::FilterType::Lanczos3,
    );

    let oriented = orientation::apply(decoded, orientation);
    let geometry = resize::apply(&oriented, options);

    let mut encode_options = options.clone();
    encode_options.quality_mode = QualityMode::Level;
    if estimate::wants_target(options) {
        let pixels = u64::from(geometry.width()) * u64::from(geometry.height());
        let probe = estimate::downscale(&geometry, estimate::SAMPLE_EDGE);
        let (quality, _, _) =
            estimate::search_quality(&probe, pixels, options, options.target_bytes);
        encode_options.quality = quality;
    }

    let display = estimate::downscale(&geometry, DISPLAY_PREVIEW_EDGE);
    let encoded = codecs::encode(&display, &encode_options)?;

    Ok(PreviewPayload {
        before_data_url: before,
        after_data_url: format!(
            "data:{};base64,{}",
            mime_type(encode_options.format),
            base64::encode(&encoded)
        ),
        output_width: geometry.width(),
        output_height: geometry.height(),
        quality_used: encode_options.quality,
    })
}

fn mime_type(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::Webp => "image/webp",
        ImageFormat::Avif => "image/avif",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn write_png(dir: &Path, name: &str, width: u32, height: u32) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut image = RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255]);
        }

        let mut buffer = Vec::new();
        image::codecs::png::PngEncoder::new(&mut buffer)
            .write_image(
                image.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .expect("png encode");

        std::fs::write(&path, buffer).expect("write png");
        path
    }

    #[test]
    fn inspects_a_real_file() {
        let dir = std::env::temp_dir().join("earthpress-inspect-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = write_png(&dir, "sample.png", 300, 200);

        let item = inspect_image(path.to_str().expect("utf-8 path")).expect("inspect");

        assert_eq!(item.name, "sample.png");
        assert_eq!(item.format, "png");
        assert_eq!((item.width, item.height), (300, 200));
        assert!(item.size > 0);
        assert!(item.id.contains("sample.png"));
        assert!(!item.directory.is_empty());
        assert!(item.preview.starts_with("data:image/jpeg;base64,"));
        assert!(item.error.is_none());
    }

    #[test]
    fn previews_are_capped_at_the_thumbnail_box() {
        let dir = std::env::temp_dir().join("earthpress-preview-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = write_png(&dir, "wide.png", 1200, 300);

        let item = inspect_image(path.to_str().expect("utf-8 path")).expect("inspect");
        let preview = item.preview.split("base64,").nth(1).expect("data uri");

        // Base64 of a 220x55 JPEG is a few KB at most.
        assert!(
            preview.len() < 40_000,
            "preview was {} bytes",
            preview.len()
        );
    }

    #[test]
    fn the_preview_pair_renders_both_halves() {
        let dir = std::env::temp_dir().join("earthpress-preview-pair-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = write_png(&dir, "pair.png", 2400, 1600);

        let options = ConvertOptions {
            size_mode: crate::model::SizeMode::Resize,
            resize_kind: crate::model::ResizeKind::Max,
            resize_value: 1600,
            ..ConvertOptions::default()
        };

        let pair = preview_pair(path.to_str().expect("utf-8 path"), &options).expect("preview");

        // The original half is always a display JPEG; the result half is the
        // real output format, so the card shows the encoder that will run.
        assert!(pair.before_data_url.starts_with("data:image/jpeg;base64,"));
        assert!(pair.after_data_url.starts_with("data:image/webp;base64,"));
        assert!(pair.before_data_url.len() > 10_000);
        assert!(pair.after_data_url.len() > 1_000);

        // 2400x1600 capped at 1600 on the long side.
        assert_eq!((pair.output_width, pair.output_height), (1600, 1067));
        assert_eq!(pair.quality_used, options.quality);
    }

    #[test]
    fn unsupported_extensions_become_error_items() {
        let items = inspect_files(vec!["C:/tmp/notes.txt".to_string()]);
        assert_eq!(items.len(), 1);
        assert!(items[0].error.is_some());
        assert_eq!(items[0].name, "notes.txt");
    }

    #[test]
    fn duplicate_paths_are_only_inspected_once() {
        let items = inspect_files(vec![
            "C:/tmp/does-not-exist.png".to_string(),
            "C:/tmp/does-not-exist.png".to_string(),
        ]);
        assert_eq!(items.len(), 1);
        assert!(items[0].error.is_some());
    }

    #[test]
    fn empty_paths_are_dropped() {
        assert!(inspect_files(vec![String::new()]).is_empty());
    }
}
