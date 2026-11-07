//! Tauri command handlers, the only surface the frontend can call.
//!
//! Pure synchronous work is pushed onto the blocking pool so the WebView stays
//! responsive while large images decode.

use crate::inspect;
use crate::model::{
    ConvertOptions, ConvertResult, EstimateInput, ImageEstimate, ImageItem, PreviewPayload,
};
use crate::pipeline;use crate::state::ConversionState;
use tauri::{AppHandle, Manager};

/// Inspects dropped/selected paths. Failures are reported per file, so this
/// command only errors when the worker itself could not run.
#[tauri::command]
pub async fn inspect_files(file_paths: Vec<String>) -> Result<Vec<ImageItem>, String> {
    let joined =
        tauri::async_runtime::spawn_blocking(move || inspect::inspect_files(file_paths)).await;

    joined.map_err(|error| format!("Inspection failed: {error}"))
}

/// Converts every file in one batch. Resolves when the whole batch is done;
/// per-image progress arrives separately on the `convert-progress` event.
#[tauri::command]
pub async fn convert_images(
    app: AppHandle,
    files: Vec<ImageItem>,
    options: ConvertOptions,
) -> Result<Vec<ConvertResult>, String> {
    if files.is_empty() {
        return Ok(Vec::new());
    }

    if let Err(error) = crate::paths::validate_output_dir(&options.output_dir) {
        return Err(error.to_string());
    }

    let joined =
        tauri::async_runtime::spawn_blocking(move || -> Result<Vec<ConvertResult>, String> {
            let state = app.state::<ConversionState>();
            let token = state.begin()?;

            // A panic inside one worker must not leave the app permanently
            // "busy", so the slot is always released before returning.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pipeline::convert_batch(&app, &files, &options, &token)
            }));

            state.finish();

            match outcome {
                Ok(results) => Ok(results),
                Err(_) => {
                    crate::logging::error("Conversion batch panicked", None);
                    Err("Conversion stopped unexpectedly.".to_string())
                }
            }
        })
        .await;

    joined.map_err(|error| format!("Conversion failed to start: {error}"))?
}

/// Signals the running batch to stop after the current file. No-op otherwise.
#[tauri::command]
pub fn cancel_conversion(app: AppHandle) -> Result<(), String> {
    app.state::<ConversionState>().cancel();
    Ok(())
}

/// Both halves of the preview card for one file: the source as it is, and the
/// result of the current settings at display size.
///
/// Decoding and resizing a full photo blocks for hundreds of milliseconds, so
/// it runs on the blocking pool rather than on the IPC thread.
#[tauri::command]
pub async fn preview_pair(
    file_path: String,
    options: ConvertOptions,
) -> Result<PreviewPayload, String> {
    let joined = tauri::async_runtime::spawn_blocking(move || {
        inspect::preview_pair(&file_path, &options)
    })
    .await;

    joined
        .map_err(|error| format!("Preview failed to start: {error}"))?
        .map_err(|error| error.to_string())
}

/// Estimates what every file in a batch will weigh with `options`.
///
/// The first pass decodes each photo into a small sample, so it runs on the
/// blocking pool; later passes reuse the cached samples and only re-encode.
#[tauri::command]
pub async fn estimate_images(
    inputs: Vec<EstimateInput>,
    options: ConvertOptions,
) -> Result<Vec<ImageEstimate>, String> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let joined = tauri::async_runtime::spawn_blocking(move || {
        crate::estimate::estimate_batch(&inputs, &options)
    })
    .await;

    joined.map_err(|error| format!("Estimates failed to start: {error}"))
}
