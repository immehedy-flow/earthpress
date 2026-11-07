//! Geometry: size modes, object-fit modes and focus-aware cropping.
//!
//! The whole pipeline works on `RgbaImage`, mirroring how sharp normalises
//! everything to RGBA internally before handing pixels to an encoder.
//!
//! Filter choice matters for fidelity with sharp: it defaults to Lanczos3,
//! and so do we.

use crate::model::{ConvertOptions, CropPosition, FitMode, ResizeKind, SizeMode};
use crate::paths::aspect_target;
use crate::smartcrop::focus_origin;
use image::{imageops, Rgba, RgbaImage};

/// Matches `fitBackground()` in the Electron build.
pub const DEFAULT_BACKGROUND: [u8; 3] = [255, 250, 242];

fn resize_to(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let width = width.max(1);
    let height = height.max(1);

    if image.width() == width && image.height() == height {
        return image.clone();
    }

    imageops::resize(image, width, height, imageops::FilterType::Lanczos3)
}

fn scale_dimension(original: u32, factor: f64) -> u32 {
    ((f64::from(original) * factor).round() as i64).max(1) as u32
}

fn scaled_size(width: u32, height: u32, factor: f64) -> (u32, u32) {
    (
        scale_dimension(width, factor),
        scale_dimension(height, factor),
    )
}

/// Parses `#rrggbb` into an opaque pixel, falling back to the warm default.
pub fn parse_background(value: &str) -> Rgba<u8> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 {
        if let Ok(rgb) = u32::from_str_radix(hex, 16) {
            return Rgba([
                ((rgb >> 16) & 0xff) as u8,
                ((rgb >> 8) & 0xff) as u8,
                (rgb & 0xff) as u8,
                255,
            ]);
        }
    }

    Rgba([
        DEFAULT_BACKGROUND[0],
        DEFAULT_BACKGROUND[1],
        DEFAULT_BACKGROUND[2],
        255,
    ])
}

/// Applies the configured size mode to `source`.
pub fn apply(source: &RgbaImage, options: &ConvertOptions) -> RgbaImage {
    if source.width() == 0 || source.height() == 0 {
        return source.clone();
    }

    match options.size_mode {
        SizeMode::Original => source.clone(),
        SizeMode::Resize => resize_mode(source, options),
        SizeMode::Aspect => {
            let (target_width, target_height) =
                aspect_target(source.width(), source.height(), &options.aspect_preset);
            fit(source, target_width, target_height, options)
        }
        SizeMode::Custom => {
            let target_width = options.custom_width.max(1);
            let target_height = options.custom_height.max(1);
            fit(source, target_width, target_height, options)
        }
    }
}

/// The output size for a source of `width` x `height`, without touching pixels.
///
/// The UI shows these numbers next to every file, so they have to agree with
/// what `apply` actually produces. `dimensions_match_apply` pins that down.
pub fn output_dimensions(width: u32, height: u32, options: &ConvertOptions) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width, height);
    }

    match options.size_mode {
        SizeMode::Original => (width, height),
        SizeMode::Resize => resize_mode_dimensions(width, height, options),
        SizeMode::Aspect => {
            let (target_width, target_height) =
                aspect_target(width, height, &options.aspect_preset);
            fit_dimensions(width, height, target_width, target_height, options)
        }
        SizeMode::Custom => fit_dimensions(
            width,
            height,
            options.custom_width.max(1),
            options.custom_height.max(1),
            options,
        ),
    }
}

