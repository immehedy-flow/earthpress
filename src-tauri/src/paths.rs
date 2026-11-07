//! Path helpers, output naming and aspect-ratio maths.
//!
//! These are direct ports of `electron/file-utils.cjs` from the Electron build,
//! with two fixes: the filename suffix is sanitised too, and unique output
//! paths are reserved so parallel workers cannot pick the same name.

use crate::error::{AppError, AppResult};
use crate::model::{ConvertOptions, ImageFormat};
use chrono::Utc;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const SUPPORTED_EXTENSIONS: [&str; 8] =
    ["jpg", "jpeg", "png", "webp", "avif", "tif", "tiff", "bmp"];

/// True when the extension is one EarthPress can read.
pub fn is_supported(path: &Path) -> bool {
    extension_of(path)
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.as_str()))
        .unwrap_or(false)
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
}

/// Maps a file on disk to a short format name, e.g. `.jpeg` -> `jpg`.
pub fn format_from_extension(path: &Path) -> String {
    match extension_of(path).as_deref() {
        Some("jpeg") => "jpg".to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Output extension for a target format.
pub fn output_extension(format: ImageFormat) -> &'static str {
    format.extension()
}

/// Strips the characters Windows will not accept in a filename.
pub fn sanitize_filename(input: &str) -> String {
    let stripped: String = input
        .chars()
        .filter(|ch| !matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .filter(|ch| (*ch as u32) >= 32)
        .collect();

    let without_leading_dots = stripped.trim_start_matches('.');

    // Collapse runs of dots: "release..final" -> "release.final".
    let mut collapsed = String::with_capacity(without_leading_dots.len());
    let mut previous_was_dot = false;
    for ch in without_leading_dots.chars() {
        if ch == '.' {
            if previous_was_dot {
                continue;
            }
            previous_was_dot = true;
        } else {
            previous_was_dot = false;
        }
        collapsed.push(ch);
    }

    collapsed.trim().chars().take(100).collect()
}

/// Parses `"16:9"` into `16.0 / 9.0`. Falls back to `1.0`.
pub fn parse_aspect_ratio(preset: &str) -> f64 {
    let mut parts = preset.split(':');
    let width: f64 = parts.next().unwrap_or("").trim().parse().unwrap_or(0.0);
    let height: f64 = parts.next().unwrap_or("").trim().parse().unwrap_or(0.0);

    if width <= 0.0 || height <= 0.0 {
        return 1.0;
    }
    width / height
}

/// Target dimensions for an aspect-ratio preset, ported from `aspectTarget`.
pub fn aspect_target(width: u32, height: u32, preset: &str) -> (u32, u32) {
    let original_width = width.max(1);
    let original_height = height.max(1);
    let target_ratio = parse_aspect_ratio(preset);
    let original_ratio = f64::from(original_width) / f64::from(original_height);

    if original_ratio > target_ratio {
        let scaled_width = (f64::from(original_height) * target_ratio).round() as i64;
        (scaled_width.max(1) as u32, original_height)
    } else {
        let scaled_height = (f64::from(original_width) / target_ratio).round() as i64;
        (original_width, scaled_height.max(1) as u32)
    }
}

/// Checks that a path the renderer asked us to read is a real, supported file.
pub fn validate_source(path: &Path) -> AppResult<()> {
    if !path.is_absolute() {
        return Err(AppError::new("Image paths must be absolute."));
    }
    if !path.is_file() {
        return Err(AppError::new("Not a file."));
    }
    if !is_supported(path) {
        return Err(AppError::new("Unsupported file type."));
    }
    Ok(())
}

/// Checks a user-chosen output folder without creating it.
///
/// Empty means "next to each source". A non-empty folder must be absolute and
/// either already a directory or creatable inside an existing parent.
pub fn validate_output_dir(dir: &str) -> AppResult<()> {
    let trimmed = dir.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return Err(AppError::new("The output folder must be an absolute path."));
    }
    if path.exists() && !path.is_dir() {
        return Err(AppError::new("The output folder is not a folder."));
    }
    if !path.exists() {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| AppError::new("The output folder has no parent directory."))?;
        if !parent.is_dir() {
            return Err(AppError::new("The output folder's parent does not exist."));
        }
    }

    Ok(())
}

/// Builds the destination path for one source file.
///
/// An empty `outputDir` means "next to the source file" - a deliberate change
/// from the Electron build, which required a folder to be chosen first.
pub fn output_path(source: &Path, options: &ConvertOptions) -> PathBuf {
    let directory = if options.output_dir.trim().is_empty() {
        source.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        PathBuf::from(options.output_dir.trim())
    };

    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "image".to_string());

    let suffix = sanitize_filename(&options.filename_suffix);
    let extension = output_extension(options.format);

    directory.join(format!("{stem}{suffix}.{extension}"))
}

