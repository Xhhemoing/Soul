//! The pixel-drawing versus photograph heuristic.

use bead_core::color::Rgb;
use bead_core::detect::{analyze, detect_block_size, flat_neighbor_ratio, ImageKind};
use bead_core::image::Image;

/// A 16×16 sprite: a one-cell border, a filled body, two eyes and a mouth.
/// Deliberately not aligned to any power of two so the block probe cannot find
/// a period that is not there.
fn sprite_16() -> Image {
    let border = Rgb::new(0x1A, 0x1A, 0x1A);
    let body = Rgb::new(0xFF, 0xD4, 0x00);
    let eye = Rgb::new(0x0B, 0x61, 0xA4);
    let mouth = Rgb::new(0xE4, 0x03, 0x2E);

    let mut pixels = Vec::with_capacity(16 * 16);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let colour = if x == 0 || y == 0 || x == 15 || y == 15 {
                border
            } else if (y == 5 || y == 6) && (x == 5 || x == 10) {
                eye
            } else if y == 11 && (3..=12).contains(&x) {
                mouth
            } else {
                body
            };
            pixels.push(colour);
        }
    }
    Image::from_pixels(16, 16, pixels).expect("16x16")
}

fn upscale(image: &Image, factor: u32) -> Image {
    let width = image.width() * factor;
    let height = image.height() * factor;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(image.clamped(i64::from(x / factor), i64::from(y / factor)));
        }
    }
    Image::from_pixels(width, height, pixels).expect("upscaled")
}

/// A smooth two-axis ramp: every pixel differs from both its neighbours, and
/// there are as many colours as there are pixels.
fn ramp_64() -> Image {
    let mut pixels = Vec::with_capacity(64 * 64);
    for y in 0..64u32 {
        for x in 0..64u32 {
            pixels.push(Rgb::new((x * 4) as u8, (y * 4) as u8, ((x + y) * 2) as u8));
        }
    }
    Image::from_pixels(64, 64, pixels).expect("64x64")
}

/// Deterministic noise, standing in for sensor grain.
fn noise_48() -> Image {
    let mut state: u32 = 0x1234_5678;
    let mut next = move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 16) as u8
    };
    let mut pixels = Vec::with_capacity(48 * 48);
    for _ in 0..(48 * 48) {
        pixels.push(Rgb::new(next(), next(), next()));
    }
    Image::from_pixels(48, 48, pixels).expect("48x48")
}

#[test]
fn a_native_resolution_sprite_reads_as_pixel_art() {
    let analysis = analyze(&sprite_16());
    assert!(
        analysis.kind.is_pixel_art(),
        "verdict was {:?} at confidence {:.3}",
        analysis.kind,
        analysis.confidence
    );
    assert_eq!(analysis.kind.block_size(), 1, "it is already 1:1");
    assert_eq!(analysis.unique_colors, 4);
}

#[test]
fn an_upscaled_sprite_reads_as_pixel_art_and_reports_its_period() {
    for factor in [2u32, 4, 8] {
        let analysis = analyze(&upscale(&sprite_16(), factor));
        assert_eq!(
            analysis.kind,
            ImageKind::PixelArt { block_size: factor },
            "{factor}x upscale"
        );
        assert!(
            analysis.confidence > 0.5,
            "{factor}x upscale should be an easy call, was {:.3}",
            analysis.confidence
        );
    }
}

#[test]
fn a_gradient_reads_as_a_photograph() {
    let analysis = analyze(&ramp_64());
    assert_eq!(analysis.kind, ImageKind::Photo);
    assert_eq!(analysis.kind.block_size(), 1);
    assert!(
        analysis.confidence > 0.9,
        "a ramp should be an easy call, was {:.3}",
        analysis.confidence
    );
    assert!(analysis.flat_neighbor_ratio < 0.01);
}

#[test]
fn noise_reads_as_a_photograph() {
    let analysis = analyze(&noise_48());
    assert_eq!(analysis.kind, ImageKind::Photo);
    assert!(analysis.confidence > 0.9);
    assert_eq!(analysis.block_size, 1);
}

#[test]
fn confidence_stays_in_range() {
    for image in [sprite_16(), upscale(&sprite_16(), 4), ramp_64(), noise_48()] {
        let analysis = analyze(&image);
        assert!(
            (0.0..=1.0).contains(&analysis.confidence),
            "confidence was {}",
            analysis.confidence
        );
    }
}

#[test]
fn the_block_probe_takes_the_largest_period() {
    // A 4× upscale is also 2×-flat; the answer has to be 4, or a caller would
    // read the drawing at twice the resolution it was made at.
    assert_eq!(detect_block_size(&upscale(&sprite_16(), 4)), 4);
    assert_eq!(detect_block_size(&sprite_16()), 1);
    assert_eq!(detect_block_size(&ramp_64()), 1);
}

#[test]
fn flat_neighbour_ratio_spans_its_range() {
    let flat = Image::filled(8, 8, Rgb::new(1, 2, 3)).expect("8x8");
    assert!((flat_neighbor_ratio(&flat) - 1.0).abs() < 1e-12);
    assert!(flat_neighbor_ratio(&ramp_64()) < 0.01);
    let sprite = flat_neighbor_ratio(&sprite_16());
    assert!(
        (0.5..1.0).contains(&sprite),
        "a sprite is mostly flat but not entirely, was {sprite:.3}"
    );
}

/// A single pixel has no neighbours and no period. It must classify rather than
/// divide by zero.
#[test]
fn a_one_pixel_image_is_handled() {
    let image = Image::filled(1, 1, Rgb::new(0, 0, 0)).expect("1x1");
    let analysis = analyze(&image);
    assert_eq!(analysis.unique_colors, 1);
    assert_eq!(analysis.block_size, 1);
    assert_eq!(analysis.flat_neighbor_ratio, 0.0);
    assert!((0.0..=1.0).contains(&analysis.confidence));
}
