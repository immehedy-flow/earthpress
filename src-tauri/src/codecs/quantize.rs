//! A small median-cut colour quantiser for indexed PNG output.
//!
//! The `png` crate writes the palette, but it does not pick one, and pulling in
//! a full quantiser crate for this single job would be overkill. This is a
//! compact median-cut implementation:
//!
//! 1. Bucket pixels into a 15-bit (5 bits per channel) histogram, keeping the
//!    accumulated channel sums so each bucket knows its true average colour.
//! 2. Repeatedly split the largest box along its widest channel at the
//!    weighted median.
//! 3. Map every bucket to its nearest palette entry once (32k comparisons),
//!    then remap pixels with an O(1) table lookup.
//!
//! No dithering is applied, which keeps the code short and the output
//! deterministic. Flat art and logos - the usual reason to pick a palette -
//! quantise cleanly without it.

use image::RgbaImage;

const BITS: u32 = 5;
const LEVELS: usize = 1 << BITS; // 32
const SHIFT: u32 = 8 - BITS; // 3
const BINS: usize = LEVELS * LEVELS * LEVELS; // 32768

/// Result of quantising an image.
pub struct QuantizedImage {
    /// RGB triples, three bytes per palette entry.
    pub entries: Vec<u8>,
    /// One alpha byte per palette entry. Empty when the image is fully opaque,
    /// which lets the caller skip the `tRNS` chunk.
    pub transparency: Vec<u8>,
    /// One palette index per pixel, row-major.
    pub indices: Vec<u8>,
}

#[derive(Debug)]
struct ColorBox {
    start: usize,
    end: usize,
    count: u64,
    widest_channel: usize,
}

fn bin_of(red: u8, green: u8, blue: u8) -> usize {
    let r = usize::from(red >> SHIFT);
    let g = usize::from(green >> SHIFT);
    let b = usize::from(blue >> SHIFT);
    (r << (BITS * 2)) | (g << BITS) | b
}

fn bin_centre(bin: usize) -> (u8, u8, u8) {
    let mask = LEVELS - 1;
    let r = ((bin >> (BITS * 2)) & mask) as u8;
    let g = ((bin >> BITS) & mask) as u8;
    let b = (bin & mask) as u8;
    let half = 1u8 << (SHIFT - 1);

    (
        (r << SHIFT).saturating_add(half),
        (g << SHIFT).saturating_add(half),
        (b << SHIFT).saturating_add(half),
    )
}

fn channel_value(bin: usize, channel: usize) -> u8 {
    let (r, g, b) = bin_centre(bin);
    match channel {
        0 => r,
        1 => g,
        _ => b,
    }
}

fn build_box(active: &[usize], start: usize, end: usize, counts: &[u32]) -> ColorBox {
    let mut count = 0u64;
    let mut min = [255u8; 3];
    let mut max = [0u8; 3];

    for &bin in &active[start..end] {
        count += u64::from(counts[bin]);
        let (r, g, b) = bin_centre(bin);
        let colour = [r, g, b];
        for channel in 0..3 {
            min[channel] = min[channel].min(colour[channel]);
            max[channel] = max[channel].max(colour[channel]);
        }
    }

    let ranges = [
        max[0].saturating_sub(min[0]),
        max[1].saturating_sub(min[1]),
        max[2].saturating_sub(min[2]),
    ];
    let widest_channel = (0..3).max_by_key(|&channel| ranges[channel]).unwrap_or(0);

    ColorBox {
        start,
        end,
        count,
        widest_channel,
    }
}

