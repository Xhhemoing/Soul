//! Deciding how many bead cells an image becomes, and which part of it is kept.
//!
//! Three framing modes, matching the three the product asks for:
//!
//! * [`FitMode::FixedBoards`] — "I own two 28×28 boards, make it fit those."
//!   The bead count is given and the image is centre-cropped to that shape.
//! * [`FitMode::AspectFit`] — "use up to this many boards, keep my
//!   proportions." The whole image is used; the cell count is the largest that
//!   fits inside the budget at the image's own aspect ratio, never below 1×1.
//! * [`FitMode::ScaleCrop`] — a fixed viewport over a crop the user positioned
//!   by hand. A crop that hangs over the edge is allowed and comes back
//!   transparent there; a crop entirely off the image is a typed error.
//!
//! A [`FitPlan`] is only arithmetic: which rectangle of source pixels maps onto
//! which grid of cells. [`render`] is the separate step that samples.
//!
//! ## Why the sampler is pinned this hard
//!
//! G5: the filter and the colour space it averages in both change the output
//! colour codes, and a browser's `drawImage` would pick different ones. So both
//! implementations hand-write the same box filter, average in **linear light**,
//! and round half away from zero. T-SCL-5 is the tell-tale: a black and a white
//! pixel averaged into one cell give 188 in linear light and 128 in sRGB code
//! values, so one fixture distinguishes the two choices for good.

use std::fmt;

use crate::color::{srgb_compress, srgb_expand, to_channel, Rgba};
use crate::image::Image;

/// A physical pegboard, in bead cells. Rectangular for v0; the hexagonal and
/// round boards in the plan need a cell mask rather than a second size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardSpec {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

impl BoardSpec {
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            name: name.into(),
            width,
            height,
        }
    }

    /// The small square board, 28×28.
    pub fn square_28() -> Self {
        Self::new("28x28", 28, 28)
    }

    /// The large square board, 56×56.
    pub fn square_56() -> Self {
        Self::new("56x56", 56, 56)
    }

    pub fn cells(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// The rectangle of source pixels that ends up on the boards, in pixel units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl SourceRect {
    pub fn aspect(&self) -> f64 {
        self.width / self.height
    }

    /// Whether any part of this rectangle overlaps a `source_width` ×
    /// `source_height` image.
    pub fn overlaps(&self, source_width: u32, source_height: u32) -> bool {
        self.width > 0.0
            && self.height > 0.0
            && self.x < f64::from(source_width)
            && self.y < f64::from(source_height)
            && self.x + self.width > 0.0
            && self.y + self.height > 0.0
    }
}

/// A framing decision: what to sample, and onto what.
#[derive(Debug, Clone, PartialEq)]
pub struct FitPlan {
    pub board: BoardSpec,
    pub cells_wide: u32,
    pub cells_high: u32,
    /// Boards needed to hold the result, rounded up. A 28×14 pattern still
    /// occupies one 28×28 board.
    pub boards_across: u32,
    pub boards_down: u32,
    pub source: SourceRect,
    /// Share of the source image left outside `source`, `0.0..=1.0`.
    pub cropped_fraction: f64,
}

impl FitPlan {
    pub fn total_cells(&self) -> u64 {
        u64::from(self.cells_wide) * u64::from(self.cells_high)
    }

    pub fn board_count(&self) -> u32 {
        self.boards_across * self.boards_down
    }
}

/// The three framing modes.
#[derive(Debug, Clone, PartialEq)]
pub enum FitMode {
    /// A bead count the user already owns the boards for.
    FixedBoards {
        board: BoardSpec,
        cols: u32,
        rows: u32,
    },
    /// The image's own proportions, inside a board budget.
    AspectFit {
        board: BoardSpec,
        max_cols: u32,
        max_rows: u32,
    },
    /// A fixed viewport over a hand-positioned crop.
    ScaleCrop {
        board: BoardSpec,
        cols: u32,
        rows: u32,
        /// Source pixels per bead cell. This is the contract's `scale`, named
        /// for its units because "zoom" is ambiguous about which way it goes.
        pixels_per_cell: f64,
        /// Top-left of the crop, in source pixels. May be negative.
        crop_x: f64,
        crop_y: f64,
    },
}

