//! Deciding how many bead cells an image becomes, and which part of it is kept.
//!
//! Three framing modes, matching the three the product asks for:
//!
//! * [`FitMode::FixedBoards`] — "I own two 28×28 boards, make it fit those."
//!   The bead count is given and the image is centre-cropped to that shape.
//! * [`FitMode::AspectFit`] — "use whatever boards you need, keep my
//!   proportions." Whole boards quantise the achievable aspect ratios, so this
//!   picks the closest one and reports the small crop that remains.
//! * [`FitMode::ScaleCrop`] — a fixed viewport the user zoomed and panned
//!   inside. `scale` is relative to the cover fit, so 1.0 reproduces
//!   [`FitMode::FixedBoards`] and 2.0 is twice as close.
//!
//! A [`FitPlan`] is only arithmetic: which rectangle of source pixels maps onto
//! which grid of cells. [`render`] is the separate step that actually samples.

use std::fmt;

use crate::color::Rgb;
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

/// How many boards, laid out as a rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardTiling {
    pub board: BoardSpec,
    pub cols: u32,
    pub rows: u32,
}

impl BoardTiling {
    pub fn cells_wide(&self) -> u32 {
        self.board.width * self.cols
    }

    pub fn cells_high(&self) -> u32 {
        self.board.height * self.rows
    }

    pub fn board_count(&self) -> u32 {
        self.cols * self.rows
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
}

/// A framing decision: what to sample, and onto what.
#[derive(Debug, Clone, PartialEq)]
pub struct FitPlan {
    pub tiling: BoardTiling,
    pub source: SourceRect,
    /// Share of the source image left outside `source`, `0.0..1.0`.
    pub cropped_fraction: f64,
}

impl FitPlan {
    pub fn cells_wide(&self) -> u32 {
        self.tiling.cells_wide()
    }

    pub fn cells_high(&self) -> u32 {
        self.tiling.cells_high()
    }

    pub fn total_cells(&self) -> u64 {
        u64::from(self.cells_wide()) * u64::from(self.cells_high())
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
    /// Whatever board rectangle within the budget best matches the image.
    AspectFit {
        board: BoardSpec,
        max_cols: u32,
        max_rows: u32,
    },
    /// A fixed viewport, zoomed and panned by hand.
    ScaleCrop {
        board: BoardSpec,
        cols: u32,
        rows: u32,
        /// 1.0 is the cover fit; larger zooms in.
        scale: f64,
        /// Pan away from centre, in source pixels. Clamped to the image.
        offset_x: f64,
        offset_y: f64,
    },
}

/// The framing could not be worked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitError {
    /// A source dimension was zero.
    EmptySource,
    /// A board dimension, or a board count, was zero.
    EmptyBoard,
    /// `scale` was zero, negative or not finite.
    BadScale,
}

impl fmt::Display for FitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FitError::EmptySource => f.write_str("the source image has no pixels"),
            FitError::EmptyBoard => f.write_str("a board layout needs a non-zero size and count"),
            FitError::BadScale => f.write_str("scale must be a finite number above zero"),
        }
    }
}

impl std::error::Error for FitError {}

/// Work out the framing for one mode.
pub fn plan(source_width: u32, source_height: u32, mode: &FitMode) -> Result<FitPlan, FitError> {
    if source_width == 0 || source_height == 0 {
        return Err(FitError::EmptySource);
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
            scale,
            offset_x,
            offset_y,
        } => scale_crop(
            source_width,
            source_height,
            board,
            *cols,
            *rows,
            *scale,
            *offset_x,
            *offset_y,
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
    let tiling = tiling(board, cols, rows)?;
    let source = cover_rect(
        source_width,
        source_height,
        f64::from(tiling.cells_wide()) / f64::from(tiling.cells_high()),
    );
    Ok(finish(source_width, source_height, tiling, source))
}

/// The board rectangle within the budget whose shape is closest to the image.
///
/// Ties go to the larger bead count, because two layouts that frame the picture
/// equally well are not equally detailed.
pub fn aspect_fit(
    source_width: u32,
    source_height: u32,
    board: &BoardSpec,
    max_cols: u32,
    max_rows: u32,
) -> Result<FitPlan, FitError> {
    if board.width == 0 || board.height == 0 || max_cols == 0 || max_rows == 0 {
        return Err(FitError::EmptyBoard);
    }
    let target = f64::from(source_width) / f64::from(source_height);
    let mut best: Option<(f64, u64, u32, u32)> = None;
    for cols in 1..=max_cols {
        for rows in 1..=max_rows {
            let aspect = f64::from(board.width * cols) / f64::from(board.height * rows);
            // Compared in log space so that 2:1 and 1:2 are equally wrong.
            let error = (aspect.ln() - target.ln()).abs();
            let cells = u64::from(board.width * cols) * u64::from(board.height * rows);
            let candidate = (error, cells, cols, rows);
            let better = match best {
                None => true,
                Some((best_error, best_cells, _, _)) => {
                    error < best_error - 1e-12
                        || ((error - best_error).abs() <= 1e-12 && cells > best_cells)
                }
            };
            if better {
                best = Some(candidate);
            }
        }
    }
    let (_, _, cols, rows) = best.expect("the loops run at least once");
    fixed_boards(source_width, source_height, board, cols, rows)
}

/// A fixed viewport with a manual zoom and pan.
#[allow(clippy::too_many_arguments)]
pub fn scale_crop(
    source_width: u32,
    source_height: u32,
    board: &BoardSpec,
    cols: u32,
    rows: u32,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
) -> Result<FitPlan, FitError> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(FitError::BadScale);
    }
    let tiling = tiling(board, cols, rows)?;
    let base = cover_rect(
        source_width,
        source_height,
        f64::from(tiling.cells_wide()) / f64::from(tiling.cells_high()),
    );

    let width = base.width / scale;
    let height = base.height / scale;
    let centre_x = base.x + base.width / 2.0 + offset_x;
    let centre_y = base.y + base.height / 2.0 + offset_y;

    // Pan is clamped rather than rejected: dragging a picture past its own edge
    // should stop at the edge, not fail.
    let max_x = f64::from(source_width) - width;
    let max_y = f64::from(source_height) - height;
    let x = (centre_x - width / 2.0).clamp(0.0, max_x.max(0.0));
    let y = (centre_y - height / 2.0).clamp(0.0, max_y.max(0.0));

    let source = SourceRect {
        x,
        y,
        width: width.min(f64::from(source_width)),
        height: height.min(f64::from(source_height)),
    };
    Ok(finish(source_width, source_height, tiling, source))
}

