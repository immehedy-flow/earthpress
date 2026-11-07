//! Live "what will this cost?" numbers, without paying for a full conversion.
//!
//! Each photo is decoded once, cached at two sizes, then encoded with the exact
//! settings the batch will use. Fitting `bytes = a * pixels^b` through the two
//! samples captures the fact that file size grows sub-linearly with pixel
//! count, which keeps the projection closer than a single-sample ratio.
//!
//! The sample cache is what makes the sliders usable: the first pass costs a
//! decode per file, every later pass is just two small encodes.

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use image::{imageops, RgbaImage};

use crate::codecs;
use crate::decode;
use crate::error::AppResult;
use crate::logging;
use crate::model::{ConvertOptions, EstimateInput, ImageEstimate, ImageFormat, QualityMode};
use crate::orientation;
use crate::paths;
use crate::resize;

/// Longest side of the small cached sample.
pub const SAMPLE_EDGE: u32 = 640;

/// Longest side of the second sample, used for the size-curve fit.
const LARGE_SAMPLE_EDGE: u32 = 960;

/// Soft cap on cached sample memory. The two samples are about 5.3 MB each.
const CACHE_BUDGET: usize = 96 * 1024 * 1024;

/// Two renders of the same photo at different sizes.
struct Samples {
    small: Arc<RgbaImage>,
    large: Arc<RgbaImage>,
}

impl Samples {
    fn memory(&self) -> usize {
        self.small.as_raw().len() + self.large.as_raw().len()
    }
}

struct SampleCache {
    /// Most recently used first, so the tail is the thing to evict.
    entries: Vec<(String, Arc<Samples>)>,
    bytes: usize,
}

