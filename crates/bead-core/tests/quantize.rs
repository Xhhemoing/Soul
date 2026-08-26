//! T-FS-1 to T-FS-7: mapping a raster onto a palette, with and without
//! Floyd–Steinberg.

use std::collections::BTreeSet;

use bead_core::color::{Rgb, Rgba};
use bead_core::grid::Cell;
use bead_core::image::Image;
use bead_core::palette::{BeadColor, ColorId, Palette};
use bead_core::quantize::{
    map_image, map_pixel, Dither, MapOptions, PatternGrid, FLOYD_STEINBERG_KERNEL,
};

mod support;

/// Black and white and nothing else: the classic dither test palette, and the
/// one the contract hand-derives T-FS-2 against.
fn mono() -> Palette {
    Palette::new(
        "mono",
        vec![
            BeadColor::new("K", "Black", Rgb::new(0, 0, 0)),
            BeadColor::new("W", "White", Rgb::new(255, 255, 255)),
        ],
    )
    .expect("valid palette")
}

fn black(palette: &Palette) -> ColorId {
    palette.find_code("K").expect("K")
}

fn white(palette: &Palette) -> ColorId {
    palette.find_code("W").expect("W")
}

fn codes(grid: &PatternGrid, palette: &Palette) -> BTreeSet<String> {
    grid.iter()
        .filter_map(|(_, slot)| slot.map(|id| palette.color(id).code.clone()))
        .collect()
}

fn grey(width: u32, height: u32, level: u8) -> Image {
    Image::filled(width, height, Rgba::new(level, level, level, 255)).expect("grey")
}

#[test]
fn the_kernel_is_the_published_one() {
    assert_eq!(
        FLOYD_STEINBERG_KERNEL
            .iter()
            .map(|(dx, dy, _)| (*dx, *dy))
            .collect::<Vec<_>>(),
        vec![(1, 0), (-1, 1), (0, 1), (1, 1)]
    );
    let sixteenths: Vec<f64> = FLOYD_STEINBERG_KERNEL
        .iter()
        .map(|(_, _, w)| w * 16.0)
        .collect();
    assert_eq!(sixteenths, vec![7.0, 3.0, 5.0, 1.0]);
}

/// T-FS-1. With no error to diffuse, the two paths are the same function.
#[test]
fn dithering_an_exact_swatch_changes_nothing() {
    let palette = Palette::generic_5mm();
    let red = palette.color(palette.find_code("G15").expect("G15")).rgb;
    let image = Image::filled(16, 16, Rgba::opaque(red)).expect("16x16");

    let flat = map_image(&image, &palette, MapOptions::default());
    let dithered = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    assert_eq!(flat, dithered);
    assert_eq!(codes(&flat, &palette), BTreeSet::from(["G15".to_owned()]));
}

/// T-FS-1, the general case: on a picture made only of exact palette swatches
/// there is still no error anywhere, so the two paths agree cell for cell.
#[test]
fn the_two_paths_agree_wherever_the_error_is_zero() {
    let palette = Palette::generic_5mm();
    let swatches: Vec<Rgb> = ["G01", "G06", "G15", "G22", "G33", "G26"]
        .iter()
        .map(|code| palette.color(palette.find_code(code).expect("code")).rgb)
        .collect();
    let mut pixels = Vec::new();
    for y in 0..6u32 {
        for x in 0..6u32 {
            pixels.push(swatches[((x + y) % 6) as usize]);
        }
    }
    let image = support::opaque(6, 6, &pixels);

    assert_eq!(
        map_image(&image, &palette, MapOptions::default()),
        map_image(
            &image,
            &palette,
            MapOptions::with_dither(Dither::FloydSteinberg)
        )
    );
}