fn tiling(board: &BoardSpec, cols: u32, rows: u32) -> Result<BoardTiling, FitError> {
    if board.width == 0 || board.height == 0 || cols == 0 || rows == 0 {
        return Err(FitError::EmptyBoard);
    }
    Ok(BoardTiling {
        board: board.clone(),
        cols,
        rows,
    })
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
    tiling: BoardTiling,
    source: SourceRect,
) -> FitPlan {
    let whole = f64::from(source_width) * f64::from(source_height);
    let kept = (source.width * source.height).clamp(0.0, whole);
    FitPlan {
        tiling,
        source,
        cropped_fraction: 1.0 - kept / whole,
    }
}

/// How to read source pixels when the cell grid is coarser than the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sampling {
    /// One source pixel per cell. Right for pixel art, where averaging would
    /// invent colours that are not in the drawing.
    Nearest,
    /// Mean of every source pixel the cell covers. Right for photographs.
    #[default]
    BoxAverage,
}

/// Sample `image` through `plan` into a cell-resolution raster.
pub fn render(image: &Image, plan: &FitPlan, sampling: Sampling) -> Image {
    let cells_wide = plan.cells_wide();
    let cells_high = plan.cells_high();
    let cell_w = plan.source.width / f64::from(cells_wide);
    let cell_h = plan.source.height / f64::from(cells_high);

    let mut pixels = Vec::with_capacity(cells_wide as usize * cells_high as usize);
    for cy in 0..cells_high {
        for cx in 0..cells_wide {
            let x0 = plan.source.x + f64::from(cx) * cell_w;
            let y0 = plan.source.y + f64::from(cy) * cell_h;
            pixels.push(match sampling {
                Sampling::Nearest => {
                    image.clamped((x0 + cell_w / 2.0) as i64, (y0 + cell_h / 2.0) as i64)
                }
                Sampling::BoxAverage => box_average(image, x0, y0, cell_w, cell_h),
            });
        }
    }
    Image::from_pixels(cells_wide, cells_high, pixels).expect("one pixel per cell")
}

fn box_average(image: &Image, x0: f64, y0: f64, cell_w: f64, cell_h: f64) -> Rgb {
    let first_x = x0.floor() as i64;
    let first_y = y0.floor() as i64;
    let last_x = ((x0 + cell_w).ceil() as i64 - 1).max(first_x);
    let last_y = ((y0 + cell_h).ceil() as i64 - 1).max(first_y);

    let mut sum = [0f64; 3];
    let mut count = 0f64;
    for y in first_y..=last_y {
        for x in first_x..=last_x {
            let p = image.clamped(x, y);
            sum[0] += f64::from(p.r);
            sum[1] += f64::from(p.g);
            sum[2] += f64::from(p.b);
            count += 1.0;
        }
    }
    if count == 0.0 {
        return image.clamped(first_x, first_y);
    }
    Rgb::new(
        (sum[0] / count).round().clamp(0.0, 255.0) as u8,
        (sum[1] / count).round().clamp(0.0, 255.0) as u8,
        (sum[2] / count).round().clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn a_square_source_on_a_square_board_is_not_cropped() {
        let plan = fixed_boards(100, 100, &BoardSpec::square_28(), 1, 1).expect("plan");
        assert_eq!(plan.cells_wide(), 28);
        assert_eq!(plan.cells_high(), 28);
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
    fn a_non_positive_scale_is_refused() {
        assert_eq!(
            scale_crop(10, 10, &BoardSpec::square_28(), 1, 1, 0.0, 0.0, 0.0).unwrap_err(),
            FitError::BadScale
        );
    }
}
