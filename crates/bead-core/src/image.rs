//! A decoded raster, the input side of the whole pipeline.
//!
//! Decoding PNG or JPEG is somebody else's job: this crate takes pixels that
//! are already 8-bit sRGB with alpha, so that it stays free of image-codec
//! dependencies and so the browser port in WP-B03 can hand over an `ImageData`
//! buffer unchanged. T-PAR-2 makes that mandatory rather than merely
//! convenient — a decoder that applies an ICC profile on one side and not the
//! other breaks the shared fixture before any of this code runs.
//!
//! Alpha follows G1: at or above [`ALPHA_OPAQUE_THRESHOLD`] the pixel is a
//! bead, below it the cell is empty.

use std::fmt;

use crate::color::{Rgb, Rgba, ALPHA_OPAQUE_THRESHOLD};
use crate::grid::{Cell, Grid, GridError};

/// A width × height buffer of 8-bit sRGB pixels with alpha, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pixels: Grid<Rgba>,
}

impl Image {
    /// Build from RGBA pixels. Both dimensions must be non-zero: G8 puts the
    /// image pipeline's refusal here, where it can be a typed error, rather
    /// than letting a zero-sized raster wander into the sampler.
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<Rgba>) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions { width, height });
        }
        Ok(Self {
            pixels: Grid::from_vec(width, height, pixels)?,
        })
    }

    /// Build from opaque RGB pixels.
    pub fn from_rgb(width: u32, height: u32, pixels: &[Rgb]) -> Result<Self, ImageError> {
        Self::from_pixels(
            width,
            height,
            pixels.iter().copied().map(Rgba::opaque).collect(),
        )
    }

    /// Build from interleaved RGBA bytes, four per pixel. This is the shape a
    /// browser `ImageData` arrives in.
    pub fn from_rgba8(width: u32, height: u32, bytes: &[u8]) -> Result<Self, ImageError> {
        let expected = usize::try_from(u64::from(width) * u64::from(height) * 4).unwrap_or(0);
        if bytes.len() != expected {
            return Err(ImageError::ByteCount {
                expected,
                got: bytes.len(),
            });
        }
        let pixels = bytes
            .chunks_exact(4)
            .map(|c| Rgba::new(c[0], c[1], c[2], c[3]))
            .collect();
        Self::from_pixels(width, height, pixels)
    }

    pub fn filled(width: u32, height: u32, color: Rgba) -> Result<Self, ImageError> {
        if width == 0 || height == 0 {
            return Err(ImageError::InvalidDimensions { width, height });
        }
        Ok(Self {
            pixels: Grid::filled(width, height, color),
        })
    }

    pub fn width(&self) -> u32 {
        self.pixels.width()
    }

    pub fn height(&self) -> u32 {
        self.pixels.height()
    }

    pub fn pixel(&self, cell: Cell) -> Option<Rgba> {
        self.pixels.get(cell).copied()
    }

    /// The pixel at `(x, y)`, or fully transparent outside the image.
    ///
    /// Sampling off the edge happens legitimately — a crop the user dragged
    /// past the border — and G1 already gives "nothing here" a representation,
    /// so there is no reason to invent an edge colour by clamping.
    pub fn sample(&self, x: i64, y: i64) -> Rgba {
        if x < 0 || y < 0 || x >= i64::from(self.width()) || y >= i64::from(self.height()) {
            return Rgba::TRANSPARENT;
        }
        self.pixels
            .get(Cell::new(x as u32, y as u32))
            .copied()
            .unwrap_or(Rgba::TRANSPARENT)
    }

    pub fn grid(&self) -> &Grid<Rgba> {
        &self.pixels
    }

    pub fn as_slice(&self) -> &[Rgba] {
        self.pixels.as_slice()
    }

    /// How many pixels count as beads.
    pub fn opaque_count(&self) -> usize {
        self.pixels
            .as_slice()
            .iter()
            .filter(|p| p.is_opaque())
            .count()
    }
}

/// A raster that could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    /// A dimension was zero (G8).
    InvalidDimensions {
        width: u32,
        height: u32,
    },
    /// The byte buffer was not `width * height * 4` long.
    ByteCount {
        expected: usize,
        got: usize,
    },
    Grid(GridError),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::InvalidDimensions { width, height } => {
                write!(f, "a {width}x{height} image has no pixels")
            }
            ImageError::ByteCount { expected, got } => {
                write!(f, "expected {expected} RGBA bytes, got {got}")
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

/// Re-exported so callers reading alpha do not have to reach into [`crate::color`].
pub const OPAQUE_AT: u8 = ALPHA_OPAQUE_THRESHOLD;

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn rejects_a_short_byte_buffer() {
        let error = Image::from_rgba8(2, 2, &[0; 12]).unwrap_err();
        assert_eq!(
            error,
            ImageError::ByteCount {
                expected: 16,
                got: 12
            }
        );
    }

    #[test]
    fn rejects_a_zero_dimension() {
        assert_eq!(
            Image::from_pixels(0, 4, Vec::new()).unwrap_err(),
            ImageError::InvalidDimensions {
                width: 0,
                height: 4
            }
        );
        assert_eq!(
            Image::filled(4, 0, Rgba::TRANSPARENT).unwrap_err(),
            ImageError::InvalidDimensions {
                width: 4,
                height: 0
            }
        );
    }

    #[test]
    fn sampling_outside_is_transparent() {
        let image = Image::filled(2, 2, Rgba::new(9, 9, 9, 255)).expect("2x2");
        assert_eq!(image.sample(-1, 0), Rgba::TRANSPARENT);
        assert_eq!(image.sample(0, 2), Rgba::TRANSPARENT);
        assert_eq!(image.sample(1, 1), Rgba::new(9, 9, 9, 255));
        assert_eq!(image.opaque_count(), 4);
    }
}
