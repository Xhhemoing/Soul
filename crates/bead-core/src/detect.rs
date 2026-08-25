//! Is this a pixel drawing or a photograph, and if it is a drawing, on what
//! grid?
//!
//! The answer changes the rest of the pipeline: pixel art should be read block
//! by block so the original grid survives, a photograph has to be resampled and
//! usually dithered. Getting it wrong is recoverable — the caller can override
//! — so this is three cheap rules over the raster and no vision model. Nothing
//! here opens a socket.
//!
//! ## The rules, in full
//!
//! G-contract: the browser port has to reach the same verdict on the same
//! bytes, because a different verdict means a different pipeline and therefore
//! a different colour sequence. So the features and the threshold are part of
//! the contract, not an implementation detail.
//!
//! Over the **opaque** pixels only:
//!
//! | signal | definition | weight |
//! |--------|------------|--------|
//! | flat neighbours | share of edge-sharing opaque pairs that are exactly equal | 0.45 |
//! | palette size | `1 − min(1, unique ÷ √(w·h))` | 0.35 |
//! | block period | 1 when [`detect_grid`] finds a cell larger than 1×1, else 0 | 0.20 |
//!
//! The weighted sum is compared with [`DECISION_THRESHOLD`]; at or above it the
//! image is [`ImageKind::PixelArt`]. [`ImageAnalysis::confidence`] is the
//! distance from that threshold, normalised so a unanimous verdict is 1.0 and a
//! coin toss is 0.0.
//!
//! Palette size is measured against the *square root* of the pixel count rather
//! than the count itself, so a 16×16 sprite with twelve colours and a 512×512
//! one with twelve colours read the same.
//!
//! Two degenerate inputs are pinned rather than computed (T-CLS-4): an image
//! with no opaque pixels, and one with a single colour, are both
//! `PixelArt { block_size: 1 }` at [`DEGENERATE_CONFIDENCE`]. Neither can be a
//! photograph of anything, and computing a ratio over zero pairs would be a
//! division by zero rather than an answer.

use std::collections::HashSet;

use crate::color::Rgb;
use crate::image::Image;

/// What the raster looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// Drawn on a grid. `block_size` is the detected pixel size, 1 when the art
    /// is already at native resolution or sits on a shifted grid that cannot be
    /// read by simple decimation.
    PixelArt { block_size: u32 },
    /// A photograph or anything else continuous-tone.
    Photo,
}

impl ImageKind {
    pub fn is_pixel_art(self) -> bool {
        matches!(self, ImageKind::PixelArt { .. })
    }

    /// The block size to read the image at: the detected period for pixel art,
    /// 1 for anything else.
    pub fn block_size(self) -> u32 {
        match self {
            ImageKind::PixelArt { block_size } => block_size,
            ImageKind::Photo => 1,
        }
    }
}

/// The verdict plus the measurements behind it, so a UI can explain itself and
/// a test can pin the reasoning rather than only the label.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageAnalysis {
    pub kind: ImageKind,
    /// How firmly the verdict is held, `0.0..=1.0`. A value near zero means the
    /// image sat on the boundary and the caller should offer the override.
    pub confidence: f64,
    /// Distinct colours among the opaque pixels.
    pub unique_colors: usize,
    /// Share of edge-sharing opaque pairs that are exactly equal, `0.0..=1.0`.
    pub flat_neighbor_ratio: f64,
    /// Square block period, 1 when there is none.
    pub block_size: u32,
    /// The full grid geometry, including a shifted phase.
    pub geometry: GridGeometry,
}

/// The weighted sum of the three signals crosses this to mean "pixel art".
pub const DECISION_THRESHOLD: f64 = 0.5;

/// Confidence reported for the two degenerate inputs T-CLS-4 pins: an image
/// with no opaque pixels, and an image with exactly one colour.
pub const DEGENERATE_CONFIDENCE: f64 = 1.0;