/// Hands out output paths so that two parallel workers never target the same
/// file, and never clobber an existing one unless `overwrite` is enabled.
#[derive(Default)]
pub struct OutputReserver {
    claimed: Mutex<HashSet<PathBuf>>,
}

impl OutputReserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserves a final output path for `desired`.
    pub fn reserve(&self, desired: &Path, overwrite: bool) -> PathBuf {
        let mut claimed = match self.claimed.lock() {
            Ok(guard) => guard,
            // A poisoned lock only means another worker panicked; carry on.
            Err(poisoned) => poisoned.into_inner(),
        };

        let candidate = if overwrite {
            // Overwriting is explicit, so reuse the exact requested path.
            desired.to_path_buf()
        } else {
            let parent = desired.parent().map(Path::to_path_buf).unwrap_or_default();
            let stem = desired
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            let extension = desired
                .extension()
                .map(|ext| ext.to_string_lossy().into_owned())
                .unwrap_or_default();

            let mut chosen = None;
            for index in 0..1000 {
                let probe = if index == 0 {
                    desired.to_path_buf()
                } else {
                    parent.join(format!("{stem}-{index}.{extension}"))
                };

                if !probe.exists() && !claimed.contains(&probe) {
                    chosen = Some(probe);
                    break;
                }
            }

            chosen.unwrap_or_else(|| {
                let stamp = Utc::now().timestamp_millis();
                parent.join(format!("{stem}-{stamp}.{extension}"))
            })
        };

        claimed.insert(candidate.clone());
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_windows_reserved_characters() {
        assert_eq!(sanitize_filename("-earthpress"), "-earthpress");
        assert_eq!(sanitize_filename("a<b>c:d\"e/f\\g|h?i*j"), "abcdefghij");
        assert_eq!(sanitize_filename("  padded  "), "padded");
        assert_eq!(sanitize_filename("...hidden"), "hidden");
        assert_eq!(sanitize_filename("release..final"), "release.final");
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn sanitize_truncates_to_one_hundred_characters() {
        let long = "x".repeat(500);
        assert_eq!(sanitize_filename(&long).chars().count(), 100);
    }

    #[test]
    fn jpeg_extension_normalises_to_jpg() {
        assert_eq!(format_from_extension(Path::new("C:/a/photo.JPEG")), "jpg");
        assert_eq!(format_from_extension(Path::new("C:/a/photo.PNG")), "png");
        assert_eq!(format_from_extension(Path::new("C:/a/photo.tiff")), "tiff");
        assert_eq!(format_from_extension(Path::new("C:/a/photo")), "");
    }

    #[test]
    fn only_known_extensions_are_supported() {
        assert!(is_supported(Path::new("a.PNG")));
        assert!(is_supported(Path::new("a.webp")));
        assert!(is_supported(Path::new("a.AVIF")));
        assert!(!is_supported(Path::new("a.psd")));
        assert!(!is_supported(Path::new("a")));
    }

    #[test]
    fn aspect_ratio_parsing_handles_junk() {
        assert!((parse_aspect_ratio("16:9") - 16.0 / 9.0).abs() < 1e-9);
        assert_eq!(parse_aspect_ratio("1:1"), 1.0);
        assert_eq!(parse_aspect_ratio(""), 1.0);
        assert_eq!(parse_aspect_ratio("0:5"), 1.0);
        assert_eq!(parse_aspect_ratio("nonsense"), 1.0);
    }

    #[test]
    fn aspect_target_shrinks_the_wider_dimension() {
        // A 200x100 source cropped to 1:1 keeps the height and narrows the width.
        assert_eq!(aspect_target(200, 100, "1:1"), (100, 100));
        // A 100x200 source cropped to 1:1 keeps the width and reduces the height.
        assert_eq!(aspect_target(100, 200, "1:1"), (100, 100));
        // A 4:3 source is narrower than 16:9, so the width survives untouched
        // and the height is cut down to 1600 / (16/9) = 900.
        assert_eq!(aspect_target(1600, 1200, "16:9"), (1600, 900));
        // The mirror image: a 3:4 portrait is wider than 9:16, so the height
        // survives and the width narrows to 1600 * (9/16) = 900.
        assert_eq!(aspect_target(1200, 1600, "9:16"), (900, 1600));
    }

    #[test]
    fn aspect_target_never_returns_zero() {
        let (width, height) = aspect_target(1, 1, "16:9");
        assert!(width >= 1 && height >= 1);
        let (width, height) = aspect_target(0, 0, "9:16");
        assert!(width >= 1 && height >= 1);
    }

    #[test]
    fn output_extension_follows_the_format() {        assert_eq!(output_extension(ImageFormat::Jpg), "jpg");
        assert_eq!(output_extension(ImageFormat::Png), "png");
        assert_eq!(output_extension(ImageFormat::Webp), "webp");
        assert_eq!(output_extension(ImageFormat::Avif), "avif");
    }

    fn item_options() -> ConvertOptions {
        ConvertOptions::default()
    }

    #[test]
    fn output_goes_next_to_the_source_by_default() {
        let path = output_path(Path::new("C:/pics/photo.png"), &item_options());
        assert_eq!(path, PathBuf::from("C:/pics/photo-earthpress.webp"));
    }

    #[test]
    fn output_dir_overrides_the_source_folder() {
        let options = ConvertOptions {
            output_dir: "C:/export".to_string(),
            format: ImageFormat::Avif,
            filename_suffix: "-small".to_string(),
            ..ConvertOptions::default()
        };
        let path = output_path(Path::new("C:/pics/photo.png"), &options);
        assert_eq!(path, PathBuf::from("C:/export/photo-small.avif"));
    }

    #[test]
    fn whitespace_only_output_dir_means_per_source() {
        let options = ConvertOptions {
            output_dir: "   ".to_string(),
            ..ConvertOptions::default()
        };
        let path = output_path(Path::new("C:/pics/photo.png"), &options);
        assert_eq!(path, PathBuf::from("C:/pics/photo-earthpress.webp"));
    }

    #[test]
    fn an_empty_suffix_keeps_the_original_stem() {
        let options = ConvertOptions {
            filename_suffix: String::new(),
            format: ImageFormat::Jpg,
            ..ConvertOptions::default()
        };
        let path = output_path(Path::new("C:/pics/photo.png"), &options);
        assert_eq!(path, PathBuf::from("C:/pics/photo.jpg"));
    }

    #[test]
    fn illegal_characters_are_stripped_from_the_suffix() {
        let options = ConvertOptions {
            filename_suffix: "-\\/:*?\"<>|".to_string(),
            ..ConvertOptions::default()
        };
        let path = output_path(Path::new("C:/pics/photo.png"), &options);
        assert_eq!(path, PathBuf::from("C:/pics/photo-.webp"));
    }

    #[test]
    fn multi_dot_filenames_keep_only_the_last_extension() {
        let options = ConvertOptions {
            filename_suffix: "-x".to_string(),
            ..ConvertOptions::default()
        };
        let path = output_path(Path::new("C:/pics/holiday.2024.raw.png"), &options);
        assert_eq!(path, PathBuf::from("C:/pics/holiday.2024.raw-x.webp"));
    }

    #[test]
    fn reserver_hands_out_distinct_paths_for_the_same_target() {
        let reserver = OutputReserver::new();
        let dir = std::env::temp_dir().join("earthpress-reserver-test");
        let desired = dir.join("photo-earthpress.webp");

        let first = reserver.reserve(&desired, false);
        let second = reserver.reserve(&desired, false);
        let third = reserver.reserve(&desired, false);

        assert_eq!(first, desired);
        assert_eq!(second, dir.join("photo-earthpress-1.webp"));
        assert_eq!(third, dir.join("photo-earthpress-2.webp"));
    }

    #[test]
    fn reserver_reuses_the_requested_path_when_overwriting() {
        let reserver = OutputReserver::new();
        let desired = std::env::temp_dir().join("earthpress-overwrite-test.jpg");
        assert_eq!(reserver.reserve(&desired, true), desired);
        assert_eq!(reserver.reserve(&desired, true), desired);
    }

    #[test]
    fn output_dir_validation_accepts_empty_and_rejects_relative_paths() {
        assert!(validate_output_dir("").is_ok());
        assert!(validate_output_dir("   ").is_ok());
        assert!(validate_output_dir("relative/folder").is_err());

        let dir = std::env::temp_dir();
        assert!(validate_output_dir(&dir.to_string_lossy()).is_ok());
    }

    #[test]
    fn output_dir_validation_rejects_files() {
        let dir = std::env::temp_dir().join("earthpress-validate-dir-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("not-a-folder.png");
        std::fs::write(&file, b"x").expect("write file");

        assert!(validate_output_dir(&file.to_string_lossy()).is_err());

        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn source_validation_requires_an_absolute_supported_file() {
        assert!(validate_source(Path::new("relative.png")).is_err());
        assert!(validate_source(Path::new("C:/does/not/exist.png")).is_err());

        let dir = std::env::temp_dir().join("earthpress-validate-source-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("image.png");
        std::fs::write(&file, b"x").expect("write file");

        assert!(validate_source(&file).is_ok());

        let _ = std::fs::remove_file(&file);
    }
}
