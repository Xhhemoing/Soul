//! A decoded raster, the input side of the whole pipeline.
//!
//! Decoding PNG or JPEG is somebody else's job: this crate takes pixels that
//! are already 8-bit sRGB so that it stays free of image-codec dependencies and
//! so the browser port in WP-B03 can hand over an `ImageData` buffer unchanged.

use std::fmt;

use crate::color::Rgb;
use crate::grid::{Cell, Grid, GridError};

/// A width × height buffer of 8-bit sRGB pixels, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pixels: Grid<Rgb>,
}

impl Image {
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<Rgb>) -> Result<Self, GridError> {
        Ok(Self {
            pixels: Grid::from_vec(width, height, pixels)?,
        })
    }

    /// Build from interleaved RGB bytes, three per pixel.
    pub fn from_rgb8(width: u32, height: u32, bytes: &[u8]) -> Result<Self, ImageError> {
        let expected = usize::try_from(u64::from(width) * u64::from(height) * 3).unwrap_or(0);
        if bytes.len() != expected {
            return Err(ImageError::ByteCount {
                expected,
                got: bytes.len(),
            });
        }
        let pixels = bytes
            .chunks_exact(3)
            .map(|c| Rgb::new(c[0], c[1], c[2]))
            .collect();
        Self::from_pixels(width, height, pixels).map_err(ImageError::Grid)
    }

    pub fn filled(width: u32, height: u32, color: Rgb) -> Result<Self, GridError> {
        Ok(Self {
            pixels: Grid::filled(width, height, color)?,
        })
    }

    pub fn width(&self) -> u32 {
        self.pixels.width()
    }

    pub fn height(&self) -> u32 {
        self.pixels.height()
    }

    pub fn pixel(&self, cell: Cell) -> Option<Rgb> {
        self.pixels.get(cell).copied()
    }

    /// The pixel at `(x, y)`, clamped to the edges. Sampling never falls off.
    pub fn clamped(&self, x: i64, y: i64) -> Rgb {
        let cx = x.clamp(0, i64::from(self.width()) - 1) as u32;
        let cy = y.clamp(0, i64::from(self.height()) - 1) as u32;
        self.pixels
            .get(Cell::new(cx, cy))
            .copied()
            .unwrap_or_default()
    }

    pub fn grid(&self) -> &Grid<Rgb> {
        &self.pixels
    }

    pub fn as_slice(&self) -> &[Rgb] {
        self.pixels.as_slice()
    }
}

/// A raster that could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    ByteCount { expected: usize, got: usize },
    Grid(GridError),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::ByteCount { expected, got } => {
                write!(f, "expected {expected} RGB bytes, got {got}")
            }
            ImageError::Grid(inner) => write!(f, "{inner}"),
        }
    }
}

impl std::error::Error for ImageError {}

impl From<GridError> for ImageError {
    fn from(value: GridError) -> Self {
        ImageError::Grid(value)
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn rejects_a_short_byte_buffer() {
        let error = Image::from_rgb8(2, 2, &[0; 9]).unwrap_err();
        assert_eq!(
            error,
            ImageError::ByteCount {
                expected: 12,
                got: 9
            }
        );
    }

    #[test]
    fn clamps_outside_sampling() {
        let mut pixels = vec![Rgb::new(0, 0, 0); 4];
        pixels[3] = Rgb::new(9, 9, 9);
        let image = Image::from_pixels(2, 2, pixels).expect("2x2");
        assert_eq!(image.clamped(-5, -5), Rgb::new(0, 0, 0));
        assert_eq!(image.clamped(99, 99), Rgb::new(9, 9, 9));
    }
}
