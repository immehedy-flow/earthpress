# 🌿 EarthPress — Local Image Converter & Compressor for Windows

An **Electron + React + Vite** desktop app powered by [**sharp**](https://sharp.pixelplumbing.com/) for fast, offline image processing.

## Core Functionalities

| # | Feature | Description |
|---|---------|-------------|
| **1** | **Format Conversion** | Convert images between **WebP, JPG, PNG, and AVIF** with a single click |
| **2** | **Quality Control** | Adjustable quality slider (1–100), plus **lossless mode** for WebP/AVIF |
| **3** | **Resizing** | Four size modes — **Original**, **Resize** (by width/height/max-side/percent), **Aspect Ratio** presets (1:1, 4:5, 16:9, etc.), and **Custom** (W×H) |
| **4** | **Smart Cropping & Fit** | Object-fit options (cover, contain, fill, inside, outside) with focus-point selection — including **smart attention** and **entropy-based** cropping |
| **5** | **Batch Processing** | Add multiple images via file picker or **drag-and-drop**, convert them all at once, with per-image progress tracking and cancellation |
| **6** | **Presets** | Save & recall named conversion presets (stored in `localStorage`) for repeatable workflows |
| **7** | **Output Management** | Configurable output folder, filename suffix (e.g., `-earthpress`), overwrite toggle, and "Open latest output" to reveal in Explorer |
| **8** | **Advanced Options** | Allow upscaling, indexed PNG palette, metadata preservation, and PNG compression level control |
| **9** | **Dark/Light Theme** | Theme toggle with instant switching |
| **10** | **Keyboard Shortcuts** | `Ctrl+O` (add images), `Ctrl+Shift+O` (output folder), `Ctrl+Enter` (convert), `Esc` (cancel), `Ctrl+D` (clear all) |
| **11** | **Sorting & Reordering** | Sort the image list by name/size/width/height/format, or drag-to-reorder manually |
| **12** | **Savings Summary** | Top bar shows total file count, combined size, and bytes saved after conversion |

## Architecture

- **Frontend**: React 19 + TypeScript + Vite, with Lucide icons
- **Backend**: Electron 43 main process using **sharp** for all image operations (`electron/image-processor.cjs`)
- **IPC Bridge**: Preload script exposes `window.earthpress` API (select images, inspect files, convert, cancel, reveal path)
- **Distribution**: Built with `electron-builder` as an NSIS installer for Windows x64

Everything runs **100% locally** — no cloud uploads, no internet required. 🔒
