//! Focus-point selection for cropping.
//!
//! `center` / `top` / `right` / `bottom` / `left` are trivial. The two smart
//! modes downscale the source to a small analysis grid, score every cell, then
//! slide a crop-sized window over that grid and keep the highest scoring
//! position:
//!
//! * [`CropPosition::Entropy`] scores cells by the Shannon entropy of their
//!   luma histogram, which is how libvips' `interesting=ENTROPY` behaves.
//! * [`CropPosition::Attention`] scores cells by edge energy, local contrast,
//!   colourfulness and a mild centre bias. This is an approximation of libvips'
//!   saliency model, not a bit-for-bit port of it.

use crate::model::CropPosition;
use image::{imageops, RgbaImage};

/// Width of the analysis grid. Small enough to stay cheap, large enough to
/// keep the window search meaningful.
const ANALYSIS_WIDTH: u32 = 64;

/// Histogram resolution for the entropy score.
const HIST_BINS: usize = 16;

/// Half-width of the neighbourhood sampled per entropy cell.
const NEIGHBOURHOOD: i64 = 2;

fn analysis_size(width: u32, height: u32) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let analysis_width = ANALYSIS_WIDTH.min(width);
    let analysis_height =
        ((f64::from(height) * f64::from(analysis_width) / f64::from(width)).round() as u32).max(1);
    (analysis_width, analysis_height)
}

fn luma(pixel: &image::Rgba<u8>) -> f32 {
    0.299 * f32::from(pixel[0]) + 0.587 * f32::from(pixel[1]) + 0.114 * f32::from(pixel[2])
}

struct ScoreGrid {
    scores: Vec<f32>,
    width: u32,
    height: u32,
}

impl ScoreGrid {
    fn at(&self, x: u32, y: u32) -> f32 {
        self.scores[(y * self.width + x) as usize]
    }
}

/// Picks the top-left crop offset for `position`.
pub fn focus_origin(
    source: &RgbaImage,
    crop_width: u32,
    crop_height: u32,
    position: CropPosition,
) -> (u32, u32) {
    let (source_width, source_height) = (source.width(), source.height());
    let crop_width = crop_width.clamp(1, source_width.max(1));
    let crop_height = crop_height.clamp(1, source_height.max(1));

    let max_x = source_width.saturating_sub(crop_width);
    let max_y = source_height.saturating_sub(crop_height);

    match position {
        CropPosition::Center => (max_x / 2, max_y / 2),
        CropPosition::Top => (max_x / 2, 0),
        CropPosition::Bottom => (max_x / 2, max_y),
        CropPosition::Left => (0, max_y / 2),
        CropPosition::Right => (max_x, max_y / 2),
        CropPosition::Entropy => smart_origin(source, crop_width, crop_height, true),
        CropPosition::Attention => smart_origin(source, crop_width, crop_height, false),
    }
}

fn smart_origin(
    source: &RgbaImage,
    crop_width: u32,
    crop_height: u32,
    entropy: bool,
) -> (u32, u32) {
    let (source_width, source_height) = (source.width(), source.height());

    let centred = (
        source_width.saturating_sub(crop_width) / 2,
        source_height.saturating_sub(crop_height) / 2,
    );

    // Nothing to slide: the crop already covers the whole image.
    if crop_width >= source_width && crop_height >= source_height {
        return (0, 0);
    }

    let (grid_width, grid_height) = analysis_size(source_width, source_height);
    let analysis: RgbaImage = imageops::resize(
        source,
        grid_width,
        grid_height,
        imageops::FilterType::Triangle,
    );

    let grid = if entropy {
        entropy_grid(&analysis)
    } else {
        attention_grid(&analysis)
    };

    let scale_x = f64::from(grid_width) / f64::from(source_width);
    let scale_y = f64::from(grid_height) / f64::from(source_height);

    let window_width = ((f64::from(crop_width) * scale_x).round() as u32).clamp(1, grid_width);
    let window_height = ((f64::from(crop_height) * scale_y).round() as u32).clamp(1, grid_height);

    let Some((best_x, best_y)) = best_window(&grid, window_width, window_height) else {
        return centred;
    };

    // Translate the winning grid cell back into source pixels, then clamp so
    // the crop always stays inside the image.
    let max_x = source_width.saturating_sub(crop_width) as i64;
    let max_y = source_height.saturating_sub(crop_height) as i64;
    let origin_x = ((f64::from(best_x) / scale_x).round() as i64).clamp(0, max_x);
    let origin_y = ((f64::from(best_y) / scale_y).round() as i64).clamp(0, max_y);

    (origin_x as u32, origin_y as u32)
}

