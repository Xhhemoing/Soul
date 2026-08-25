//! Turning a raster into a grid of bead colours, with optional dithering.
//!
//! The output is a [`PatternGrid`]: one optional colour per cell, `None` where
//! the source pixel was transparent (G1). Empty cells are not beads, so nothing
//! downstream counts them, orders them or places them.
//!
//! Two paths, one nearest-colour rule. Without dithering each pixel is mapped
//! independently. With Floyd–Steinberg the quantisation error is pushed into
//! the neighbours that have not been decided yet, at 7/16 right, 3/16 down and
//! left, 5/16 down, 1/16 down and right.
//!
//! G7 pins the details that would otherwise differ between this and the browser
//! port:
//!
//! * classic raster order — left to right, top to bottom, no serpentine;
//! * error accumulates in gamma-encoded sRGB code values as `f64`, and is *not*
//!   clamped as it accumulates;
//! * the working value is clamped to `0..=255` only for the palette lookup, so
//!   an out-of-gamut accumulation still quantises to the nearest bead rather
//!   than wrapping;
//! * error is not diffused outside the grid, and it is neither given to nor
//!   relayed through an empty cell.
//!
//! The nearest-colour decision itself still goes through Lab and CIEDE2000.

use std::collections::HashMap;

use crate::color::{Rgb, Rgba};
use crate::grid::Grid;
use crate::image::Image;
use crate::palette::{ColorId, Palette};

/// A bead pattern: one colour per cell, `None` where there is no bead.
pub type PatternGrid = Grid<Option<ColorId>>;

/// Whether to diffuse quantisation error into neighbouring pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dither {
    /// Map each pixel on its own.
    #[default]
    None,
    /// Floyd–Steinberg in classic raster order.
    FloydSteinberg,
}

impl Dither {
    pub fn is_enabled(self) -> bool {
        matches!(self, Dither::FloydSteinberg)
    }
}

/// How to map a raster onto a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapOptions {
    pub dither: Dither,
}

impl MapOptions {
    pub fn with_dither(dither: Dither) -> Self {
        Self { dither }
    }
}

/// The Floyd–Steinberg kernel: `(dx, dy, weight)`, in scan order.
pub const FLOYD_STEINBERG_KERNEL: [(i64, i64, f64); 4] = [
    (1, 0, 7.0 / 16.0),
    (-1, 1, 3.0 / 16.0),
    (0, 1, 5.0 / 16.0),
    (1, 1, 1.0 / 16.0),
];

/// A mapping, plus how close each decision was.
#[derive(Debug, Clone, PartialEq)]
pub struct MapTrace {
    pub grid: PatternGrid,
    /// How much closer the chosen bead was than the runner-up, per cell in
    /// reading order. `None` for empty cells, infinite for a one-colour
    /// palette. T-PAR-3 uses this to reject a fixture whose answer could flip
    /// on a last-place floating-point difference between Rust and JavaScript.
    pub margins: Vec<Option<f64>>,
}

/// Map every pixel of `image` to its nearest palette entry.
pub fn map_image(image: &Image, palette: &Palette, options: MapOptions) -> PatternGrid {
    map_image_traced(image, palette, options).grid
}

/// As [`map_image`], also reporting how close each decision was.
pub fn map_image_traced(image: &Image, palette: &Palette, options: MapOptions) -> MapTrace {
    match options.dither {
        Dither::None => map_direct(image, palette),
        Dither::FloydSteinberg => map_dithered(image, palette),
    }
}

/// Map one pixel, without any error state. `None` when it is not a bead.
pub fn map_pixel(pixel: Rgba, palette: &Palette) -> Option<ColorId> {
    palette.nearest_rgba(pixel).map(|m| m.id)
}

fn map_direct(image: &Image, palette: &Palette) -> MapTrace {
    let mut memo: HashMap<Rgb, (ColorId, f64)> = HashMap::new();
    let mut margins = Vec::with_capacity(image.as_slice().len());
    let grid = image.grid().map(|_, pixel| match pixel.rgb() {
        Some(rgb) => {
            let (id, margin) = *memo.entry(rgb).or_insert_with(|| {
                (
                    palette.nearest(rgb).id,
                    palette.decision_margin(rgb.to_lab()),
                )
            });
            margins.push(Some(margin));
            Some(id)
        }
        None => {
            margins.push(None);
            None
        }
    });
    MapTrace { grid, margins }
}

fn map_dithered(image: &Image, palette: &Palette) -> MapTrace {
    let width = image.width() as usize;
    let height = image.height() as usize;

    // The working buffer is the whole image in f64 so that error pushed onto a
    // pixel is still there when the scan reaches it.
    let mut work: Vec<[f64; 3]> = image
        .as_slice()
        .iter()
        .map(|p| [f64::from(p.r), f64::from(p.g), f64::from(p.b)])
        .collect();
    let opaque: Vec<bool> = image.as_slice().iter().map(|p| p.is_opaque()).collect();
    let mut out: Vec<Option<ColorId>> = vec![None; width * height];
    let mut margins: Vec<Option<f64>> = vec![None; width * height];

    for y in 0..height {
        for x in 0..width {
            let here = y * width + x;
            if !opaque[here] {
                // An empty cell places nothing and, having quantised nothing,
                // has no error to pass on. Whatever landed on it stops there.
                continue;
            }
            let current = work[here];
            let clamped = Rgb::new(
                clamp_channel(current[0]),
                clamp_channel(current[1]),
                clamp_channel(current[2]),
            );
            let chosen = palette.nearest(clamped).id;
            out[here] = Some(chosen);
            margins[here] = Some(palette.decision_margin(clamped.to_lab()));

            let placed = palette.color(chosen).rgb;
            let error = [
                current[0] - f64::from(placed.r),
                current[1] - f64::from(placed.g),
                current[2] - f64::from(placed.b),
            ];

            for (dx, dy, weight) in FLOYD_STEINBERG_KERNEL {
                let nx = x as i64 + dx;
                let ny = y as i64 + dy;
                if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                    continue;
                }
                let target = ny as usize * width + nx as usize;
                if !opaque[target] {
                    continue;
                }
                for (channel, residual) in error.iter().enumerate() {
                    work[target][channel] += residual * weight;
                }
            }
        }
    }

    MapTrace {
        grid: Grid::from_vec(image.width(), image.height(), out)
            .expect("the output has exactly one cell per pixel"),
        margins,
    }
}

fn clamp_channel(value: f64) -> u8 {
    crate::color::to_channel(value)
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn the_kernel_sums_to_one() {
        let total: f64 = FLOYD_STEINBERG_KERNEL.iter().map(|(_, _, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-12);
    }

    #[test]
    fn clamping_saturates_rather_than_wrapping() {
        assert_eq!(clamp_channel(-40.0), 0);
        assert_eq!(clamp_channel(300.0), 255);
        assert_eq!(clamp_channel(127.6), 128);
    }
}
