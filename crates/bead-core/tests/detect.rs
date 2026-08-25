//! T-CLS-1 to T-CLS-4 and T-GRID-1: pixel drawing or photograph, and on what
//! lattice.

use bead_core::color::{Rgb, Rgba};
use bead_core::detect::{
    analyze, detect_grid, flat_neighbor_ratio, GridGeometry, ImageKind, DEGENERATE_CONFIDENCE,
};
use bead_core::image::Image;

mod support;

/// Twelve colours, chosen so that no two neighbouring columns or rows of the
/// sprite are identical. That matters for T-GRID-1: if two adjacent columns of
/// the drawing happened to match, the boundary between them would leave no
/// trace and the detected cell size could come out a multiple of the real one.
const SPRITE_COLORS: [(u8, u8, u8); 12] = [
    (0x00, 0x00, 0x00),
    (0xFF, 0xFF, 0xFF),
    (0xE4, 0x03, 0x2E),
    (0xF5, 0x82, 0x1F),
    (0xFF, 0xD4, 0x00),
    (0x2E, 0x9E, 0x45),
    (0x00, 0x9B, 0x9F),
    (0x0B, 0x61, 0xA4),
    (0x59, 0x25, 0x9E),
    (0xE0, 0x21, 0x8A),
    (0x7B, 0x4B, 0x25),
    (0x9E, 0x9E, 0x9E),
];

/// A 24×24 drawing: a border, a body, two eyes, a mouth, and a one-pixel
/// colour ruler along the bottom and right edges.
///
/// The ruler is what makes the drawing legible to T-GRID-1. It brings the
/// colour count to twelve, and because it never repeats a value between
/// neighbours it guarantees that *every* cell boundary leaves a trace — so the
/// detected period cannot come out a multiple of the real one just because two
/// neighbouring cells happened to share a colour.
fn sprite_24() -> Image {
    let colour = |index: u32| {
        let (r, g, b) = SPRITE_COLORS[(index % 12) as usize];
        Rgb::new(r, g, b)
    };
    let mut pixels = Vec::with_capacity(24 * 24);
    for y in 0..24u32 {
        for x in 0..24u32 {
            let pixel = if y == 23 {
                colour(x)
            } else if x == 23 {
                colour(y + 5)
            } else if x == 0 || y == 0 || x == 22 || y == 22 {
                colour(0)
            } else if (7..9).contains(&y) && ((6..8).contains(&x) || (15..17).contains(&x)) {
                colour(7)
            } else if (15..17).contains(&y) && (7..17).contains(&x) {
                colour(2)
            } else {
                colour(4)
            };
            pixels.push(pixel);
        }
    }
    support::opaque(24, 24, &pixels)
}

/// The precondition T-GRID-1 rests on, asserted rather than assumed.
#[test]
fn the_sprite_never_repeats_a_neighbouring_line() {
    let sprite = sprite_24();
    for x in 1..sprite.width() {
        assert!(
            (0..sprite.height()).any(|y| sprite.sample(i64::from(x - 1), i64::from(y))
                != sprite.sample(i64::from(x), i64::from(y))),
            "columns {} and {x} are identical",
            x - 1
        );
    }
    for y in 1..sprite.height() {
        assert!(
            (0..sprite.width()).any(|x| sprite.sample(i64::from(x), i64::from(y - 1))
                != sprite.sample(i64::from(x), i64::from(y))),
            "rows {} and {y} are identical",
            y - 1
        );
    }
}

/// A 256×256 smooth two-axis gradient with well over ten thousand colours.
fn gradient_256() -> Image {
    let mut pixels = Vec::with_capacity(256 * 256);
    for y in 0..256u32 {
        for x in 0..256u32 {
            pixels.push(Rgb::new(x as u8, y as u8, ((x + y) / 2) as u8));
        }
    }
    support::opaque(256, 256, &pixels)
}

fn noise_48() -> Image {
    let mut random = support::Lcg::new(0x0FF1_CE05);
    let mut pixels = Vec::with_capacity(48 * 48);
    for _ in 0..(48 * 48) {
        pixels.push(Rgb::new(random.byte(), random.byte(), random.byte()));
    }
    support::opaque(48, 48, &pixels)
}