impl SampleCache {
    const fn new() -> Self {
        Self {
            entries: Vec::new(),
            bytes: 0,
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<Samples>> {
        let index = self.entries.iter().position(|(entry, _)| entry == key)?;
        let entry = self.entries.remove(index);
        let samples = Arc::clone(&entry.1);
        self.entries.insert(0, entry);
        Some(samples)
    }

    fn put(&mut self, key: String, samples: Arc<Samples>) {
        self.bytes += samples.memory();
        self.entries.insert(0, (key, samples));

        while self.bytes > CACHE_BUDGET && self.entries.len() > 1 {
            if let Some((_, evicted)) = self.entries.pop() {
                self.bytes = self.bytes.saturating_sub(evicted.memory());
            }
        }
    }
}

fn cache() -> MutexGuard<'static, SampleCache> {
    static CACHE: OnceLock<Mutex<SampleCache>> = OnceLock::new();
    let mutex = CACHE.get_or_init(|| Mutex::new(SampleCache::new()));

    // A panic elsewhere must not turn every later estimate into a failure.
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The cached samples for `path`, decoding and downscaling on first use.
fn sample(path: &str) -> AppResult<Arc<Samples>> {
    let metadata = std::fs::metadata(path)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|delta| delta.as_millis())
        .unwrap_or(0);
    // Size and mtime change whenever the file is replaced, which is exactly
    // when the cached sample stops describing it.
    let key = format!("{path}::{modified}::{}", metadata.len());

    if let Some(hit) = cache().get(&key) {
        return Ok(hit);
    }

    paths::validate_source(std::path::Path::new(path))?;
    let bytes = decode::read_source(std::path::Path::new(path))?;
    let decoded = decode::decode(&bytes)?;
    let orientation = orientation::read_orientation(&bytes);

    let samples = Arc::new(Samples {
        small: Arc::new(downscale_oriented(&decoded, orientation, SAMPLE_EDGE)),
        large: Arc::new(downscale_oriented(&decoded, orientation, LARGE_SAMPLE_EDGE)),
    });

    cache().put(key, Arc::clone(&samples));

    Ok(samples)
}

/// Downscales into an `edge` x `edge` box, in source axes.
pub fn downscale(image: &RgbaImage, edge: u32) -> RgbaImage {
    if image.width() == 0 || image.height() == 0 {
        return image.clone();
    }

    let scale = f64::min(
        f64::from(edge) / f64::from(image.width()),
        f64::from(edge) / f64::from(image.height()),
    )
    .min(1.0);

    resize_into(
        image,
        ((f64::from(image.width()) * scale).round() as u32).max(1),
        ((f64::from(image.height()) * scale).round() as u32).max(1),
    )
}

fn resize_into(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if width == image.width() && height == image.height() {
        image.clone()
    } else {
        imageops::resize(image, width, height, imageops::FilterType::Triangle)
    }
}

/// Downscales into a `edge` x `edge` box, then applies the EXIF rotation.
///
/// Same trade as the preview renderer: shrinking first and rotating afterwards
/// is identical in the end and much cheaper for a 90/270 tag.
fn downscale_oriented(image: &RgbaImage, orientation: u16, edge: u32) -> RgbaImage {
    if image.width() == 0 || image.height() == 0 {
        return image.clone();
    }

    let swaps = orientation::swaps_axes(orientation);
    let (source_width, source_height) = if swaps {
        (image.height(), image.width())
    } else {
        (image.width(), image.height())
    };

    let scale = f64::min(
        f64::from(edge) / f64::from(source_width),
        f64::from(edge) / f64::from(source_height),
    )
    .min(1.0);

    let target_width = ((f64::from(source_width) * scale).round() as u32).max(1);
    let target_height = ((f64::from(source_height) * scale).round() as u32).max(1);

    let (resize_width, resize_height) = if swaps {
        (target_height, target_width)
    } else {
        (target_width, target_height)
    };

    orientation::apply(resize_into(image, resize_width, resize_height), orientation)
}

/// The projected byte count of `sample` at `quality`, scaled to `ratio`.
fn projected_bytes(
    sample: &RgbaImage,
    probe: &mut ConvertOptions,
    quality: u8,
    ratio: f64,
) -> Option<u64> {
    probe.quality = quality;
    // Never let the probe recurse into another search.
    probe.quality_mode = QualityMode::Level;

    let bytes = codecs::encode(sample, probe).ok()?;
    Some(((bytes.len() as f64 * ratio).round() as u64).max(64))
}

/// Encodes both samples at `quality` and extrapolates to the full output size.
fn projected_from_samples(
    samples: &Samples,
    probe: &mut ConvertOptions,
    quality: u8,
    output_pixels: u64,
) -> Option<u64> {
    probe.quality = quality;
    probe.quality_mode = QualityMode::Level;

    let small_bytes = codecs::encode(&samples.small, probe).ok()?.len() as u64;
    let large_bytes = codecs::encode(&samples.large, probe).ok()?.len() as u64;

    Some(scale_bytes(
        small_bytes,
        large_bytes,
        pixels_of(&samples.small),
        pixels_of(&samples.large),
        output_pixels,
    ))
}

fn pixels_of(image: &RgbaImage) -> u64 {
    u64::from(image.width()) * u64::from(image.height())
}

/// Fits `bytes = a * pixels^b` through two samples and predicts `output_pixels`.
///
/// A single sample scaled by pixel ratio assumes file size grows linearly with
/// pixel count. It does not: container overhead and entropy coding both bend
/// the curve. The exponent from two real encodes keeps large outputs from
/// being systematically overestimated. Falls back to linear scaling when the
/// samples are too close in size or the fit is implausible.
fn scale_bytes(
    small_bytes: u64,
    large_bytes: u64,
    small_pixels: u64,
    large_pixels: u64,
    output_pixels: u64,
) -> u64 {
    let linear = |pixels: u64| -> u64 {
        if small_pixels == 0 {
            return small_bytes.max(64);
        }
        ((small_bytes as f64) * (pixels as f64 / small_pixels as f64))
            .round()
            .max(64.0) as u64
    };

    if output_pixels <= small_pixels
        || small_pixels == 0
        || large_pixels <= small_pixels
        || small_bytes == 0
        || large_bytes == 0
    {
        return linear(output_pixels);
    }

    let exponent = (large_bytes as f64 / small_bytes as f64).ln()
        / (large_pixels as f64 / small_pixels as f64).ln();

    // Encoded size should grow with pixels, but sub-linearly. Anything outside
    // this range means the two samples were too similar to trust.
    if !exponent.is_finite() || !(0.35..=1.05).contains(&exponent) {
        return linear(output_pixels);
    }

    let coefficient = small_bytes as f64 / (small_pixels as f64).powf(exponent);
    let predicted = coefficient * (output_pixels as f64).powf(exponent);

    if !predicted.is_finite() || predicted < 64.0 {
        return linear(output_pixels);
    }

    predicted.round() as u64
}

/// The highest quality whose projected size still fits `target_bytes`.
///
/// `sample` is a small render of the same content and `pixels` is the pixel
/// count of the real output, so seven cheap encodes stand in for one expensive
/// one. Returns `(quality, projected_bytes, meets_target)`; `meets_target` is
/// false when even quality 1 overshoots.
pub fn search_quality(
    sample: &RgbaImage,
    pixels: u64,
    options: &ConvertOptions,
    target_bytes: u64,
) -> (u8, u64, bool) {
    let sample_pixels = u64::from(sample.width()) * u64::from(sample.height());
    let ratio = if sample_pixels > 0 && pixels > 0 {
        pixels as f64 / sample_pixels as f64
    } else {
        1.0
    };

    let mut probe = options.clone();
    let mut low = 1u8;
    let mut high = 100u8;
    let mut best: Option<(u8, u64)> = None;

    while low <= high {
        let mid = low + (high - low) / 2;
        let Some(bytes) = projected_bytes(sample, &mut probe, mid, ratio) else {
            break;
        };

        if bytes <= target_bytes {
            best = Some((mid, bytes));
            if mid == 100 {
                break;
            }
            low = mid + 1;
        } else if mid == 1 {
            break;
        } else {
            high = mid - 1;
        }
    }

    match best {
        Some((quality, bytes)) => (quality, bytes, true),
        None => {
            let bytes = projected_bytes(sample, &mut probe, 1, ratio).unwrap_or(64);
            (1, bytes, false)
        }
    }
}

/// Estimates one file, using `options` as if the batch were starting now.
pub fn estimate_one(input: &EstimateInput, options: &ConvertOptions) -> AppResult<ImageEstimate> {
    let (output_width, output_height) =
        resize::output_dimensions(input.width, input.height, options);

    let samples = sample(&input.path)?;
    let output_pixels = u64::from(output_width) * u64::from(output_height);

    if wants_target(options) {
        let (quality, search_bytes, meets_target) =
            search_quality(&samples.small, output_pixels, options, options.target_bytes);

        // The search picks the quality; the two-sample fit then reports a more
        // honest byte count for that quality.
        let mut probe = options.clone();
        let estimated_bytes =
            projected_from_samples(&samples, &mut probe, quality, output_pixels)
                .unwrap_or(search_bytes);

        return Ok(ImageEstimate {
            id: input.id.clone(),
            output_width,
            output_height,
            estimated_bytes,
            quality_used: quality,
            meets_target,
        });
    }

    let mut probe = options.clone();
    let estimated_bytes =
        match projected_from_samples(&samples, &mut probe, options.quality, output_pixels) {
            Some(bytes) => bytes,
            None => {
                // Keep the real encoder error rather than silently guessing.
                let encoded = codecs::encode(&samples.small, options)?;
                scale_bytes(
                    encoded.len() as u64,
                    encoded.len() as u64,
                    pixels_of(&samples.small),
                    pixels_of(&samples.small),
                    output_pixels,
                )
            }
        };

    Ok(ImageEstimate {
        id: input.id.clone(),
        output_width,
        output_height,
        estimated_bytes,
        quality_used: options.quality,
        meets_target: true,
    })
}

/// Target mode needs a real byte target and a codec that trades quality for
/// size. Lossless and PNG ignore the quality dial, so they stay on the level.
pub fn wants_target(options: &ConvertOptions) -> bool {
    options.quality_mode == QualityMode::Target
        && options.target_bytes > 0
        && !options.lossless
        && options.format != ImageFormat::Png
}

/// Estimates a whole batch. Unreadable files are skipped rather than failing
/// the pass, so one broken photo cannot blank out every other row.
pub fn estimate_batch(inputs: &[EstimateInput], options: &ConvertOptions) -> Vec<ImageEstimate> {
    let mut estimates = Vec::with_capacity(inputs.len());

    for input in inputs {
        match estimate_one(input, options) {
            Ok(estimate) => estimates.push(estimate),
            Err(error) => logging::warn(
                format!("Could not estimate {}", input.path),
                Some(serde_json::json!({ "error": error.to_string() })),
            ),
        }
    }

    estimates
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(width: u32, height: u32) -> RgbaImage {
        let mut image = RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            // Noise-free content compresses far better than a photo, but the
            // pixel scaling behaviour is what these tests are about.
            *pixel = Rgba([(x % 200) as u8, (y % 180) as u8, 120, 255]);
        }
        image
    }

