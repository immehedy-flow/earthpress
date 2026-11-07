//! EXIF orientation handling.
//!
//! The Electron build leaned on `sharp().rotate()`, which reads the EXIF
//! orientation tag and bakes it into the pixels. We do the same: read the tag
//! with `kamadak-exif`, then apply the matching transform.

use exif::{In, Reader, Tag};
use image::{imageops, RgbaImage};
use std::io::Cursor;

/// Returns the EXIF orientation (1-8), or `1` when absent/unreadable.
pub fn read_orientation(bytes: &[u8]) -> u16 {
    let mut cursor = Cursor::new(bytes);
    let Ok(exif) = Reader::new().read_from_container(&mut cursor) else {
        return 1;
    };

    exif.get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .filter(|value| (1..=8).contains(value))
        .map(|value| value as u16)
        .unwrap_or(1)
}

/// True when the tag rotates by 90 or 270 degrees, so width and height trade
/// places once the pixels are baked.
pub fn swaps_axes(orientation: u16) -> bool {
    matches!(orientation, 5..=8)
}

/// Bakes the orientation into the pixels.
pub fn apply(image: RgbaImage, orientation: u16) -> RgbaImage {
    match orientation {
        // 0 degrees.
        1 => image,
        // Mirrored horizontally.
        2 => imageops::flip_horizontal(&image),
        // 180 degrees.
        3 => imageops::rotate180(&image),
        // Mirrored vertically.
        4 => imageops::flip_vertical(&image),
        // Transposed.
        5 => imageops::flip_horizontal(&imageops::rotate90(&image)),
        // 90 degrees clockwise - by far the most common phone-camera case.
        6 => imageops::rotate90(&image),
        // Transversed.
        7 => imageops::flip_horizontal(&imageops::rotate270(&image)),
        // 270 degrees clockwise.
        8 => imageops::rotate270(&image),
        _ => image,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// 3x2 image: first column red, second green, third blue.
    fn striped() -> RgbaImage {
        let mut image = RgbaImage::new(3, 2);
        for (x, _y, pixel) in image.enumerate_pixels_mut() {
            *pixel = match x {
                0 => Rgba([255, 0, 0, 255]),
                1 => Rgba([0, 255, 0, 255]),
                _ => Rgba([0, 0, 255, 255]),
            };
        }
        image
    }

    #[test]
    fn orientation_one_is_a_no_op() {
        let result = apply(striped(), 1);
        assert_eq!((result.width(), result.height()), (3, 2));
        assert_eq!(result.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn orientation_six_rotates_clockwise() {
        let result = apply(striped(), 6);
        assert_eq!((result.width(), result.height()), (2, 3));
        // The old bottom-left red pixel is now top-left.
        assert_eq!(result.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(result.get_pixel(0, 2), &Rgba([0, 0, 255, 255]));
    }

    #[test]
    fn orientation_eight_rotates_counter_clockwise() {
        let result = apply(striped(), 8);
        assert_eq!((result.width(), result.height()), (2, 3));
        // Counter-clockwise turns the right-most column into the top row.
        assert_eq!(result.get_pixel(0, 0), &Rgba([0, 0, 255, 255]));
        assert_eq!(result.get_pixel(1, 2), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn orientation_three_is_a_half_turn() {
        let result = apply(striped(), 3);
        assert_eq!((result.width(), result.height()), (3, 2));
        assert_eq!(result.get_pixel(2, 0), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn out_of_range_orientations_are_ignored() {
        let result = apply(striped(), 99);
        assert_eq!((result.width(), result.height()), (3, 2));

        let result = apply(striped(), 0);
        assert_eq!((result.width(), result.height()), (3, 2));
    }

    #[test]
    fn missing_exif_defaults_to_one() {
        assert_eq!(read_orientation(b"not an image"), 1);
    }

    #[test]
    fn only_quarter_turns_swap_the_axes() {
        for tag in 0..=9u16 {
            assert_eq!(
                swaps_axes(tag),
                (5..=8).contains(&tag),
                "orientation {tag} reported the wrong axis behaviour"
            );
        }
    }
}