/// T-FS-2. The hand-derivable case, spelled out in the contract: four mid-grey
/// pixels and a black-and-white palette give a checkerboard.
///
/// Deriving it: (0,0) is nearer white, leaving −127 of error, which goes 7/16
/// right, 5/16 down and 1/16 diagonally. (1,0) is then dark enough to be black
/// and pushes +72.4 back down and left; (0,1) is likewise black; the
/// accumulated error carries (1,1) up to white.
#[test]
fn the_weights_and_scan_order_produce_the_derived_checkerboard() {
    let palette = mono();
    let grid = map_image(
        &grey(2, 2, 128),
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    let (k, w) = (black(&palette), white(&palette));
    assert_eq!(
        grid.as_slice(),
        &[Some(w), Some(k), Some(k), Some(w)],
        "expected [white, black; black, white]"
    );
}

/// T-FS-3. A one-row ramp: error can only travel right, and whatever is left at
/// the last pixel is dropped rather than wrapped onto the next row.
#[test]
fn a_single_row_ramp_is_frozen() {
    let palette = mono();
    let levels: Vec<Rgb> = (0..8u32)
        .map(|i| Rgb::new((i * 32) as u8, (i * 32) as u8, (i * 32) as u8))
        .collect();
    let image = support::opaque(8, 1, &levels);
    let grid = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    let (k, w) = (black(&palette), white(&palette));
    assert_eq!(
        grid.as_slice(),
        &[
            Some(k),
            Some(k),
            Some(k),
            Some(w),
            Some(k),
            Some(w),
            Some(w),
            Some(w)
        ]
    );

    // The same run twice: nothing here depends on iteration order.
    assert_eq!(
        grid,
        map_image(
            &image,
            &palette,
            MapOptions::with_dither(Dither::FloydSteinberg)
        )
    );
}

/// T-FS-4. Degenerate shapes: the kernel reaches off every edge and must simply
/// drop what falls outside.
#[test]
fn the_kernel_never_reaches_out_of_bounds() {
    let palette = mono();
    for (width, height) in [(1u32, 1u32), (1, 9), (9, 1), (2, 3), (3, 2)] {
        let grid = map_image(
            &grey(width, height, 128),
            &palette,
            MapOptions::with_dither(Dither::FloydSteinberg),
        );
        assert_eq!(grid.width(), width);
        assert_eq!(grid.height(), height);
        assert_eq!(grid.len(), (width * height) as usize);
        assert!(grid.iter().all(|(_, slot)| slot.is_some()));
    }
}

/// T-FS-5. Error accumulation can carry a working value far outside `0..=255`.
/// It still has to come back as a bead.
#[test]
fn an_out_of_range_accumulation_still_lands_on_a_bead() {
    let palette = mono();
    // Alternating extremes: every pixel hands its neighbour an error the size
    // of the whole range, and the accumulation overshoots in both directions.
    let mut pixels = Vec::new();
    for y in 0..16u32 {
        for x in 0..16u32 {
            pixels.push(if (x / 3 + y / 5) % 2 == 0 {
                Rgb::new(255, 255, 255)
            } else {
                Rgb::new(0, 0, 0)
            });
        }
    }
    let image = support::opaque(16, 16, &pixels);
    let grid = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    assert_eq!(grid.len(), 256);
    for (cell, slot) in grid.iter() {
        let id = slot.unwrap_or_else(|| panic!("{cell} came back empty"));
        assert!(palette.get(id).is_some(), "{cell} got {id}, not a bead");
    }
}

/// T-FS-6. Mid grey on a black-and-white palette should come out roughly
/// 128/255 white. If the error were being quietly clamped away as it
/// accumulated, the mixture would drift towards one extreme.
#[test]
fn the_mixture_preserves_the_mean() {
    let palette = mono();
    let grid = map_image(
        &grey(64, 64, 128),
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    let w = white(&palette);
    let whites = grid.iter().filter(|(_, slot)| **slot == Some(w)).count() as f64;
    let share = whites / grid.len() as f64;
    let wanted = 128.0 / 255.0;
    assert!(
        (share - wanted).abs() <= 0.02,
        "white share was {share:.4}, wanted {wanted:.4}"
    );
}

/// T-FS-7. A hole in the middle: it takes no bead, and it neither absorbs the
/// error that would have landed on it nor passes any on.
#[test]
fn an_empty_cell_is_isolated_from_the_error() {
    let palette = mono();
    let image = support::ring(8, 8, Some((3, 3, 2, 2)), Rgba::new(128, 128, 128, 255));
    let grid = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );

    for y in 3..5u32 {
        for x in 3..5u32 {
            assert_eq!(
                grid.get(Cell::new(x, y)),
                Some(&None),
                "the hole at ({x}, {y}) took a bead"
            );
        }
    }
    assert_eq!(
        grid.iter().filter(|(_, slot)| slot.is_some()).count(),
        64 - 4
    );

    // Frozen: the exact pattern around the hole is what the browser port has to
    // reproduce, and it is the part most sensitive to getting the isolation
    // rule wrong.
    let rendered: String = (0..8)
        .map(|y| {
            (0..8)
                .map(|x| match grid.get(Cell::new(x, y)) {
                    Some(Some(id)) => palette.color(*id).code.chars().next().unwrap_or('?'),
                    _ => '.',
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        rendered,
        "WKWKWKWK\n\
         KWKWKWKW\n\
         WKWKWKWK\n\
         KWK..WKW\n\
         WKW..KWK\n\
         KWKWKWKW\n\
         WKWKWKWK\n\
         KWKWKWKW"
    );
}

#[test]
fn dither_none_is_the_default_and_maps_pixels_independently() {
    let palette = Palette::generic_5mm();
    assert!(!Dither::default().is_enabled());
    assert!(Dither::FloydSteinberg.is_enabled());

    let pixels: Vec<Rgb> = ["#FEFEFE", "#040404", "#E30D30", "#0C62A5"]
        .iter()
        .map(|hex| Rgb::from_hex(hex).expect("hex"))
        .collect();
    let image = support::opaque(4, 1, &pixels);
    let grid = map_image(&image, &palette, MapOptions::default());

    let got: Vec<&str> = grid
        .iter()
        .map(|(_, slot)| palette.color(slot.expect("a bead")).code.as_str())
        .collect();
    assert_eq!(got, vec!["G01", "G06", "G15", "G33"]);
    for (pixel, code) in pixels.iter().zip(&got) {
        let id = map_pixel(Rgba::opaque(*pixel), &palette).expect("a bead");
        assert_eq!(palette.color(id).code, *code);
    }
    assert_eq!(map_pixel(Rgba::TRANSPARENT, &palette), None);
}

/// Without dithering, a transparent pixel is an empty cell and nothing else.
#[test]
fn transparency_survives_the_undithered_path() {
    let palette = Palette::generic_5mm();
    let image = support::ring(4, 4, Some((1, 1, 2, 2)), Rgba::new(255, 212, 0, 255));
    let grid = map_image(&image, &palette, MapOptions::default());
    assert_eq!(grid.iter().filter(|(_, slot)| slot.is_none()).count(), 4);
    assert_eq!(codes(&grid, &palette), BTreeSet::from(["G22".to_owned()]));
}