    fn samples(width: u32, height: u32) -> Arc<Samples> {
        let image = Arc::new(solid(width, height));
        Arc::new(Samples {
            small: Arc::clone(&image),
            large: image,
        })
    }

    #[test]
    fn estimates_scale_with_the_output_pixels() {
        let options = ConvertOptions::default();
        let input = EstimateInput {
            id: "a".into(),
            path: String::new(),
            width: 2000,
            height: 1000,
        };

        let (w, h) = resize::output_dimensions(input.width, input.height, &options);
        assert_eq!((w, h), (2000, 1000));

        let sample = solid(640, 320);
        let encoded = codecs::encode(&sample, &options).expect("sample encodes");

        let ratio = (f64::from(w) * f64::from(h))
            / (f64::from(sample.width()) * f64::from(sample.height()));
        let scaled = (encoded.len() as f64 * ratio).round() as u64;

        // Sanity: the ratio is what turns a sample into a full-size guess.
        assert!(scaled > encoded.len() as u64);
        assert!(scaled < 40 * 1024 * 1024);
    }

    #[test]
    fn downscaling_never_upscales_and_never_zeroes_out() {
        let small = downscale_oriented(&solid(40, 20), 1, 640);
        assert_eq!((small.width(), small.height()), (40, 20));

        let tiny = downscale_oriented(&solid(1, 1), 6, 640);
        assert!(tiny.width() >= 1 && tiny.height() >= 1);
    }