/// The framing could not be worked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitError {
    /// A source dimension was zero (G8).
    InvalidDimensions { width: u32, height: u32 },
    /// A board dimension, or a board count, was zero.
    EmptyBoard,
    /// `pixels_per_cell` was zero, negative or not finite.
    BadScale,
    /// The crop does not overlap the image at all.
    CropOutsideImage,
}

impl fmt::Display for FitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FitError::InvalidDimensions { width, height } => {
                write!(f, "a {width}x{height} image has no pixels")
            }
            FitError::EmptyBoard => f.write_str("a board layout needs a non-zero size and count"),
            FitError::BadScale => f.write_str("pixels per cell must be a finite number above zero"),
            FitError::CropOutsideImage => f.write_str("the crop lies entirely outside the image"),
        }
    }
}

impl std::error::Error for FitError {}

/// Work out the framing for one mode.
pub fn plan(source_width: u32, source_height: u32, mode: &FitMode) -> Result<FitPlan, FitError> {
    if source_width == 0 || source_height == 0 {
        return Err(FitError::InvalidDimensions {
            width: source_width,
            height: source_height,
        });
    }
    match mode {
        FitMode::FixedBoards { board, cols, rows } => {
            fixed_boards(source_width, source_height, board, *cols, *rows)
        }
        FitMode::AspectFit {
            board,
            max_cols,
            max_rows,
        } => aspect_fit(source_width, source_height, board, *max_cols, *max_rows),
        FitMode::ScaleCrop {
            board,
            cols,
            rows,
            pixels_per_cell,
            crop_x,
            crop_y,
        } => scale_crop(
            source_width,
            source_height,
            board,
            *cols,
            *rows,
            *pixels_per_cell,
            *crop_x,
            *crop_y,
        ),
    }
}

/// A given board rectangle, centre-cropped to its shape.
pub fn fixed_boards(
    source_width: u32,
    source_height: u32,
    board: &BoardSpec,
    cols: u32,
    rows: u32,
) -> Result<FitPlan, FitError> {
    check_dimensions(source_width, source_height)?;
    check_board(board, cols, rows)?;
    let cells_wide = board.width * cols;
    let cells_high = board.height * rows;
    let source = cover_rect(
        source_width,
        source_height,
        f64::from(cells_wide) / f64::from(cells_high),
    );
    Ok(finish(
        source_width,
        source_height,
        board.clone(),
        cells_wide,
        cells_high,
        source,
    ))
}

/// The image's own proportions, scaled to fit inside the board budget.
///
/// The whole image is used, so `cropped_fraction` is zero: keeping the
/// proportions is the entire point of this mode, and cropping to make the cell
/// count land on a whole board would defeat it. Each dimension is rounded half
/// away from zero and never falls below one, so a 3000×1 panorama comes out
/// 28×1 rather than 28×0.
pub fn aspect_fit(
    source_width: u32,
    source_height: u32,
    board: &BoardSpec,
    max_cols: u32,
    max_rows: u32,
) -> Result<FitPlan, FitError> {
    check_dimensions(source_width, source_height)?;
    check_board(board, max_cols, max_rows)?;

    let budget_w = f64::from(board.width * max_cols);
    let budget_h = f64::from(board.height * max_rows);
    let scale = (budget_w / f64::from(source_width)).min(budget_h / f64::from(source_height));

    let cells_wide = round_cells(f64::from(source_width) * scale);
    let cells_high = round_cells(f64::from(source_height) * scale);

    let source = SourceRect {
        x: 0.0,
        y: 0.0,
        width: f64::from(source_width),
        height: f64::from(source_height),
    };
    Ok(finish(
        source_width,
        source_height,
        board.clone(),
        cells_wide,
        cells_high,
        source,
    ))
}

