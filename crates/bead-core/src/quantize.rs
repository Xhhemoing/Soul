//! Turning a raster into a grid of bead colours, with optional dithering.
//!
//! Two paths, one nearest-colour rule. Without dithering each pixel is mapped
//! independently. With Floyd–Steinberg the quantisation error is pushed into
//! the neighbours that have not been decided yet, at 7/16 right, 3/16 down and
//! left, 5/16 down, 1/16 down and right. Diffusion happens in gamma-encoded
//! sRGB because that is what every dither the user has seen — and every
//! `<canvas>` port of this code — operates on; the nearest-colour decision
//! itself still goes through Lab and CIEDE2000.
//!
//! Error accumulation can push a working value outside `0..=255`. It is clamped
//! before the lookup, so a colour outside the palette's gamut still quantises to
//! the nearest available bead rather than to nothing.

use std::collections::HashMap;

use crate::color::Rgb;
use crate::grid::Grid;
use crate::image::Image;
use crate::palette::{ColorId, Palette};

/// Whether to diffuse quantisation error into neighbouring pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dither {
    /// Map each pixel on its own.
    #[default]
    None,
    /// Floyd–Steinberg, left to right and top to bottom.
    FloydSteinberg,
    /// Floyd–Steinberg with alternating row direction, which hides the
    /// diagonal worming the one-directional variant leaves in flat areas.
    FloydSteinbergSerpentine,
}

impl Dither {
    pub fn is_enabled(self) -> bool {
        !matches!(self, Dither::None)
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

/// The Floyd–Steinberg kernel: `(dx, dy, numerator)` over a denominator of 16.
pub const FLOYD_STEINBERG_KERNEL: [(i64, i64, f64); 4] = [
    (1, 0, 7.0 / 16.0),
    (-1, 1, 3.0 / 16.0),
    (0, 1, 5.0 / 16.0),
    (1, 1, 1.0 / 16.0),
];

/// Map every pixel of `image` to its nearest palette entry.
pub fn map_image(image: &Image, palette: &Palette, options: MapOptions) -> Grid<ColorId> {
    match options.dither {
        Dither::None => map_direct(image, palette),
        Dither::FloydSteinberg => map_dithered(image, palette, false),
        Dither::FloydSteinbergSerpentine => map_dithered(image, palette, true),
    }
}

/// Map a single already-known colour, without any error state.
pub fn map_pixel(rgb: Rgb, palette: &Palette) -> ColorId {
    palette.nearest(rgb).id
}

fn map_direct(image: &Image, palette: &Palette) -> Grid<ColorId> {
    let mut memo: HashMap<Rgb, ColorId> = HashMap::new();
    image
        .grid()
        .map(|_, rgb| *memo.entry(*rgb).or_insert_with(|| palette.nearest(*rgb).id))
}

fn map_dithered(image: &Image, palette: &Palette, serpentine: bool) -> Grid<ColorId> {
    let width = image.width() as usize;
    let height = image.height() as usize;

    // The working buffer is the whole image in f64 so that error pushed onto a
    // pixel is still there when the scan reaches it.
    let mut work: Vec<[f64; 3]> = image
        .as_slice()
        .iter()
        .map(|p| [f64::from(p.r), f64::from(p.g), f64::from(p.b)])
        .collect();
    let mut out: Vec<ColorId> = Vec::with_capacity(width * height);
    out.resize(width * height, ColorId(0));

    for y in 0..height {
        let rightwards = !serpentine || y % 2 == 0;
        for step in 0..width {
            let x = if rightwards { step } else { width - 1 - step };
            let here = y * width + x;
            let current = work[here];
            let clamped = Rgb::new(
                clamp_channel(current[0]),
                clamp_channel(current[1]),
                clamp_channel(current[2]),
            );
            let chosen = palette.nearest(clamped).id;
            out[here] = chosen;

            let placed = palette.color(chosen).rgb;
            let error = [
                current[0] - f64::from(placed.r),
                current[1] - f64::from(placed.g),
                current[2] - f64::from(placed.b),
            ];

            for (dx, dy, weight) in FLOYD_STEINBERG_KERNEL {
                let dx = if rightwards { dx } else { -dx };
                let nx = x as i64 + dx;
                let ny = y as i64 + dy;
                if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
                    continue;
                }
                let target = ny as usize * width + nx as usize;
                for (channel, residual) in error.iter().enumerate() {
                    work[target][channel] += residual * weight;
                }
            }
        }
    }

    Grid::from_vec(image.width(), image.height(), out)
        .expect("the output has exactly one id per pixel")
}

fn clamp_channel(value: f64) -> u8 {
    if value.is_nan() {
        return 0;
    }
    value.round().clamp(0.0, 255.0) as u8
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