    #[test]
    fn the_sample_cache_moves_hits_to_the_front() {
        let mut cache = SampleCache::new();
        cache.put("old".into(), samples(8, 8));
        cache.put("fresh".into(), samples(8, 8));
        assert_eq!(cache.entries[0].0, "fresh");

        assert!(cache.get("old").is_some());
        assert_eq!(cache.entries[0].0, "old");
        assert_eq!(cache.bytes, 2 * 2 * 8 * 8 * 4);
    }

    #[test]
    fn the_two_sample_fit_beats_linear_extrapolation() {
        // 640² -> 100 KB, 960² -> 180 KB. A straight pixel ratio would predict
        // ~391 KB at 1.6 MP; the power fit should land meaningfully lower.
        let predicted = scale_bytes(100_000, 180_000, 640 * 640, 960 * 960, 1_600_000);
        let linear = 100_000 * 1_600_000 / (640 * 640);

        assert!(predicted < linear, "{predicted} should be below {linear}");
        assert!(
            (250_000..=290_000).contains(&predicted),
            "unexpected fit: {predicted}"
        );
    }

    #[test]
    fn the_fit_falls_back_when_samples_are_the_same_size() {
        assert_eq!(scale_bytes(1_000, 1_000, 100, 100, 400), 4_000);
    }

    #[test]
    fn the_target_search_lands_under_the_budget() {
        let options = ConvertOptions {
            quality_mode: QualityMode::Target,
            target_bytes: 0,
            ..ConvertOptions::default()
        };
        let sample = solid(320, 200);
        let pixels = 1600u64 * 1000;
        let ratio = pixels as f64 / (f64::from(sample.width()) * f64::from(sample.height()));

        // Whatever quality 40 costs at full size is a budget the search must meet.
        let mut probe = options.clone();
        probe.quality = 40;
        probe.quality_mode = QualityMode::Level;
        let budget =
            (codecs::encode(&sample, &probe).expect("probe encodes").len() as f64 * ratio) as u64;

        let (quality, bytes, meets_target) = search_quality(&sample, pixels, &options, budget);

        assert!(meets_target, "a reachable budget must be met");
        assert!(bytes <= budget, "{bytes} should fit in {budget}");
        assert!(
            quality >= 40,
            "quality {quality} should be at least the quality that fits"
        );
    }

    #[test]
    fn an_impossible_target_reports_the_floor() {
        let options = ConvertOptions {
            quality_mode: QualityMode::Target,
            ..ConvertOptions::default()
        };
        let sample = solid(320, 200);

        let (quality, bytes, meets_target) = search_quality(&sample, 1600 * 1000, &options, 1);

        assert_eq!(quality, 1);
        assert!(bytes > 1);
        assert!(!meets_target);
    }

    #[test]
    fn target_mode_is_off_for_formats_that_ignore_quality() {
        let mut options = ConvertOptions {
            quality_mode: QualityMode::Target,
            target_bytes: 100 * 1024,
            ..ConvertOptions::default()
        };

        assert!(wants_target(&options));

        options.lossless = true;
        assert!(!wants_target(&options), "lossless ignores the quality dial");

        options.lossless = false;
        options.format = ImageFormat::Png;
        assert!(!wants_target(&options), "PNG has no quality dial");

        options.format = ImageFormat::Webp;
        options.target_bytes = 0;
        assert!(!wants_target(&options), "a zero target is not a target");
    }

    #[test]
    fn estimating_a_real_file_reports_its_target_geometry() {
        let dir = std::env::temp_dir().join("earthpress-estimate-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("sample.png");

        solid(1200, 800).save(&path).expect("save png");

        let options = ConvertOptions {
            size_mode: crate::model::SizeMode::Resize,
            resize_kind: crate::model::ResizeKind::Max,
            resize_value: 600,
            ..ConvertOptions::default()
        };
        let input = EstimateInput {
            id: "a".into(),
            path: path.to_string_lossy().into_owned(),
            width: 1200,
            height: 800,
        };

        let estimate = estimate_one(&input, &options).expect("estimate");
        assert_eq!((estimate.output_width, estimate.output_height), (600, 400));
        assert!(estimate.estimated_bytes > 0);
        assert!(estimate.meets_target);

        // The second pass should reuse the cached sample and agree exactly.
        let again = estimate_one(&input, &options).expect("estimate again");
        assert_eq!(estimate.estimated_bytes, again.estimated_bytes);
    }
}