/// T-CLS-1.
#[test]
fn an_eight_times_export_is_pixel_art_with_high_confidence() {
    let exported = support::upscale(&sprite_24(), 8);
    assert_eq!((exported.width(), exported.height()), (192, 192));

    let analysis = analyze(&exported);
    assert_eq!(analysis.kind, ImageKind::PixelArt { block_size: 8 });
    assert_eq!(analysis.unique_colors, 12);
    assert!(
        analysis.confidence >= 0.8,
        "confidence was {:.4}",
        analysis.confidence
    );
}

/// T-CLS-1, the other export factors, so the block probe is not pinned to one
/// lucky number.
#[test]
fn other_export_factors_are_read_at_their_own_size() {
    for factor in [2u32, 3, 4, 8] {
        let analysis = analyze(&support::upscale(&sprite_24(), factor));
        assert_eq!(
            analysis.kind,
            ImageKind::PixelArt { block_size: factor },
            "{factor}x export"
        );
    }
}

/// T-CLS-1 at native resolution: no upscale to lean on, so the verdict has to
/// come from the flat neighbours and the small palette alone.
#[test]
fn a_native_resolution_drawing_is_still_pixel_art() {
    let analysis = analyze(&sprite_24());
    assert!(
        analysis.kind.is_pixel_art(),
        "verdict was {:?} at confidence {:.4}",
        analysis.kind,
        analysis.confidence
    );
    assert_eq!(analysis.kind.block_size(), 1);
}

/// T-CLS-2.
#[test]
fn a_smooth_gradient_is_a_photograph() {
    let image = gradient_256();
    let analysis = analyze(&image);
    assert!(
        analysis.unique_colors >= 10_000,
        "the sample needs to be genuinely continuous-tone, had {}",
        analysis.unique_colors
    );
    assert_eq!(analysis.kind, ImageKind::Photo);
    assert!(analysis.confidence > 0.9);
}

#[test]
fn noise_is_a_photograph() {
    let analysis = analyze(&noise_48());
    assert_eq!(analysis.kind, ImageKind::Photo);
    assert!(analysis.confidence > 0.9);
    assert_eq!(analysis.geometry, GridGeometry::NONE);
}

/// T-CLS-3. Every degenerate shape, and nothing anywhere is NaN or out of
/// range. A fully transparent image is the dangerous one: the opaque pixel
/// count is zero, and every ratio in the classifier has it in the denominator.
#[test]
fn confidence_is_always_a_number_between_zero_and_one() {
    let cases: Vec<(&str, Image)> = vec![
        (
            "one pixel",
            Image::filled(1, 1, Rgba::new(1, 2, 3, 255)).expect("1x1"),
        ),
        (
            "single colour",
            Image::filled(32, 32, Rgba::new(9, 9, 9, 255)).expect("32x32"),
        ),
        ("noise", noise_48()),
        (
            "fully transparent",
            Image::filled(16, 16, Rgba::TRANSPARENT).expect("16x16"),
        ),
        (
            "transparent with one bead",
            support::ring(8, 8, Some((0, 0, 8, 7)), Rgba::new(4, 5, 6, 255)),
        ),
        ("gradient", gradient_256()),
    ];
    for (what, image) in cases {
        let analysis = analyze(&image);
        assert!(
            analysis.confidence.is_finite(),
            "{what}: confidence was {}",
            analysis.confidence
        );
        assert!(
            (0.0..=1.0).contains(&analysis.confidence),
            "{what}: confidence was {}",
            analysis.confidence
        );
        assert!(
            analysis.flat_neighbor_ratio.is_finite()
                && (0.0..=1.0).contains(&analysis.flat_neighbor_ratio),
            "{what}: flat ratio was {}",
            analysis.flat_neighbor_ratio
        );
        assert!(analysis.block_size >= 1, "{what}: block size was 0");
    }
}

