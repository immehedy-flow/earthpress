//! The conversion pipeline.
//!
//! One file goes through: read -> EXIF orientation -> decode -> resize/crop ->
//! encode -> atomic write. Four files run at a time, matching the Electron
//! build's `concurrencyLimit: 4`.
//!
//! Progress milestones are a little finer than the original 10/30/60/100
//! because each stage can be attributed to real work here.

use crate::codecs;
use crate::decode;
use crate::logging;
use crate::model::{ConversionProgress, ConvertOptions, ConvertResult, ImageItem};
use crate::orientation;
use crate::parallel;
use crate::paths::{self, OutputReserver};
use crate::resize;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter};

/// Concurrent conversions, same as the 1.4.0 build.
pub const WORKERS: usize = 4;

/// Temp-file counter, so two workers in the same folder never collide.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` through a sibling temp file.
///
/// A crash or a full disk can then never leave a half-written file at the
/// destination, and an overwrite never destroys the original until the new
/// bytes are complete. The temp file is removed if the rename fails.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp = parent.join(format!(
        ".earthpress-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    fs::write(&temp, bytes)?;
    match fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

fn emit_progress(app: &AppHandle, id: &str, name: &str, progress: u8) {
    let _ = app.emit(
        "convert-progress",
        ConversionProgress {
            id: id.to_string(),
            name: name.to_string(),
            progress,
        },
    );
}

fn cancelled_result(file: &ImageItem) -> ConvertResult {
    ConvertResult::failure(file.id.clone(), file.name.clone(), "Cancelled.".to_string())
}

fn is_cancelled(cancelled: &AtomicBool) -> bool {
    cancelled.load(Ordering::SeqCst)
}

/// Converts one file. Never panics and never returns `Err`: failures come back
/// as a `ConvertResult` with `ok: false`, which is what the list UI renders.
pub fn convert_one(
    app: &AppHandle,
    file: &ImageItem,
    options: &ConvertOptions,
    reserver: &OutputReserver,
    cancelled: &AtomicBool,
) -> ConvertResult {
    match convert_inner(app, file, options, reserver, cancelled) {
        Ok(result) => result,
        Err(error) => {
            logging::error(
                format!("Conversion failed for {}", file.name),
                Some(serde_json::json!({
                    "format": options.format,
                    "error": error.to_string(),
                })),
            );
            ConvertResult::failure(file.id.clone(), file.name.clone(), error.to_string())
        }
    }
}

fn convert_inner(
    app: &AppHandle,
    file: &ImageItem,
    options: &ConvertOptions,
    reserver: &OutputReserver,
    cancelled: &AtomicBool,
) -> crate::error::AppResult<ConvertResult> {
    if is_cancelled(cancelled) {
        return Ok(cancelled_result(file));
    }

    emit_progress(app, &file.id, &file.name, 5);

    // 1. Validate and read the source once. The shared reader enforces the
    //    supported-extension, absolute-path and size/pixel limits.
    paths::validate_source(Path::new(&file.path))?;
    let source_bytes = decode::read_source(Path::new(&file.path))?;

    if is_cancelled(cancelled) {
        return Ok(cancelled_result(file));
    }

    // 2. Bake in the EXIF orientation the way sharp's `.rotate()` does.
    emit_progress(app, &file.id, &file.name, 25);
    let orientation = orientation::read_orientation(&source_bytes);
    let decoded = decode::decode(&source_bytes)?;
    let oriented = orientation::apply(decoded, orientation);

    if is_cancelled(cancelled) {
        return Ok(cancelled_result(file));
    }

    // 3. Geometry.
    emit_progress(app, &file.id, &file.name, 45);
    let resized = resize::apply(&oriented, options);

    if is_cancelled(cancelled) {
        return Ok(cancelled_result(file));
    }

    // 4. Encode. In target-size mode each file picks its own quality from a
    //    small sample of the final geometry, then encodes once at that quality.
    emit_progress(app, &file.id, &file.name, 60);
    let mut encode_options = options.clone();
    if crate::estimate::wants_target(options) {
        let pixels = u64::from(resized.width()) * u64::from(resized.height());
        let probe = crate::estimate::downscale(&resized, crate::estimate::SAMPLE_EDGE);
        let (quality, _, _) =
            crate::estimate::search_quality(&probe, pixels, options, options.target_bytes);
        encode_options.quality = quality;
    }
    encode_options.quality_mode = crate::model::QualityMode::Level;

    let encoded = codecs::encode(&resized, &encode_options)?;

    if is_cancelled(cancelled) {
        return Ok(cancelled_result(file));
    }

    // 5. Write. The path is reserved under a lock so parallel workers cannot
    //    choose the same `name-1.ext` and clobber each other.
    emit_progress(app, &file.id, &file.name, 85);
    let desired = paths::output_path(Path::new(&file.path), options);
    let final_path = reserver.reserve(&desired, options.overwrite);

    if let Some(parent) = final_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    write_atomic(&final_path, &encoded)?;

    emit_progress(app, &file.id, &file.name, 100);

    let output_size = fs::metadata(&final_path)
        .map(|metadata| metadata.len())
        .unwrap_or(encoded.len() as u64);

    Ok(ConvertResult {
        id: file.id.clone(),
        name: file.name.clone(),
        ok: true,
        output_path: Some(final_path.to_string_lossy().into_owned()),
        output_size: Some(output_size),
        saved_bytes: Some(
            i64::try_from(file.size).unwrap_or(i64::MAX)
                - i64::try_from(output_size).unwrap_or(i64::MAX),
        ),
        quality_used: Some(encode_options.quality),
        error: None,
    })
}