fn resize_mode_dimensions(width: u32, height: u32, options: &ConvertOptions) -> (u32, u32) {
    let value = options.resize_value.max(1);
    let allow_upscale = options.allow_upscale;

    match options.resize_kind {
        ResizeKind::Max => {
            let scale = f64::min(
                f64::from(value) / f64::from(width),
                f64::from(value) / f64::from(height),
            );
            let scale = if allow_upscale { scale } else { scale.min(1.0) };
            scaled_size(width, height, scale)
        }
        ResizeKind::Width => {
            let target_width = if !allow_upscale && value > width {
                width
            } else {
                value
            };
            (
                target_width,
                scale_dimension(height, f64::from(target_width) / f64::from(width.max(1))),
            )
        }
        ResizeKind::Height => {
            let target_height = if !allow_upscale && value > height {
                height
            } else {
                value
            };
            (
                scale_dimension(width, f64::from(target_height) / f64::from(height.max(1))),
                target_height,
            )
        }
        ResizeKind::Percent => {
            let mut scale = f64::from(value) / 100.0;
            if !allow_upscale {
                scale = scale.min(1.0);
            }
            scaled_size(width, height, scale)
        }
    }
}

fn fit_dimensions(
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
    options: &ConvertOptions,
) -> (u32, u32) {
    let target_width = target_width.max(1);
    let target_height = target_height.max(1);
    let allow_upscale = options.allow_upscale;

    match options.fit_mode {
        FitMode::Fill => {
            let mut scale_x = f64::from(target_width) / f64::from(width);
            let mut scale_y = f64::from(target_height) / f64::from(height);
            if !allow_upscale {
                scale_x = scale_x.min(1.0);
                scale_y = scale_y.min(1.0);
            }
            (
                scale_dimension(width, scale_x),
                scale_dimension(height, scale_y),
            )
        }
        FitMode::Inside => {
            let scale = f64::min(
                f64::from(target_width) / f64::from(width),
                f64::from(target_height) / f64::from(height),
            );
            let scale = if allow_upscale { scale } else { scale.min(1.0) };
            scaled_size(width, height, scale)
        }
        FitMode::Outside => {
            let mut scale = f64::max(
                f64::from(target_width) / f64::from(width),
                f64::from(target_height) / f64::from(height),
            );
            if !allow_upscale {
                scale = scale.min(1.0);
            }
            let (scaled_width, scaled_height) = scaled_size(width, height, scale);
            (
                target_width.min(scaled_width),
                target_height.min(scaled_height),
            )
        }
        FitMode::Cover => {
            let (crop_width, crop_height) =
                cover_crop_size(width, height, target_width, target_height);
            let mut scale_x = f64::from(target_width) / f64::from(crop_width.max(1));
            let mut scale_y = f64::from(target_height) / f64::from(crop_height.max(1));
            if !allow_upscale {
                scale_x = scale_x.min(1.0);
                scale_y = scale_y.min(1.0);
            }
            scaled_size(crop_width, crop_height, scale_x.min(scale_y))
        }
        // Contain pads onto a canvas, so the frame is always honoured.
        FitMode::Contain => (target_width, target_height),
    }
}

/// The `Resize` size mode: width / height / max-side / percent.
fn resize_mode(source: &RgbaImage, options: &ConvertOptions) -> RgbaImage {
    let width = source.width();
    let height = source.height();
    let value = options.resize_value.max(1);
    let allow_upscale = options.allow_upscale;

    match options.resize_kind {
        ResizeKind::Max => {
            // Square bounding box, aspect preserved, never stretched.
            let scale = f64::min(
                f64::from(value) / f64::from(width),
                f64::from(value) / f64::from(height),
            );
            let scale = if allow_upscale { scale } else { scale.min(1.0) };
            let (target_width, target_height) = scaled_size(width, height, scale);
            resize_to(source, target_width, target_height)
        }
        ResizeKind::Width => {
            let target_width = if !allow_upscale && value > width {
                width
            } else {
                value
            };
            let target_height =
                scale_dimension(height, f64::from(target_width) / f64::from(width.max(1)));
            resize_to(source, target_width, target_height)
        }
        ResizeKind::Height => {
            let target_height = if !allow_upscale && value > height {
                height
            } else {
                value
            };
            let target_width =
                scale_dimension(width, f64::from(target_height) / f64::from(height.max(1)));
            resize_to(source, target_width, target_height)
        }
        ResizeKind::Percent => {
            let mut scale = f64::from(value) / 100.0;
            if !allow_upscale {
                scale = scale.min(1.0);
            }
            let (target_width, target_height) = scaled_size(width, height, scale);
            resize_to(source, target_width, target_height)
        }
    }
}