/// Shannon entropy of the luma histogram around every cell, normalised to 0..1.
fn entropy_grid(image: &RgbaImage) -> ScoreGrid {
    let width = image.width();
    let height = image.height();
    let mut scores = vec![0.0f32; (width * height) as usize];
    let normaliser = 1.0 / (HIST_BINS as f32).log2();

    for y in 0..height {
        for x in 0..width {
            let mut histogram = [0f32; HIST_BINS];
            let mut samples = 0f32;

            let y_start = (i64::from(y) - NEIGHBOURHOOD).max(0) as u32;
            let y_end = (i64::from(y) + NEIGHBOURHOOD).min(i64::from(height) - 1) as u32;
            let x_start = (i64::from(x) - NEIGHBOURHOOD).max(0) as u32;
            let x_end = (i64::from(x) + NEIGHBOURHOOD).min(i64::from(width) - 1) as u32;

            for sample_y in y_start..=y_end {
                for sample_x in x_start..=x_end {
                    let value = luma(image.get_pixel(sample_x, sample_y));
                    let luma_byte = value.clamp(0.0, 255.0) as u32;
                    let bin = ((luma_byte * HIST_BINS as u32) / 256).min(HIST_BINS as u32 - 1);
                    histogram[bin as usize] += 1.0;
                    samples += 1.0;
                }
            }

            let mut entropy = 0.0f32;
            for count in histogram {
                if count > 0.0 {
                    let probability = count / samples;
                    entropy -= probability * probability.log2();
                }
            }

            scores[(y * width + x) as usize] = (entropy * normaliser).clamp(0.0, 1.0);
        }
    }

    ScoreGrid {
        scores,
        width,
        height,
    }
}

/// Edge energy, local contrast, saturation and centre bias, normalised to 0..1.
fn attention_grid(image: &RgbaImage) -> ScoreGrid {
    let width = image.width();
    let height = image.height();
    let mut raw = vec![0.0f32; (width * height) as usize];

    let luma_at = |x: u32, y: u32| luma(image.get_pixel(x, y));

    for y in 0..height {
        for x in 0..width {
            let centre = luma_at(x, y);

            // Gradient magnitude from the four direct neighbours.
            let left = luma_at(x.saturating_sub(1), y);
            let right = luma_at((x + 1).min(width - 1), y);
            let up = luma_at(x, y.saturating_sub(1));
            let down = luma_at(x, (y + 1).min(height - 1));
            let gradient = ((right - left).abs() + (down - up).abs()) / 255.0;

            // How far this pixel sits from its own neighbourhood mean.
            let local_mean = (left + right + up + down + centre) / 5.0;
            let contrast = (centre - local_mean).abs() / 255.0;

            let pixel = image.get_pixel(x, y);
            let max_channel = pixel[0].max(pixel[1]).max(pixel[2]);
            let min_channel = pixel[0].min(pixel[1]).min(pixel[2]);
            let saturation = f32::from(max_channel.saturating_sub(min_channel)) / 255.0;

            raw[(y * width + x) as usize] = gradient * 2.0 + contrast + saturation * 0.5;
        }
    }

    // Mild centre bias so flat images do not get an arbitrary corner.
    let centre_x = f64::from(width.saturating_sub(1)) / 2.0;
    let centre_y = f64::from(height.saturating_sub(1)) / 2.0;
    let max_distance = (centre_x * centre_x + centre_y * centre_y)
        .sqrt()
        .max(f64::EPSILON);
    let peak = raw.iter().copied().fold(0.0f32, f32::max).max(f32::EPSILON);

    let scores = raw
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let x = (index as u32) % width;
            let y = (index as u32) / width;
            let dx = f64::from(x) - centre_x;
            let dy = f64::from(y) - centre_y;
            let distance = (dx * dx + dy * dy).sqrt();
            let bias = 1.0 - 0.35 * (distance / max_distance);
            ((f64::from(value / peak)) * bias) as f32
        })
        .collect();

    ScoreGrid {
        scores,
        width,
        height,
    }
}

