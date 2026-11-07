//! Serde types shared with the React frontend.
//!
//! Field names are camelCase on the wire so they line up with
//! `src/types.d.ts` without any translation layer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpg,
    Png,
    Webp,
    Avif,
}

impl ImageFormat {
    /// File extension to write for this format.
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Jpg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Webp => "webp",
            ImageFormat::Avif => "avif",
        }
    }

    /// True when the encoder understands the `quality` slider.
    ///
    /// The conversion path itself never branches on these - the UI owns the
    /// slider - but they document the format matrix and the tests pin them.
    #[cfg(test)]
    pub fn supports_quality(self) -> bool {
        !matches!(self, ImageFormat::Png)
    }

    /// True when the encoder has a real lossless mode.
    #[cfg(test)]
    pub fn supports_lossless(self) -> bool {
        matches!(self, ImageFormat::Webp | ImageFormat::Avif)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeMode {
    Original,
    Resize,
    Aspect,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResizeKind {
    Width,
    Height,
    Max,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    Cover,
    Contain,
    Fill,
    Inside,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CropPosition {
    Center,
    Top,
    Right,
    Bottom,
    Left,
    /// Saliency based focus (approximation of libvips "attention").
    Attention,
    /// Shannon-entropy based focus.
    Entropy,
}

fn default_quality() -> u8 {
    82
}

fn default_resize_value() -> u32 {
    1600
}

fn default_aspect_preset() -> String {
    "1:1".to_string()
}

fn default_custom_size() -> u32 {
    1080
}

fn default_background() -> String {
    "#fffaf2".to_string()
}

fn default_png_compression() -> u8 {
    8
}

fn default_size_mode() -> SizeMode {
    SizeMode::Original
}

fn default_resize_kind() -> ResizeKind {
    ResizeKind::Max
}

fn default_fit_mode() -> FitMode {
    FitMode::Cover
}

fn default_crop_position() -> CropPosition {
    CropPosition::Center
}

/// How the encoder chooses its quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QualityMode {
    /// One quality for the whole batch.
    #[default]
    Level,
    /// Per file: the highest quality that fits the byte target.
    Target,
}

/// Mirrors `ConvertOptions` in `src/types.d.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOptions {
    #[serde(default = "default_format")]
    pub format: ImageFormat,

    #[serde(default = "default_quality")]
    pub quality: u8,

    #[serde(default)]
    pub lossless: bool,

    /// Level picks one quality for the batch; Target lets each file find the
    /// highest quality that still fits `target_bytes`.
    #[serde(default)]
    pub quality_mode: QualityMode,

    /// Target output size in bytes. Only read when `quality_mode` is `Target`.
    #[serde(default)]
    pub target_bytes: u64,

    #[serde(default = "default_size_mode")]
    pub size_mode: SizeMode,

    #[serde(default = "default_resize_kind")]
    pub resize_kind: ResizeKind,

    /// Legacy field kept for wire compatibility with older saved presets.
    #[serde(default)]
    pub resize_mode: Option<String>,

    #[serde(default = "default_resize_value")]
    pub resize_value: u32,

    #[serde(default = "default_aspect_preset")]
    pub aspect_preset: String,

    #[serde(default = "default_custom_size")]
    pub custom_width: u32,

    #[serde(default = "default_custom_size")]
    pub custom_height: u32,

    #[serde(default = "default_fit_mode")]
    pub fit_mode: FitMode,

    #[serde(default = "default_crop_position")]
    pub crop_position: CropPosition,

    #[serde(default = "default_background")]
    pub background: String,

    #[serde(default)]
    pub allow_upscale: bool,

    #[serde(default = "default_png_compression")]
    pub png_compression: u8,

    #[serde(default)]
    pub png_palette: bool,

    #[serde(default)]
    pub overwrite: bool,

    #[serde(default)]
    pub filename_suffix: String,

    /// Empty means "write next to each source file".
    #[serde(default)]
    pub output_dir: String,
}

fn default_format() -> ImageFormat {
    ImageFormat::Webp
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            format: ImageFormat::Webp,
            quality: default_quality(),
            lossless: false,
            quality_mode: QualityMode::Level,
            target_bytes: 0,
            size_mode: SizeMode::Original,
            resize_kind: ResizeKind::Max,
            resize_mode: None,
            resize_value: default_resize_value(),
            aspect_preset: default_aspect_preset(),
            custom_width: 1080,
            custom_height: 1080,
            fit_mode: FitMode::Cover,
            crop_position: CropPosition::Center,
            background: default_background(),
            allow_upscale: false,
            png_compression: default_png_compression(),
            png_palette: false,
            overwrite: false,
            filename_suffix: "-earthpress".to_string(),
            output_dir: String::new(),
        }
    }
}