/// The object-fit modes, used by the `Aspect` and `Custom` size modes.
fn fit(
    source: &RgbaImage,
    target_width: u32,
    target_height: u32,
    options: &ConvertOptions,
) -> RgbaImage {
    let width = source.width();
    let height = source.height();
    let target_width = target_width.max(1);
    let target_height = target_height.max(1);
    let allow_upscale = options.allow_upscale;

    match options.fit_mode {
        FitMode::Fill => {
            // Stretch to the exact target, never enlarging when upscale is off.
            let mut scale_x = f64::from(target_width) / f64::from(width);
            let mut scale_y = f64::from(target_height) / f64::from(height);
            if !allow_upscale {
                scale_x = scale_x.min(1.0);
                scale_y = scale_y.min(1.0);
            }
            resize_to(
                source,
                scale_dimension(width, scale_x),
                scale_dimension(height, scale_y),
            )
        }
        FitMode::Inside => {
            let scale = f64::min(
                f64::from(target_width) / f64::from(width),
                f64::from(target_height) / f64::from(height),
            );
            let scale = if allow_upscale { scale } else { scale.min(1.0) };
            let (new_width, new_height) = scaled_size(width, height, scale);
            resize_to(source, new_width, new_height)
        }
        FitMode::Outside => {
            let mut scale = f64::max(
                f64::from(target_width) / f64::from(width),
                f64::from(target_height) / f64::from(height),
            );
            if !allow_upscale {
                scale = scale.min(1.0);
            }
            let (scaled_width, scaled_height) = scaled_size(width, height, scale);
            let resized = resize_to(source, scaled_width, scaled_height);
            // Trim back down to the requested frame, honouring the focus point.
            let crop_width = target_width.min(resized.width());
            let crop_height = target_height.min(resized.height());
            crop_focus(&resized, crop_width, crop_height, options.crop_position)
        }
        FitMode::Cover => {
            // Crop the source to the target aspect ratio first, then scale.
            // Doing it this way keeps the smart-crop analysis in source pixels.
            let (crop_width, crop_height) =
                cover_crop_size(width, height, target_width, target_height);
            let cropped = crop_focus(source, crop_width, crop_height, options.crop_position);

            let mut scale_x = f64::from(target_width) / f64::from(cropped.width());
            let mut scale_y = f64::from(target_height) / f64::from(cropped.height());
            if !allow_upscale {
                scale_x = scale_x.min(1.0);
                scale_y = scale_y.min(1.0);
            }
            // One shared factor so the aspect ratio survives the clamp.
            let scale = scale_x.min(scale_y);
            let (new_width, new_height) = scaled_size(cropped.width(), cropped.height(), scale);
            resize_to(&cropped, new_width, new_height)
        }
        FitMode::Contain => {
            let scale = f64::min(
                f64::from(target_width) / f64::from(width),
                f64::from(target_height) / f64::from(height),
            );
            let scale = if allow_upscale { scale } else { scale.min(1.0) };
            let (new_width, new_height) = scaled_size(width, height, scale);
            let resized = resize_to(source, new_width, new_height);

            // Padding adds canvas, so the target frame is always honoured.
            let mut canvas = RgbaImage::from_pixel(
                target_width,
                target_height,
                parse_background(&options.background),
            );
            let offset_x = i64::from((target_width.saturating_sub(new_width)) / 2);
            let offset_y = i64::from((target_height.saturating_sub(new_height)) / 2);
            imageops::overlay(&mut canvas, &resized, offset_x, offset_y);
            canvas
        }
    }
}

