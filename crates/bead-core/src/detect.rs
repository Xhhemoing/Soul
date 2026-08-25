//! Is this a pixel drawing or a photograph?
//!
//! The answer changes the rest of the pipeline: pixel art should be read block
//! by block so the original grid survives, a photograph has to be resampled and
//! usually dithered. Getting it wrong is recoverable — the caller can override
//! — so this is three cheap rules over the raster and no vision model. Nothing
//! here opens a socket.
//!
//! The three signals:
//!
//! * **Block period.** Art exported at 4× has every 4×4 block flat. The largest
//!   `k` that divides both dimensions and leaves every `k`-aligned block a
//!   single colour is the strongest evidence there is.
//! * **Flat neighbours.** The share of edge-sharing pixel pairs that are
//!   *exactly* equal. Photographs have sensor noise and gradients, so this sits
//!   near zero even in a clear sky.
//! * **Palette size.** Unique colours against the square root of the pixel
//!   count. A drawing reuses a handful of colours no matter how big it is.

use crate::color::Rgb;
use crate::image::Image;

/// What the raster looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    /// Drawn on a grid. `block_size` is the detected pixel size, 1 when the art
    /// is already at native resolution.
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
    pub unique_colors: usize,
    /// Share of edge-sharing pixel pairs that are exactly equal, `0.0..=1.0`.
    pub flat_neighbor_ratio: f64,
    /// Largest detected flat block period, 1 when there is none.
    pub block_size: u32,
}

/// The weighted sum of the three signals crosses this to mean "pixel art".
const DECISION_THRESHOLD: f64 = 0.5;

/// Largest upscale factor worth probing. Beyond 32× the block test starts
/// finding periods in large flat regions rather than in the art.
const MAX_BLOCK_PROBE: u32 = 32;

/// Classify a raster.
pub fn analyze(image: &Image) -> ImageAnalysis {
    let block_size = detect_block_size(image);
    let unique_colors = count_unique(image);
    let flat_neighbor_ratio = flat_neighbor_ratio(image);

    let total = (image.width() as f64) * (image.height() as f64);
    // Against the square root of the pixel count, not the count itself: a 16×16
    // sprite with 12 colours and a 512×512 one with 12 colours should read the
    // same, and dividing by the area would make only the large one look flat.
    let palette_signal = 1.0 - (unique_colors as f64 / total.sqrt().max(1.0)).clamp(0.0, 1.0);
    let block_signal = if block_size > 1 { 1.0 } else { 0.0 };

    let weighted = 0.45 * flat_neighbor_ratio + 0.35 * palette_signal + 0.20 * block_signal;

    let kind = if weighted >= DECISION_THRESHOLD {
        ImageKind::PixelArt { block_size }
    } else {
        ImageKind::Photo
    };
    // Distance from the boundary, normalised so that a unanimous verdict is 1.0
    // and a coin-flip is 0.0.
    let confidence = ((weighted - DECISION_THRESHOLD).abs() / DECISION_THRESHOLD).clamp(0.0, 1.0);

    ImageAnalysis {
        kind,
        confidence,
        unique_colors,
        flat_neighbor_ratio,
        block_size,
    }
}

/// The largest `k` in `2..=MAX_BLOCK_PROBE` that divides both dimensions and
/// leaves every `k`-aligned block one flat colour. 1 when there is no such `k`.
pub fn detect_block_size(image: &Image) -> u32 {
    let limit = MAX_BLOCK_PROBE.min(image.width()).min(image.height());
    let mut best = 1;
    for k in 2..=limit {
        if image.width() % k != 0 || image.height() % k != 0 {
            continue;
        }
        if blocks_are_flat(image, k) {
            best = k;
        }
    }
    best
}

fn blocks_are_flat(image: &Image, k: u32) -> bool {
    let mut by = 0;
    while by < image.height() {
        let mut bx = 0;
        while bx < image.width() {
            let anchor = image.clamped(i64::from(bx), i64::from(by));
            for y in by..by + k {
                for x in bx..bx + k {
                    if image.clamped(i64::from(x), i64::from(y)) != anchor {
                        return false;
                    }
                }
            }
            bx += k;
        }
        by += k;
    }
    true
}

fn count_unique(image: &Image) -> usize {
    let mut seen: std::collections::HashSet<Rgb> = std::collections::HashSet::new();
    for pixel in image.as_slice() {
        seen.insert(*pixel);
    }
    seen.len()
}

/// Share of horizontally and vertically adjacent pixel pairs that are equal.
pub fn flat_neighbor_ratio(image: &Image) -> f64 {
    let width = i64::from(image.width());
    let height = i64::from(image.height());
    let mut pairs = 0u64;
    let mut equal = 0u64;
    for y in 0..height {
        for x in 0..width {
            let here = image.clamped(x, y);
            if x + 1 < width {
                pairs += 1;
                if here == image.clamped(x + 1, y) {
                    equal += 1;
                }
            }
            if y + 1 < height {
                pairs += 1;
                if here == image.clamped(x, y + 1) {
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

    #[test]
    fn a_flat_image_has_every_neighbour_equal() {
        let image = Image::filled(4, 4, Rgb::new(1, 2, 3)).expect("4x4");
        assert!((flat_neighbor_ratio(&image) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_checkerboard_has_no_equal_neighbours() {
        let mut pixels = Vec::new();
        for y in 0..4u32 {
            for x in 0..4u32 {
                pixels.push(if (x + y) % 2 == 0 {
                    Rgb::new(0, 0, 0)
                } else {
                    Rgb::new(255, 255, 255)
                });
            }
        }
        let image = Image::from_pixels(4, 4, pixels).expect("4x4");
        assert!(flat_neighbor_ratio(&image) < 1e-12);
        assert_eq!(detect_block_size(&image), 1);
    }
}