/// Mirrors `ImageItem` in `src/types.d.ts`. Every field is defaulted so that
/// the partial error objects the inspector produces also round-trip.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub directory: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub preview: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ImageItem {
    /// The shape the old Electron backend returned for unreadable files.
    pub fn failed(path: &str, message: String) -> Self {
        let path_buf = std::path::Path::new(path);
        Self {
            name: path_buf
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string()),
            path: path.to_string(),
            error: Some(message),
            ..Default::default()
        }
    }
}

/// The minimum an estimate needs to know about one file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateInput {
    pub id: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
}

/// Mirrors `ImageEstimate` in `src/types.d.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageEstimate {
    pub id: String,
    pub output_width: u32,
    pub output_height: u32,
    pub estimated_bytes: u64,
    pub quality_used: u8,
    pub meets_target: bool,
}

/// Mirrors `PreviewPayload` in `src/types.d.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPayload {
    pub before_data_url: String,
    pub after_data_url: String,
    pub output_width: u32,
    pub output_height: u32,
    pub quality_used: u8,
}

/// Mirrors `ConvertResult` in `src/types.d.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertResult {
    pub id: String,
    pub name: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_bytes: Option<i64>,
    /// The quality the encoder actually used, which differs from the slider in
    /// target-size mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_used: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ConvertResult {
    pub fn failure(id: String, name: String, error: String) -> Self {
        Self {
            id,
            name,
            ok: false,
            output_path: None,
            output_size: None,
            saved_bytes: None,
            quality_used: None,
            error: Some(error),
        }
    }
}

/// Mirrors `ConversionProgress` in `src/types.d.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionProgress {
    pub id: String,
    pub name: String,
    pub progress: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_deserialize_from_camel_case() {
        let json = r##"{
            "format": "avif",
            "quality": 64,
            "sizeMode": "aspect",
            "resizeKind": "percent",
            "resizeValue": 50,
            "aspectPreset": "16:9",
            "customWidth": 800,
            "customHeight": 600,
            "fitMode": "contain",
            "cropPosition": "entropy",
            "background": "#123456",
            "allowUpscale": true,
            "pngCompression": 3,
            "pngPalette": true,
            "overwrite": true,
            "filenameSuffix": "-x",
            "outputDir": "C:/out"
        }"##;

        let options: ConvertOptions = serde_json::from_str(json).expect("options should parse");
        assert_eq!(options.format, ImageFormat::Avif);
        assert_eq!(options.quality, 64);
        assert_eq!(options.size_mode, SizeMode::Aspect);
        assert_eq!(options.resize_kind, ResizeKind::Percent);
        assert_eq!(options.aspect_preset, "16:9");
        assert_eq!(options.fit_mode, FitMode::Contain);
        assert_eq!(options.crop_position, CropPosition::Entropy);
        assert!(options.allow_upscale);
        assert!(options.png_palette);
        assert_eq!(options.output_dir, "C:/out");
    }

    #[test]
    fn options_fill_in_defaults_for_partial_presets() {
        let options: ConvertOptions =
            serde_json::from_str(r#"{"format":"png"}"#).expect("partial options should parse");
        assert_eq!(options.quality, 82);
        assert_eq!(options.size_mode, SizeMode::Original);
        assert_eq!(options.resize_value, 1600);
        assert_eq!(options.aspect_preset, "1:1");
        assert_eq!(options.custom_width, 1080);
        assert_eq!(options.background, "#fffaf2");
        assert_eq!(options.png_compression, 8);
        assert!(!options.lossless);
    }

    #[test]
    fn failed_item_serializes_only_id_name_path_and_error() {
        let item = ImageItem::failed("C:/pics/broken.png", "unsupported".into());
        let value = serde_json::to_value(&item).expect("serializable");
        assert_eq!(value["name"], "broken.png");
        assert_eq!(value["error"], "unsupported");
        assert_eq!(value["width"], 0);
        assert_eq!(value["preview"], "");
    }

    #[test]
    fn format_extension_and_lossless_support() {
        assert_eq!(ImageFormat::Jpg.extension(), "jpg");
        assert_eq!(ImageFormat::Avif.extension(), "avif");
        assert!(ImageFormat::Webp.supports_lossless());
        assert!(ImageFormat::Avif.supports_lossless());
        assert!(!ImageFormat::Jpg.supports_lossless());
        assert!(!ImageFormat::Png.supports_quality());
        assert!(ImageFormat::Jpg.supports_quality());
    }
}