/// Largest cell size worth probing on either axis.
pub const MAX_CELL_PROBE: u32 = 64;

/// The cell lattice a drawing sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridGeometry {
    pub cell_width: u32,
    pub cell_height: u32,
    /// Where the first full cell starts, in pixels from the left edge.
    pub offset_x: u32,
    /// Where the first full cell starts, in pixels from the top edge.
    pub offset_y: u32,
}

impl GridGeometry {
    /// The 1×1 lattice: no periodicity found.
    pub const NONE: GridGeometry = GridGeometry {
        cell_width: 1,
        cell_height: 1,
        offset_x: 0,
        offset_y: 0,
    };

    pub fn is_square(self) -> bool {
        self.cell_width == self.cell_height
    }

    pub fn is_aligned(self) -> bool {
        self.offset_x == 0 && self.offset_y == 0
    }

    /// The period a caller can read the image at by plain decimation: a square,
    /// origin-aligned cell. 1 when the lattice is neither.
    pub fn block_size(self) -> u32 {
        if self.is_square() && self.is_aligned() {
            self.cell_width
        } else {
            1
        }
    }
}

/// Classify a raster.
pub fn analyze(image: &Image) -> ImageAnalysis {
    let unique_colors = count_unique(image);
    let flat_neighbor_ratio = flat_neighbor_ratio(image);

    if unique_colors <= 1 {
        return ImageAnalysis {
            kind: ImageKind::PixelArt { block_size: 1 },
            confidence: DEGENERATE_CONFIDENCE,
            unique_colors,
            flat_neighbor_ratio,
            block_size: 1,
            geometry: GridGeometry::NONE,
        };
    }

    let geometry = detect_grid(image);
    let block_size = geometry.block_size();

    let total = f64::from(image.width()) * f64::from(image.height());
    let palette_signal = 1.0 - (unique_colors as f64 / total.sqrt().max(1.0)).clamp(0.0, 1.0);
    let block_signal = if geometry.cell_width > 1 || geometry.cell_height > 1 {
        1.0
    } else {
        0.0
    };

    let weighted = 0.45 * flat_neighbor_ratio + 0.35 * palette_signal + 0.20 * block_signal;

    let kind = if weighted >= DECISION_THRESHOLD {
        ImageKind::PixelArt { block_size }
    } else {
        ImageKind::Photo
    };
    let confidence = ((weighted - DECISION_THRESHOLD).abs() / DECISION_THRESHOLD).clamp(0.0, 1.0);

    ImageAnalysis {
        kind,
        confidence,
        unique_colors,
        flat_neighbor_ratio,
        block_size,
        geometry,
    }
}

/// Find the cell lattice the image is drawn on, including its phase.
///
/// Each axis is solved on its own from the positions where one whole line
/// differs from the line before it. If the art is on a lattice those positions
/// are all congruent modulo the cell size, so the greatest common divisor of
/// their gaps *is* the cell size and the first of them gives the phase. Reading
/// it off the gaps rather than searching offsets means a missing boundary — two
/// neighbouring cells that happen to share a colour — costs nothing.
///
/// Fewer than two boundaries on an axis is not enough to establish a period,
/// and that axis comes back as 1.
pub fn detect_grid(image: &Image) -> GridGeometry {
    let width = image.width();
    let height = image.height();

    let column_changes: Vec<u32> = (1..width)
        .filter(|x| !column_equal(image, x - 1, *x))
        .collect();
    let row_changes: Vec<u32> = (1..height)
        .filter(|y| !row_equal(image, y - 1, *y))
        .collect();

    let (cell_width, offset_x) = period_of(&column_changes, width);
    let (cell_height, offset_y) = period_of(&row_changes, height);

    GridGeometry {
        cell_width,
        cell_height,
        offset_x,
        offset_y,
    }
}