/// Largest source rectangle with the target aspect ratio, centred on the source.
fn cover_crop_size(width: u32, height: u32, target_width: u32, target_height: u32) -> (u32, u32) {
    let target_ratio = f64::from(target_width.max(1)) / f64::from(target_height.max(1));
    let source_ratio = f64::from(width.max(1)) / f64::from(height.max(1));

    let (crop_width, crop_height) = if source_ratio > target_ratio {
        (
            (f64::from(height) * target_ratio).round() as i64,
            i64::from(height),
        )
    } else {
        (
            i64::from(width),
            (f64::from(width) / target_ratio).round() as i64,
        )
    };

    (
        crop_width.clamp(1, i64::from(width.max(1))) as u32,
        crop_height.clamp(1, i64::from(height.max(1))) as u32,
    )
}

fn crop_focus(image: &RgbaImage, width: u32, height: u32, position: CropPosition) -> RgbaImage {
    let width = width.clamp(1, image.width().max(1));
    let height = height.clamp(1, image.height().max(1));

    if width == image.width() && height == image.height() {
        return image.clone();
    }

    let (x, y) = focus_origin(image, width, height, position);
    imageops::crop_imm(image, x, y, width, height).to_image()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ConvertOptions;

    fn sample(width: u32, height: u32) -> RgbaImage {
        // Distinct pixel values so scaling mistakes are visible.
        let mut image = RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]);
        }
        image
    }

    fn options_for(size_mode: SizeMode, fit_mode: FitMode) -> ConvertOptions {
        ConvertOptions {
            size_mode,
            fit_mode,
            ..ConvertOptions::default()
        }
    }

    #[test]
    fn original_mode_is_a_passthrough() {
        let image = sample(40, 20);
        let result = apply(&image, &ConvertOptions::default());
        assert_eq!((result.width(), result.height()), (40, 20));
    }

    #[test]
    fn max_side_scales_down_and_preserves_aspect() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Resize,
            resize_kind: ResizeKind::Max,
            resize_value: 100,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 50));
    }

    #[test]
    fn max_side_never_enlarges_unless_allowed() {
        let image = sample(50, 50);
        let base = ConvertOptions {
            size_mode: SizeMode::Resize,
            resize_kind: ResizeKind::Max,
            resize_value: 500,
            ..ConvertOptions::default()
        };

        let clamped = apply(&image, &base);
        assert_eq!((clamped.width(), clamped.height()), (50, 50));

        let enlarged = apply(
            &image,
            &ConvertOptions {
                allow_upscale: true,
                ..base
            },
        );
        assert_eq!((enlarged.width(), enlarged.height()), (500, 500));
    }

    #[test]
    fn width_only_keeps_the_aspect_ratio() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Resize,
            resize_kind: ResizeKind::Width,
            resize_value: 100,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 50));
    }

    #[test]
    fn height_only_keeps_the_aspect_ratio() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Resize,
            resize_kind: ResizeKind::Height,
            resize_value: 50,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 50));
    }

    #[test]
    fn percent_scales_both_dimensions() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Resize,
            resize_kind: ResizeKind::Percent,
            resize_value: 25,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 50));
    }

    #[test]
    fn custom_fill_always_hits_the_exact_target() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Custom,
            custom_width: 120,
            custom_height: 120,
            fit_mode: FitMode::Fill,
            allow_upscale: true,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (120, 120));
    }

    #[test]
    fn custom_cover_crops_to_the_target_without_distortion() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Custom,
            custom_width: 100,
            custom_height: 100,
            fit_mode: FitMode::Cover,
            allow_upscale: true,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 100));
    }

    #[test]
    fn cover_without_upscale_stays_smaller_than_the_target() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Custom,
            custom_width: 400,
            custom_height: 400,
            fit_mode: FitMode::Cover,
            allow_upscale: false,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        // Crops to 1:1 first (200x200), then refuses to enlarge.
        assert_eq!((result.width(), result.height()), (200, 200));
    }

    #[test]
    fn custom_inside_fits_within_the_frame() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Custom,
            custom_width: 100,
            custom_height: 100,
            fit_mode: FitMode::Inside,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 50));
    }

    #[test]
    fn custom_contain_pads_to_the_exact_frame() {
        let image = sample(400, 200);
        let options = ConvertOptions {
            size_mode: SizeMode::Custom,
            custom_width: 100,
            custom_height: 100,
            fit_mode: FitMode::Contain,
            background: "#000000".into(),
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert_eq!((result.width(), result.height()), (100, 100));
        // Top rows are the background colour because the image is letterboxed.
        assert_eq!(result.get_pixel(50, 0), &Rgba([0, 0, 0, 255]));
    }

    #[test]
    fn aspect_ratio_mode_hits_the_requested_ratio() {
        let image = sample(400, 300);
        let options = options_for(SizeMode::Aspect, FitMode::Cover);
        let result = apply(
            &image,
            &ConvertOptions {
                aspect_preset: "16:9".into(),
                allow_upscale: true,
                ..options
            },
        );
        // aspectTarget() keeps the full width and crops the height to 400/16*9.
        assert_eq!((result.width(), result.height()), (400, 225));
    }

    #[test]
    fn background_parsing_handles_junk_and_case() {
        // `Rgba` indexes per channel, so whole-pixel comparisons are the clear
        // way to assert on it.
        let fallback = Rgba([
            DEFAULT_BACKGROUND[0],
            DEFAULT_BACKGROUND[1],
            DEFAULT_BACKGROUND[2],
            255,
        ]);

        assert_eq!(parse_background("#fffaf2"), Rgba([255, 250, 242, 255]));
        assert_eq!(parse_background("#FFFaf2"), Rgba([255, 250, 242, 255]));
        assert_eq!(parse_background("not-a-colour"), fallback);
        assert_eq!(parse_background(""), fallback);
    }

    #[test]
    fn single_pixel_source_never_produces_zero_dimensions() {
        let image = sample(1, 1);
        let options = ConvertOptions {
            size_mode: SizeMode::Aspect,
            aspect_preset: "16:9".into(),
            fit_mode: FitMode::Cover,
            allow_upscale: true,
            ..ConvertOptions::default()
        };
        let result = apply(&image, &options);
        assert!(result.width() >= 1 && result.height() >= 1);
    }

    /// The estimate engine reports sizes before a single pixel is touched, and
    /// the UI prints them next to every file. If this ever fails, the app is
    /// promising one size and writing another.
    #[test]
    fn reported_dimensions_match_the_pixels() {
        let sources = [(400, 300), (2400, 1600), (37, 91), (1, 1)];
        let mut matrix = vec![ConvertOptions::default()];

        for kind in [
            ResizeKind::Max,
            ResizeKind::Width,
            ResizeKind::Height,
            ResizeKind::Percent,
        ] {
            for value in [50, 120, 1600] {
                matrix.push(ConvertOptions {
                    size_mode: SizeMode::Resize,
                    resize_kind: kind,
                    resize_value: value,
                    ..ConvertOptions::default()
                });
            }
        }

        for fit_mode in [
            FitMode::Fill,
            FitMode::Inside,
            FitMode::Outside,
            FitMode::Cover,
            FitMode::Contain,
        ] {
            for allow_upscale in [false, true] {
                matrix.push(ConvertOptions {
                    size_mode: SizeMode::Custom,
                    custom_width: 120,
                    custom_height: 90,
                    fit_mode,
                    allow_upscale,
                    ..ConvertOptions::default()
                });
                matrix.push(ConvertOptions {
                    size_mode: SizeMode::Aspect,
                    aspect_preset: "16:9".into(),
                    fit_mode,
                    allow_upscale,
                    ..ConvertOptions::default()
                });
            }
        }

        for (width, height) in sources {
            let image = sample(width, height);
            for options in &matrix {
                let actual = apply(&image, options);
                assert_eq!(
                    output_dimensions(width, height, options),
                    (actual.width(), actual.height()),
                    "size_mode={:?} resize_kind={:?} fit={:?} upscale={} source={}x{}",
                    options.size_mode,
                    options.resize_kind,
                    options.fit_mode,
                    options.allow_upscale,
                    width,
                    height,
                );
            }
        }
    }
}