/// Highest scoring top-left position for a window of the given grid size.
fn best_window(grid: &ScoreGrid, window_width: u32, window_height: u32) -> Option<(u32, u32)> {
    if grid.width == 0 || grid.height == 0 {
        return None;
    }

    let max_x = grid.width.saturating_sub(window_width);
    let max_y = grid.height.saturating_sub(window_height);

    let mut best: Option<(u32, u32, f32)> = None;

    for y in 0..=max_y {
        for x in 0..=max_x {
            let mut total = 0.0f32;
            for window_y in y..y + window_height {
                for window_x in x..x + window_width {
                    total += grid.at(window_x, window_y);
                }
            }

            match best {
                Some((_, _, best_score)) if best_score >= total => {}
                _ => best = Some((x, y, total)),
            }
        }
    }

    best.map(|(x, y, _)| (x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(width: u32, height: u32, colour: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(colour))
    }

    #[test]
    fn image_smaller_than_the_crop_returns_the_origin() {
        let image = solid(10, 10, [0, 0, 0, 255]);
        assert_eq!(focus_origin(&image, 50, 50, CropPosition::Center), (0, 0));
        assert_eq!(focus_origin(&image, 50, 50, CropPosition::Entropy), (0, 0));
        assert_eq!(
            focus_origin(&image, 50, 50, CropPosition::Attention),
            (0, 0)
        );
    }

    #[test]
    fn edge_positions_hug_the_requested_edge() {
        let image = solid(200, 100, [128, 128, 128, 255]);
        assert_eq!(
            focus_origin(&image, 100, 100, CropPosition::Center),
            (50, 0)
        );
        assert_eq!(focus_origin(&image, 100, 100, CropPosition::Left), (0, 0));
        assert_eq!(
            focus_origin(&image, 100, 100, CropPosition::Right),
            (100, 0)
        );
        assert_eq!(focus_origin(&image, 100, 100, CropPosition::Top), (50, 0));
        assert_eq!(
            focus_origin(&image, 100, 100, CropPosition::Bottom),
            (50, 0)
        );
    }

    #[test]
    fn crop_origin_always_keeps_the_window_inside_the_source() {
        let image = solid(320, 180, [10, 200, 40, 255]);
        for position in [
            CropPosition::Center,
            CropPosition::Top,
            CropPosition::Right,
            CropPosition::Bottom,
            CropPosition::Left,
            CropPosition::Entropy,
            CropPosition::Attention,
        ] {
            let (x, y) = focus_origin(&image, 150, 150, position);
            assert!(x + 150 <= 320, "{position:?} overflowed on x");
            assert!(y + 150 <= 180, "{position:?} overflowed on y");
        }
    }

    #[test]
    fn entropy_focus_prefers_the_busy_region() {
        // Left half flat, right half high-frequency stripes.
        let mut image = RgbaImage::new(200, 100);
        for (x, _y, pixel) in image.enumerate_pixels_mut() {
            *pixel = if x < 100 {
                Rgba([120, 120, 120, 255])
            } else if x % 2 == 0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            };
        }

        let (x, _y) = focus_origin(&image, 100, 100, CropPosition::Entropy);
        assert!(x >= 60, "expected the busy right half, got x={x}");
    }

    #[test]
    fn flat_images_do_not_panic_in_either_smart_mode() {
        let image = solid(120, 80, [255, 255, 255, 255]);
        let (x, y) = focus_origin(&image, 40, 40, CropPosition::Entropy);
        assert!(x + 40 <= 120 && y + 40 <= 80);
        let (x, y) = focus_origin(&image, 40, 40, CropPosition::Attention);
        assert!(x + 40 <= 120 && y + 40 <= 80);
    }

    #[test]
    fn single_pixel_images_are_handled() {
        let image = solid(1, 1, [10, 20, 30, 255]);
        assert_eq!(focus_origin(&image, 1, 1, CropPosition::Entropy), (0, 0));
        assert_eq!(focus_origin(&image, 1, 1, CropPosition::Attention), (0, 0));
    }
}
