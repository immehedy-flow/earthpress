//! EarthPress: a local image converter and compressor.
//!
//! The React frontend is unchanged from the Electron 1.3.0 build apart from the
//! IPC bridge in `src/bridge.ts`. Everything below is the native replacement
//! for `electron/image-processor.cjs` and friends.

mod base64;
mod codecs;
mod commands;
mod decode;
mod error;
mod estimate;
mod inspect;
mod logging;
mod model;
mod orientation;
mod parallel;
mod paths;
mod pipeline;
mod resize;
mod smartcrop;
mod state;

use state::ConversionState;
use tauri::Manager;

pub fn run() {
    logging::info("EarthPress starting", None);

    tauri::Builder::default()
        // Registered for the frontend's native file/folder dialogs and
        // "show in folder" calls; Rust does not call either plugin directly.
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(ConversionState::default())
        .invoke_handler(tauri::generate_handler![
            commands::inspect_files,
            commands::convert_images,
            commands::cancel_conversion,
            commands::preview_pair,
            commands::estimate_images,
        ])
        .setup(|app| {
            // Follow the OS theme until the webview reports its stored choice.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_theme(None);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start EarthPress");
}
