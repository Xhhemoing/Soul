//! Mapping a raster onto a palette, with and without Floyd–Steinberg.

use std::collections::BTreeSet;

use bead_core::color::Rgb;
use bead_core::image::Image;
use bead_core::palette::{ColorId, Palette};
use bead_core::quantize::{map_image, map_pixel, Dither, MapOptions, FLOYD_STEINBERG_KERNEL};

fn codes(grid: &bead_core::Grid<ColorId>, palette: &Palette) -> BTreeSet<String> {
    grid.iter()
        .map(|(_, id)| palette.color(*id).code.clone())
        .collect()
}

/// Mean of the beads that were actually placed, in sRGB code values.
fn mean_placed(grid: &bead_core::Grid<ColorId>, palette: &Palette) -> [f64; 3] {
    let mut sum = [0f64; 3];
    for (_, id) in grid.iter() {
        let rgb = palette.color(*id).rgb;
        sum[0] += f64::from(rgb.r);
        sum[1] += f64::from(rgb.g);
        sum[2] += f64::from(rgb.b);
    }
    let n = grid.len() as f64;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
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

#[test]
fn an_exact_swatch_needs_no_dithering_either_way() {
    let palette = Palette::generic_5mm();
    let red = palette.color(palette.find_code("G15").expect("G15")).rgb;
    let image = Image::filled(16, 16, red).expect("16x16");

    for dither in [
        Dither::None,
        Dither::FloydSteinberg,
        Dither::FloydSteinbergSerpentine,
    ] {
        let grid = map_image(&image, &palette, MapOptions::with_dither(dither));
        assert_eq!(
            codes(&grid, &palette),
            BTreeSet::from(["G15".to_owned()]),
            "{dither:?} invented a colour on an exact swatch"
        );
    }
}

/// The point of dithering: a colour that is not in the palette becomes a mix
/// whose average is closer to the original than any single bead could be.
#[test]
fn dithering_trades_a_flat_error_for_a_mixture() {
    let palette = Palette::generic_5mm();
    // Halfway between G01 White and G03 Light Grey, so neither is a good answer.
    let target = Rgb::new(0xE9, 0xE9, 0xE9);
    let image = Image::filled(32, 32, target).expect("32x32");
    let wanted = [
        f64::from(target.r),
        f64::from(target.g),
        f64::from(target.b),
    ];

    let flat = map_image(&image, &palette, MapOptions::default());
    let dithered = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );

    assert_eq!(codes(&flat, &palette).len(), 1, "no dithering, one colour");
    assert!(
        codes(&dithered, &palette).len() >= 2,
        "dithering should reach for a second colour"
    );
    assert!(
        distance(mean_placed(&dithered, &palette), wanted)
            < distance(mean_placed(&flat, &palette), wanted),
        "the dithered average should sit closer to the original"
    );
}

/// Accumulated error can push a working pixel past 0 or 255. It has to clamp
/// into the palette rather than wrap, or a bright edge grows a black fringe.
#[test]
fn out_of_range_error_still_lands_on_a_bead() {
    let palette = Palette::generic_5mm();
    // A hard black-and-white edge is the worst case: every boundary pixel
    // carries an error the size of the whole range into its neighbour.
    let mut pixels = Vec::new();
    for y in 0..16u32 {
        for x in 0..16u32 {
            pixels.push(if (x / 4 + y / 4) % 2 == 0 {
                Rgb::new(255, 255, 255)
            } else {
                Rgb::new(0, 0, 0)
            });
        }
    }
    let image = Image::from_pixels(16, 16, pixels).expect("16x16");
    let grid = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    assert_eq!(grid.len(), 256);
    for (_, id) in grid.iter() {
        assert!(palette.get(*id).is_some(), "{id} is not in the palette");
    }
}

#[test]
fn both_dither_directions_are_deterministic() {
    let palette = Palette::generic_5mm();
    let mut pixels = Vec::new();
    for y in 0..24u32 {
        for x in 0..24u32 {
            pixels.push(Rgb::new((x * 9) as u8, (y * 7) as u8, ((x + y) * 5) as u8));
        }
    }
    let image = Image::from_pixels(24, 24, pixels).expect("24x24");

    for dither in [Dither::FloydSteinberg, Dither::FloydSteinbergSerpentine] {
        let first = map_image(&image, &palette, MapOptions::with_dither(dither));
        let second = map_image(&image, &palette, MapOptions::with_dither(dither));
        assert_eq!(first, second, "{dither:?} is not reproducible");
    }
}

/// Serpentine and one-directional Floyd–Steinberg are both valid and both
/// supported, but on an asymmetric image they are not the same picture, so the
/// option is doing something.
#[test]
fn serpentine_differs_from_one_directional() {
    let palette = Palette::generic_5mm();
    let mut pixels = Vec::new();
    for y in 0..24u32 {
        for x in 0..24u32 {
            pixels.push(Rgb::new((100 + x * 3) as u8, (80 + y * 4) as u8, 160));
        }
    }
    let image = Image::from_pixels(24, 24, pixels).expect("24x24");
    let straight = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinberg),
    );
    let serpentine = map_image(
        &image,
        &palette,
        MapOptions::with_dither(Dither::FloydSteinbergSerpentine),
    );
    let differing = straight
        .iter()
        .zip(serpentine.iter())
        .filter(|((_, a), (_, b))| a != b)
        .count();
    assert!(differing > 0, "the two scan orders produced the same grid");
}

#[test]
fn dither_none_is_the_default_and_maps_pixels_independently() {
    let palette = Palette::generic_5mm();
    assert!(!Dither::default().is_enabled());
    assert!(Dither::FloydSteinberg.is_enabled());

    let mut pixels = Vec::new();
    for hex in ["#FEFEFE", "#040404", "#E30D30", "#0C62A5"] {
        pixels.push(Rgb::from_hex(hex).expect("hex"));
    }
    let image = Image::from_pixels(4, 1, pixels.clone()).expect("4x1");
    let grid = map_image(&image, &palette, MapOptions::default());

    let got: Vec<&str> = grid
        .iter()
        .map(|(_, id)| palette.color(*id).code.as_str())
        .collect();
    assert_eq!(got, vec!["G01", "G06", "G15", "G33"]);
    for (pixel, code) in pixels.iter().zip(&got) {
        assert_eq!(palette.color(map_pixel(*pixel, &palette)).code, *code);
    }
}