/// T-CLS-4. The two inputs the contract pins rather than computes.
#[test]
fn the_degenerate_inputs_are_pinned() {
    let transparent = analyze(&Image::filled(16, 16, Rgba::TRANSPARENT).expect("16x16"));
    assert_eq!(transparent.kind, ImageKind::PixelArt { block_size: 1 });
    assert_eq!(transparent.confidence, DEGENERATE_CONFIDENCE);
    assert_eq!(transparent.unique_colors, 0);

    let single = analyze(&Image::filled(32, 32, Rgba::new(9, 9, 9, 255)).expect("32x32"));
    assert_eq!(single.kind, ImageKind::PixelArt { block_size: 1 });
    assert_eq!(single.confidence, DEGENERATE_CONFIDENCE);
    assert_eq!(single.unique_colors, 1);

    // One bead in an otherwise empty image is also a single colour.
    let lonely = analyze(&support::ring(
        8,
        8,
        Some((0, 0, 8, 7)),
        Rgba::new(4, 5, 6, 255),
    ));
    assert_eq!(lonely.kind, ImageKind::PixelArt { block_size: 1 });
    assert_eq!(lonely.confidence, DEGENERATE_CONFIDENCE);
}

/// T-GRID-1, the aligned case.
#[test]
fn the_lattice_of_an_aligned_export_is_found() {
    let exported = support::upscale(&sprite_24(), 8);
    assert_eq!(
        detect_grid(&exported),
        GridGeometry {
            cell_width: 8,
            cell_height: 8,
            offset_x: 0,
            offset_y: 0
        }
    );
}

/// T-GRID-1, the shifted case: the same export with a three-pixel margin, so
/// the first cell is a partial one and every boundary after it is at
/// `3 + 8k`. Reading the period off the gaps rather than the first boundary is
/// what makes this come out 8-with-phase-3 rather than 1.
#[test]
fn a_three_pixel_phase_shift_is_reported_as_a_phase_shift() {
    let exported = support::upscale(&sprite_24(), 8);
    let margin = Rgba::new(0x33, 0x33, 0x33, 255);
    let width = exported.width() + 3;
    let height = exported.height() + 3;

    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(if x < 3 || y < 3 {
                margin
            } else {
                exported.sample(i64::from(x) - 3, i64::from(y) - 3)
            });
        }
    }
    let shifted = Image::from_pixels(width, height, pixels).expect("shifted");

    assert_eq!(
        detect_grid(&shifted),
        GridGeometry {
            cell_width: 8,
            cell_height: 8,
            offset_x: 3,
            offset_y: 3
        }
    );
    // A shifted lattice cannot be read by plain decimation, so the block size a
    // caller would decimate by stays 1 even though the cell size is 8.
    assert_eq!(detect_grid(&shifted).block_size(), 1);
    assert!(analyze(&shifted).kind.is_pixel_art());
}

#[test]
fn a_continuous_image_has_no_lattice() {
    assert_eq!(detect_grid(&gradient_256()), GridGeometry::NONE);
    assert_eq!(detect_grid(&noise_48()), GridGeometry::NONE);
    assert_eq!(detect_grid(&sprite_24()), GridGeometry::NONE);
}

/// Non-square cells are a real export shape — a sprite sheet stretched on one
/// axis — and the two axes are solved independently, so they come out right.
#[test]
fn a_non_square_lattice_is_reported_per_axis() {
    let sprite = sprite_24();
    let (fx, fy) = (6u32, 2u32);
    let width = sprite.width() * fx;
    let height = sprite.height() * fy;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(sprite.sample(i64::from(x / fx), i64::from(y / fy)));
        }
    }
    let stretched = Image::from_pixels(width, height, pixels).expect("stretched");

    let geometry = detect_grid(&stretched);
    assert_eq!(geometry.cell_width, 6);
    assert_eq!(geometry.cell_height, 2);
    assert!(!geometry.is_square());
    assert_eq!(geometry.block_size(), 1, "not square, so not decimable");
}

#[test]
fn flat_neighbour_ratio_spans_its_range() {
    let flat = Image::filled(8, 8, Rgba::new(1, 2, 3, 255)).expect("8x8");
    assert!((flat_neighbor_ratio(&flat) - 1.0).abs() < 1e-12);
    assert!(flat_neighbor_ratio(&gradient_256()) < 0.01);
    assert_eq!(
        flat_neighbor_ratio(&Image::filled(4, 4, Rgba::TRANSPARENT).expect("4x4")),
        0.0,
        "no opaque pairs means no ratio, not a division by zero"
    );
}