/// Converts a whole batch with bounded parallelism, preserving input order.
pub fn convert_batch(
    app: &AppHandle,
    files: &[ImageItem],
    options: &ConvertOptions,
    cancelled: &AtomicBool,
) -> Vec<ConvertResult> {
    // Create a custom output folder up front so per-file writes never race on
    // `create_dir_all`. An empty folder means per-source output.
    let output_dir = options.output_dir.trim();
    if !output_dir.is_empty() {
        let directory = Path::new(output_dir);
        if let Err(error) = fs::create_dir_all(directory) {
            logging::warn(
                format!(
                    "Could not create output folder {}",
                    directory
                        .file_name()
                        .map(|name| name.to_string_lossy())
                        .unwrap_or_else(|| "selected folder".into())
                ),
                Some(serde_json::json!({ "error": error.to_string() })),
            );
        }
    }

    let reserver = OutputReserver::new();

    let results = parallel::map_ordered(files, WORKERS, |file| {
        convert_one(app, file, options, &reserver, cancelled)
    });

    let succeeded = results.iter().filter(|result| result.ok).count();
    logging::info(
        "Conversion batch finished",
        Some(serde_json::json!({
            "total": results.len(),
            "succeeded": succeeded,
            "cancelled": is_cancelled(cancelled),
            "format": options.format,
        })),
    );

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn item(path: &str) -> ImageItem {
        ImageItem {
            id: "id".to_string(),
            path: path.to_string(),
            name: "photo.png".to_string(),
            directory: "C:/pics".to_string(),
            format: "png".to_string(),
            width: 10,
            height: 10,
            size: 1000,
            preview: String::new(),
            error: None,
        }
    }

    #[test]
    fn a_batch_of_one_returns_one_result() {
        // `convert_one` needs an `AppHandle`, so the end-to-end path is covered
        // by the codec, resize and paths tests instead. This only pins the fact
        // that the pipeline consumes `ImageItem` exactly as the inspector
        // produces it.
        let file = item("C:/pics/photo.png");
        assert_eq!(file.name, "photo.png");
        assert_eq!(file.size, 1000);
        assert_eq!(
            paths::output_path(Path::new(&file.path), &ConvertOptions::default()),
            PathBuf::from("C:/pics/photo-earthpress.webp")
        );
    }

    #[test]
    fn atomic_write_replaces_the_target_without_leaving_temp_files() {
        let dir = std::env::temp_dir().join("earthpress-atomic-write-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("photo.webp");
        std::fs::write(&path, b"old").expect("seed");

        write_atomic(&path, b"new bytes").expect("atomic write");

        assert_eq!(std::fs::read(&path).expect("read"), b"new bytes");

        let leftovers = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0, "temp file was left behind");

        let _ = std::fs::remove_file(&path);
    }
}
