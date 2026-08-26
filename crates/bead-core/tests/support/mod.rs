//! Shared helpers for the integration tests.
//!
//! Not every test file uses every helper, so each one is allowed to be unused.

#![allow(dead_code)]

use bead_core::color::{Rgb, Rgba};
use bead_core::image::Image;
use bead_core::palette::{ColorId, Palette};
use bead_core::quantize::PatternGrid;
use bead_core::Grid;

/// A linear congruential generator, so "random" inputs are the same random
/// inputs on every machine and in every run.
///
/// The constants are Numerical Recipes'. Nothing here needs statistical
/// quality; it needs to be reproducible and to not be a dependency.
pub struct Lcg {
    state: u32,
}

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u32() >> 16) as u8
    }

    /// A value in `0..bound`.
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        self.next_u32() % bound
    }
}

/// Repeat every pixel `factor` times on both axes, the way an art tool exports
/// a sprite at 4×.
pub fn upscale(image: &Image, factor: u32) -> Image {
    let width = image.width() * factor;
    let height = image.height() * factor;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(image.sample(i64::from(x / factor), i64::from(y / factor)));
        }
    }
    Image::from_pixels(width, height, pixels).expect("upscaled")
}

/// An opaque image from RGB triples.
pub fn opaque(width: u32, height: u32, pixels: &[Rgb]) -> Image {
    Image::from_rgb(width, height, pixels).expect("opaque image")
}

/// A pattern grid from palette codes, where an empty string is an empty cell.
pub fn grid_from_codes(palette: &Palette, width: u32, rows: &[&str]) -> PatternGrid {
    let mut cells: Vec<Option<ColorId>> = Vec::new();
    for row in rows {
        for code in row.split_whitespace() {
            cells.push(if code == "." {
                None
            } else {
                Some(palette.find_code(code).unwrap_or_else(|| {
                    panic!("`{code}` is not in palette `{}`", palette.namespace())
                }))
            });
        }
    }
    let height = cells.len() as u32 / width.max(1);
    Grid::from_vec(width, height, cells).expect("a rectangular grid")
}

/// A solid rectangle of one colour with an optional transparent hole.
pub fn ring(width: u32, height: u32, hole: Option<(u32, u32, u32, u32)>, color: Rgba) -> Image {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let inside = hole
                .is_some_and(|(hx, hy, hw, hh)| x >= hx && x < hx + hw && y >= hy && y < hy + hh);
            pixels.push(if inside { Rgba::TRANSPARENT } else { color });
        }
    }
    Image::from_pixels(width, height, pixels).expect("ring")
}