/// Reduces `image` to at most `max_colors` palette entries.
pub fn quantize(image: &RgbaImage, max_colors: usize) -> QuantizedImage {
    let max_colors = max_colors.clamp(1, 256);
    let (width, height) = image.dimensions();
    let pixel_count = (width as usize) * (height as usize);

    if pixel_count == 0 {
        return QuantizedImage {
            entries: vec![0, 0, 0],
            transparency: Vec::new(),
            indices: Vec::new(),
        };
    }

    let mut counts = vec![0u32; BINS];
    let mut sum_red = vec![0u64; BINS];
    let mut sum_green = vec![0u64; BINS];
    let mut sum_blue = vec![0u64; BINS];
    let mut sum_alpha = vec![0u64; BINS];

    for pixel in image.pixels() {
        let bin = bin_of(pixel[0], pixel[1], pixel[2]);
        counts[bin] += 1;
        sum_red[bin] += u64::from(pixel[0]);
        sum_green[bin] += u64::from(pixel[1]);
        sum_blue[bin] += u64::from(pixel[2]);
        sum_alpha[bin] += u64::from(pixel[3]);
    }

    let mut active: Vec<usize> = (0..BINS).filter(|&bin| counts[bin] > 0).collect();

    if active.is_empty() {
        return QuantizedImage {
            entries: vec![0, 0, 0],
            transparency: Vec::new(),
            indices: vec![0; pixel_count],
        };
    }

    let mut boxes = vec![build_box(&active, 0, active.len(), &counts)];

    while boxes.len() < max_colors {
        let Some(index) = boxes
            .iter()
            .enumerate()
            .filter(|(_, item)| item.end - item.start > 1)
            .max_by_key(|(_, item)| item.count)
            .map(|(index, _)| index)
        else {
            // Every remaining box holds a single colour.
            break;
        };

        let target = boxes.swap_remove(index);
        let channel = target.widest_channel;

        active[target.start..target.end].sort_by_key(|&bin| channel_value(bin, channel));

        // Split at the weighted median so both halves hold a similar number of
        // pixels; that is what keeps a dominant background colour from
        // swallowing the palette.
        let half = target.count / 2;
        let mut running = 0u64;
        let mut split = target.start + 1;
        for index in target.start..target.end {
            running += u64::from(counts[active[index]]);
            if running > half {
                split = index + 1;
                break;
            }
        }
        split = split.clamp(target.start + 1, target.end - 1);

        boxes.push(build_box(&active, target.start, split, &counts));
        boxes.push(build_box(&active, split, target.end, &counts));
    }

    // Average colour of every box becomes a palette entry.
    let mut entries = Vec::with_capacity(boxes.len() * 3);
    let mut transparency = Vec::with_capacity(boxes.len());

    for item in &boxes {
        let mut total = 0u64;
        let mut red = 0u64;
        let mut green = 0u64;
        let mut blue = 0u64;
        let mut alpha = 0u64;

        for &bin in &active[item.start..item.end] {
            total += u64::from(counts[bin]);
            red += sum_red[bin];
            green += sum_green[bin];
            blue += sum_blue[bin];
            alpha += sum_alpha[bin];
        }

        let total = total.max(1);
        entries.push((red / total) as u8);
        entries.push((green / total) as u8);
        entries.push((blue / total) as u8);
        transparency.push((alpha / total) as u8);
    }

    if transparency.iter().all(|&value| value == 255) {
        transparency.clear();
    }

    // One nearest-colour lookup per bucket, so per-pixel work stays O(1).
    let mut lookup = vec![0u8; BINS];
    for (bin, slot) in lookup.iter_mut().enumerate() {
        let (red, green, blue) = bin_centre(bin);
        let mut best = 0usize;
        let mut best_distance = u32::MAX;

        for index in 0..boxes.len() {
            let dr = i32::from(red) - i32::from(entries[index * 3]);
            let dg = i32::from(green) - i32::from(entries[index * 3 + 1]);
            let db = i32::from(blue) - i32::from(entries[index * 3 + 2]);
            let distance = (dr * dr + dg * dg + db * db) as u32;
            if distance < best_distance {
                best_distance = distance;
                best = index;
            }
        }

        *slot = best as u8;
    }

    let mut indices = Vec::with_capacity(pixel_count);
    for pixel in image.pixels() {
        indices.push(lookup[bin_of(pixel[0], pixel[1], pixel[2])]);
    }

    QuantizedImage {
        entries,
        transparency,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn flat(width: u32, height: u32, colour: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(colour))
    }

    #[test]
    fn single_colour_image_yields_one_entry() {
        let result = quantize(&flat(8, 8, [10, 20, 30, 255]), 256);
        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.entries[0..3], [10, 20, 30]);
        assert!(result.transparency.is_empty());
        assert_eq!(result.indices, vec![0u8; 64]);
    }

    #[test]
    fn four_colour_image_keeps_them_distinct() {
        let colours = [
            [255u8, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 255],
        ];

        let mut image = RgbaImage::new(4, 1);
        for (index, pixel) in image.pixels_mut().enumerate() {
            *pixel = Rgba(colours[index]);
        }

        let result = quantize(&image, 4);
        assert_eq!(result.entries.len() / 3, 4);

        // Every source pixel must map to a palette entry close to itself.
        for (index, &colour) in colours.iter().enumerate() {
            let entry = usize::from(result.indices[index]) * 3;
            for (channel, &value) in colour.iter().take(3).enumerate() {
                let difference = i32::from(result.entries[entry + channel]) - i32::from(value);
                assert!(
                    difference.abs() <= 8,
                    "channel {channel} drifted by {difference}"
                );
            }
        }
    }

    #[test]
    fn palette_never_exceeds_the_requested_size() {
        let mut image = RgbaImage::new(32, 32);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgba([(x * 8) as u8, (y * 8) as u8, ((x + y) * 4) as u8, 255]);
        }

        for limit in [2usize, 8, 64] {
            let result = quantize(&image, limit);
            assert!(
                result.entries.len() / 3 <= limit,
                "requested {limit}, got {}",
                result.entries.len() / 3
            );
            assert_eq!(result.indices.len(), 32 * 32);
        }
    }

    #[test]
    fn zero_max_colours_is_clamped_to_one() {
        let result = quantize(&flat(4, 4, [1, 2, 3, 255]), 0);
        assert_eq!(result.entries.len() / 3, 1);
    }

    #[test]
    fn transparency_is_kept_when_pixels_are_translucent() {
        let mut image = RgbaImage::new(2, 1);
        image.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, Rgba([0, 0, 255, 0]));

        let result = quantize(&image, 2);
        assert_eq!(result.transparency.len(), result.entries.len() / 3);
        assert!(result.transparency.iter().any(|&alpha| alpha < 255));
    }

    #[test]
    fn empty_images_are_handled() {
        let result = quantize(&RgbaImage::new(0, 0), 256);
        assert_eq!(result.entries.len(), 3);
        assert!(result.indices.is_empty());
    }
}