/// `(cell size, phase)` from the boundary positions on one axis.
fn period_of(changes: &[u32], length: u32) -> (u32, u32) {
    if changes.len() < 2 {
        return (1, 0);
    }
    let first = changes[0];
    let mut spacing = 0u32;
    for position in &changes[1..] {
        spacing = gcd(spacing, position - first);
    }
    if spacing <= 1 {
        return (1, 0);
    }
    let limit = MAX_CELL_PROBE.min(length);
    let cell = largest_divisor_at_most(spacing, limit);
    if cell <= 1 {
        return (1, 0);
    }
    (cell, first % cell)
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn largest_divisor_at_most(value: u32, limit: u32) -> u32 {
    if value == 0 {
        return 1;
    }
    (1..=limit.min(value))
        .rev()
        .find(|candidate| value % candidate == 0)
        .unwrap_or(1)
}

/// Two columns are equal when every pair of pixels is the same bead, counting
/// every transparent pixel as the same nothing.
fn column_equal(image: &Image, left: u32, right: u32) -> bool {
    (0..image.height()).all(|y| {
        image.sample(i64::from(left), i64::from(y)).rgb()
            == image.sample(i64::from(right), i64::from(y)).rgb()
    })
}

fn row_equal(image: &Image, top: u32, bottom: u32) -> bool {
    (0..image.width()).all(|x| {
        image.sample(i64::from(x), i64::from(top)).rgb()
            == image.sample(i64::from(x), i64::from(bottom)).rgb()
    })
}

/// Distinct colours among the opaque pixels.
pub fn count_unique(image: &Image) -> usize {
    let mut seen: HashSet<Rgb> = HashSet::new();
    for pixel in image.as_slice() {
        if let Some(rgb) = pixel.rgb() {
            seen.insert(rgb);
        }
    }
    seen.len()
}

/// Share of edge-sharing pixel pairs that are equal, counting only pairs where
/// both pixels are opaque. Zero when there are no such pairs.
pub fn flat_neighbor_ratio(image: &Image) -> f64 {
    let width = i64::from(image.width());
    let height = i64::from(image.height());
    let mut pairs = 0u64;
    let mut equal = 0u64;
    for y in 0..height {
        for x in 0..width {
            let Some(here) = image.sample(x, y).rgb() else {
                continue;
            };
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx >= width || ny >= height {
                    continue;
                }
                let Some(there) = image.sample(nx, ny).rgb() else {
                    continue;
                };
                pairs += 1;
                if here == there {
                    equal += 1;
                }
            }
        }
    }
    if pairs == 0 {
        return 0.0;
    }
    equal as f64 / pairs as f64
}

#[cfg(test)]
mod unit {
    use super::*;
    use crate::color::Rgba;

    #[test]
    fn gcd_is_the_usual_one() {
        assert_eq!(gcd(0, 8), 8);
        assert_eq!(gcd(8, 12), 4);
        assert_eq!(gcd(7, 13), 1);
    }

    #[test]
    fn a_divisor_is_capped_but_still_divides() {
        assert_eq!(largest_divisor_at_most(96, 64), 48);
        assert_eq!(largest_divisor_at_most(8, 64), 8);
        assert_eq!(largest_divisor_at_most(7, 4), 1);
    }

    #[test]
    fn one_boundary_is_not_a_period() {
        assert_eq!(period_of(&[5], 100), (1, 0));
        assert_eq!(period_of(&[], 100), (1, 0));
    }

    #[test]
    fn a_checkerboard_has_no_equal_neighbours() {
        let mut pixels = Vec::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.push(if (x + y) % 2 == 0 {
                    Rgba::new(0, 0, 0, 255)
                } else {
                    Rgba::new(255, 255, 255, 255)
                });
            }
        }
        let image = Image::from_pixels(4, 4, pixels).expect("4x4");
        assert!(flat_neighbor_ratio(&image) < 1e-12);
        assert_eq!(detect_grid(&image), GridGeometry::NONE);
    }
}