/// A fixed viewport over a hand-positioned crop.
#[allow(clippy::too_many_arguments)]
pub fn scale_crop(
    source_width: u32,
    source_height: u32,
    board: &BoardSpec,
    cols: u32,
    rows: u32,
    pixels_per_cell: f64,
    crop_x: f64,
    crop_y: f64,
) -> Result<FitPlan, FitError> {
    check_dimensions(source_width, source_height)?;
    check_board(board, cols, rows)?;
    if !pixels_per_cell.is_finite() || pixels_per_cell <= 0.0 {
        return Err(FitError::BadScale);
    }
    if !crop_x.is_finite() || !crop_y.is_finite() {
        return Err(FitError::BadScale);
    }

    let cells_wide = board.width * cols;
    let cells_high = board.height * rows;
    let source = SourceRect {
        x: crop_x,
        y: crop_y,
        width: f64::from(cells_wide) * pixels_per_cell,
        height: f64::from(cells_high) * pixels_per_cell,
    };
    if !source.overlaps(source_width, source_height) {
        return Err(FitError::CropOutsideImage);
    }
    Ok(finish(
        source_width,
        source_height,
        board.clone(),
        cells_wide,
        cells_high,
        source,
    ))
}

fn check_dimensions(width: u32, height: u32) -> Result<(), FitError> {
    if width == 0 || height == 0 {
        return Err(FitError::InvalidDimensions { width, height });
    }
    Ok(())
}

fn check_board(board: &BoardSpec, cols: u32, rows: u32) -> Result<(), FitError> {
    if board.width == 0 || board.height == 0 || cols == 0 || rows == 0 {
        return Err(FitError::EmptyBoard);
    }
    Ok(())
}

/// Round half away from zero, then hold at one. Pinned by T-SCL-2.
fn round_cells(value: f64) -> u32 {
    let rounded = value.round();
    if !rounded.is_finite() || rounded < 1.0 {
        return 1;
    }
    rounded.min(f64::from(u32::MAX)) as u32
}

/// The largest centred rectangle of the given aspect that fits in the source.
fn cover_rect(source_width: u32, source_height: u32, target_aspect: f64) -> SourceRect {
    let w = f64::from(source_width);
    let h = f64::from(source_height);
    if w / h > target_aspect {
        let width = h * target_aspect;
        SourceRect {
            x: (w - width) / 2.0,
            y: 0.0,
            width,
            height: h,
        }
    } else {
        let height = w / target_aspect;
        SourceRect {
            x: 0.0,
            y: (h - height) / 2.0,
            width: w,
            height,
        }
    }
}

fn finish(
    source_width: u32,
    source_height: u32,
    board: BoardSpec,
    cells_wide: u32,
    cells_high: u32,
    source: SourceRect,
) -> FitPlan {
    let whole = f64::from(source_width) * f64::from(source_height);
    // Only the part of the crop that is actually over the image counts as kept;
    // a crop hanging over the edge does not preserve the pixels that are not
    // there.
    let overlap_w = (source.x + source.width).min(f64::from(source_width)) - source.x.max(0.0);
    let overlap_h = (source.y + source.height).min(f64::from(source_height)) - source.y.max(0.0);
    let kept = (overlap_w.max(0.0) * overlap_h.max(0.0)).clamp(0.0, whole);
    FitPlan {
        cells_wide,
        cells_high,
        boards_across: cells_wide.div_ceil(board.width.max(1)),
        boards_down: cells_high.div_ceil(board.height.max(1)),
        board,
        source,
        cropped_fraction: 1.0 - kept / whole,
    }
}

/// How to read source pixels when the cell grid is coarser than the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sampling {
    /// One source pixel per cell, taken at the cell's centre. Right for pixel
    /// art, where averaging would invent colours that are not in the drawing.
    Nearest,
    /// Mean of every source pixel the cell covers, in linear light. Right for
    /// photographs.
    #[default]
    BoxAverage,
}

/// Sample `image` through `plan` into a cell-resolution raster.
///
/// Cells that fall outside the image come back transparent, which is what makes
/// an over-hanging crop legal: the missing part becomes empty cells rather than
/// a smeared edge colour.
pub fn render(image: &Image, plan: &FitPlan, sampling: Sampling) -> Image {
    let cells_wide = plan.cells_wide;
    let cells_high = plan.cells_high;
    let cell_w = plan.source.width / f64::from(cells_wide);
    let cell_h = plan.source.height / f64::from(cells_high);

    let mut pixels = Vec::with_capacity(cells_wide as usize * cells_high as usize);
    for cy in 0..cells_high {
        for cx in 0..cells_wide {
            let x0 = plan.source.x + f64::from(cx) * cell_w;
            let y0 = plan.source.y + f64::from(cy) * cell_h;
            pixels.push(match sampling {
                Sampling::Nearest => image.sample(
                    (x0 + cell_w / 2.0).floor() as i64,
                    (y0 + cell_h / 2.0).floor() as i64,
                ),
                Sampling::BoxAverage => box_average(image, x0, y0, cell_w, cell_h),
            });
        }
    }
    Image::from_pixels(cells_wide, cells_high, pixels).expect("one pixel per cell")
}

/// Mean of the covered pixels, with colour averaged in linear light.
///
/// Alpha is averaged over every covered sample and then re-thresholded, but
/// colour is averaged over the opaque samples only. Letting a transparent pixel
/// contribute its colour would drag every edge cell towards whatever the
/// buffer happens to hold behind the alpha, which is usually black.
fn box_average(image: &Image, x0: f64, y0: f64, cell_w: f64, cell_h: f64) -> Rgba {
    let first_x = x0.floor() as i64;
    let first_y = y0.floor() as i64;
    let last_x = ((x0 + cell_w).ceil() as i64 - 1).max(first_x);
    let last_y = ((y0 + cell_h).ceil() as i64 - 1).max(first_y);

    let mut linear = [0f64; 3];
    let mut opaque_count = 0f64;
    let mut alpha_sum = 0f64;
    let mut total = 0f64;

    for y in first_y..=last_y {
        for x in first_x..=last_x {
            let pixel = image.sample(x, y);
            total += 1.0;
            alpha_sum += f64::from(pixel.a);
            if pixel.is_opaque() {
                opaque_count += 1.0;
                linear[0] += srgb_expand(f64::from(pixel.r) / 255.0);
                linear[1] += srgb_expand(f64::from(pixel.g) / 255.0);
                linear[2] += srgb_expand(f64::from(pixel.b) / 255.0);
            }
        }
    }

    if total == 0.0
        || opaque_count == 0.0
        || alpha_sum / total.max(1.0) < f64::from(crate::color::ALPHA_OPAQUE_THRESHOLD)
    {
        return Rgba::TRANSPARENT;
    }

    Rgba::new(
        to_channel(srgb_compress(linear[0] / opaque_count) * 255.0),
        to_channel(srgb_compress(linear[1] / opaque_count) * 255.0),
        to_channel(srgb_compress(linear[2] / opaque_count) * 255.0),
        255,
    )
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn a_square_source_on_a_square_board_is_not_cropped() {
        let plan = fixed_boards(100, 100, &BoardSpec::square_28(), 1, 1).expect("plan");
        assert_eq!((plan.cells_wide, plan.cells_high), (28, 28));
        assert!(plan.cropped_fraction.abs() < 1e-12);
    }

    #[test]
    fn a_zero_board_count_is_refused() {
        assert_eq!(
            fixed_boards(10, 10, &BoardSpec::square_28(), 0, 1).unwrap_err(),
            FitError::EmptyBoard
        );
    }

    #[test]
    fn rounding_holds_at_one_cell() {
        assert_eq!(round_cells(0.004), 1);
        assert_eq!(round_cells(0.5), 1);
        assert_eq!(round_cells(1.4), 1);
        assert_eq!(round_cells(1.5), 2);
        assert_eq!(round_cells(f64::NAN), 1);
    }
}
