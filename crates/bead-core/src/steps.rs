//! Turning a finished pattern into an order to place the beads in.
//!
//! Four modes, one shape of answer: an ordered list of [`StepGroup`]s, each an
//! ordered list of cells. A group is what the guidance screen highlights at
//! once — a colour, a board, a region's outline, a row — and every mode
//! partitions the *filled* cells exactly, so "how far along am I" is a count
//! and never an estimate.
//!
//! Empty cells (G1) produce no step in any mode, and a group that would contain
//! nothing but empty cells is dropped rather than shown as a board with no work
//! in it.
//!
//! Every ordering is total. Where two things could reasonably come first, the
//! tie breaks on the palette colour index, or on a region's topmost-then-
//! leftmost cell (G6) — never on hash iteration order, because the browser port
//! has to produce the same sequence.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::fit::BoardSpec;
use crate::grid::Cell;
use crate::palette::{ColorId, Palette};
use crate::quantize::PatternGrid;

/// Which colour to start with when working colour by colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorOrder {
    /// Rarest colour first. The default: the few-bead accent colours are the
    /// ones that are easy to misplace once the board is crowded.
    #[default]
    AccentFirst,
    /// Commonest colour first, for people who would rather block in the mass.
    BulkFirst,
}

/// The four assembly orders.
#[derive(Debug, Clone, PartialEq)]
pub enum StepMode {
    /// One colour at a time, across the whole pattern.
    ColorByColor { order: ColorOrder },
    /// One board at a time, left to right and top to bottom.
    Tile { board: BoardSpec },
    /// Per connected region of filled cells: the edge that touches the outside,
    /// then the edge that touches a hole, then the middle. Building the edges
    /// first gives the infill something to sit against, which is what stops a
    /// large field from drifting.
    OutlineInfill,
    /// Top to bottom, left to right.
    RowByRow,
}

impl StepMode {
    /// A stable identifier for the mode, for UI state and fixtures.
    pub fn slug(&self) -> &'static str {
        match self {
            StepMode::ColorByColor { .. } => "color-by-color",
            StepMode::Tile { .. } => "tile",
            StepMode::OutlineInfill => "outline-infill",
            StepMode::RowByRow => "row-by-row",
        }
    }
}

/// Where a cell sits within its region, for [`StepMode::OutlineInfill`].
///
/// G4 defines these on the filled mask, not per colour: a region is a
/// four-connected run of filled cells whatever their colours, because what
/// holds a fuse-bead panel together is the beads touching, not the beads
/// matching. A cell belongs to exactly one phase and [`Phase::Outline`] wins,
/// so a one-cell-wide ring is all outline rather than counted twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Touches the outside — the background connected to the edge of the
    /// pattern, or the edge itself — diagonally or otherwise.
    Outline,
    /// Touches a hole: background enclosed by the region.
    InnerEdge,
    /// Everything else.
    Fill,
}

impl Phase {
    pub fn slug(self) -> &'static str {
        match self {
            Phase::Outline => "outline",
            Phase::InnerEdge => "inner-edge",
            Phase::Fill => "fill",
        }
    }
}

/// One highlighted stretch of work.
#[derive(Debug, Clone, PartialEq)]
pub struct StepGroup {
    pub label: String,
    /// The group's colour, when every cell in it is the same one.
    pub color: Option<ColorId>,
    /// Only set by [`StepMode::OutlineInfill`].
    pub phase: Option<Phase>,
    pub cells: Vec<Cell>,
}

impl StepGroup {
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Always false: empty groups are dropped before a plan is returned.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// An ordered assembly plan.
#[derive(Debug, Clone, PartialEq)]
pub struct StepPlan {
    pub mode: &'static str,
    pub groups: Vec<StepGroup>,
}

impl StepPlan {
    /// Total beads to place.
    pub fn total_cells(&self) -> usize {
        self.groups.iter().map(StepGroup::len).sum()
    }

    /// Every placement in order, as `(group index, cell)`.
    pub fn placements(&self) -> impl Iterator<Item = (usize, Cell)> + '_ {
        self.groups
            .iter()
            .enumerate()
            .flat_map(|(i, g)| g.cells.iter().map(move |c| (i, *c)))
    }
}

/// Build the plan for one mode.
pub fn plan_steps(grid: &PatternGrid, palette: &Palette, mode: &StepMode) -> StepPlan {
    let groups = match mode {
        StepMode::ColorByColor { order } => color_by_color(grid, palette, *order),
        StepMode::Tile { board } => tile(grid, board),
        StepMode::OutlineInfill => outline_infill(grid),
        StepMode::RowByRow => row_by_row(grid),
    };
    StepPlan {
        mode: mode.slug(),
        groups: groups.into_iter().filter(|g| !g.cells.is_empty()).collect(),
    }
}

/// `G12 Rose`, or `#7` when the id is not from this palette.
fn describe(palette: &Palette, id: ColorId) -> String {
    match palette.get(id) {
        Some(color) => format!("{} {}", color.code, color.name),
        None => id.to_string(),
    }
}

fn color_by_color(grid: &PatternGrid, palette: &Palette, order: ColorOrder) -> Vec<StepGroup> {
    let mut by_color: BTreeMap<ColorId, Vec<Cell>> = BTreeMap::new();
    for (cell, slot) in grid.iter() {
        if let Some(id) = slot {
            by_color.entry(*id).or_default().push(cell);
        }
    }

    let mut colors: Vec<(ColorId, Vec<Cell>)> = by_color.into_iter().collect();
    // Ties break on the colour index in both directions, so two colours with
    // the same count keep the same relative order whichever way round the
    // primary key points.
    colors.sort_by(|(a_id, a_cells), (b_id, b_cells)| {
        let primary = match order {
            ColorOrder::AccentFirst => a_cells.len().cmp(&b_cells.len()),
            ColorOrder::BulkFirst => b_cells.len().cmp(&a_cells.len()),
        };
        primary.then_with(|| a_id.cmp(b_id))
    });

    colors
        .into_iter()
        .map(|(id, cells)| StepGroup {
            label: format!("{} ×{}", describe(palette, id), cells.len()),
            color: Some(id),
            phase: None,
            cells,
        })
        .collect()
}

fn tile(grid: &PatternGrid, board: &BoardSpec) -> Vec<StepGroup> {
    let board_w = board.width.max(1);
    let board_h = board.height.max(1);
    let cols = grid.width().div_ceil(board_w);
    let rows = grid.height().div_ceil(board_h);

    let mut groups = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let x0 = col * board_w;
            let y0 = row * board_h;
            let mut cells = Vec::new();
            for y in y0..(y0 + board_h).min(grid.height()) {
                for x in x0..(x0 + board_w).min(grid.width()) {
                    let cell = Cell::new(x, y);
                    if matches!(grid.get(cell), Some(Some(_))) {
                        cells.push(cell);
                    }
                }
            }
            let color = single_color(grid, &cells);
            groups.push(StepGroup {
                label: format!("Board {} r{}c{}", board.name, row + 1, col + 1),
                color,
                phase: None,
                cells,
            });
        }
    }
    groups
}

fn row_by_row(grid: &PatternGrid) -> Vec<StepGroup> {
    (0..grid.height())
        .map(|y| {
            let cells: Vec<Cell> = (0..grid.width())
                .map(|x| Cell::new(x, y))
                .filter(|cell| matches!(grid.get(*cell), Some(Some(_))))
                .collect();
            let color = single_color(grid, &cells);
            StepGroup {
                label: format!("Row {}", y + 1),
                color,
                phase: None,
                cells,
            }
        })
        .collect()
}

fn outline_infill(grid: &PatternGrid) -> Vec<StepGroup> {
    let holes = enclosed_background(grid);

    let mut visited: Vec<bool> = vec![false; grid.len()];
    let index = |cell: Cell| cell.y as usize * grid.width() as usize + cell.x as usize;

    let mut groups = Vec::new();
    let mut region_number = 0usize;

    for (start, slot) in grid.iter() {
        if slot.is_none() || visited[index(start)] {
            continue;
        }
        let region = flood_filled(grid, start, &mut visited, index);
        region_number += 1;

        let members: BTreeSet<Cell> = region.iter().copied().collect();
        let mut outline = Vec::new();
        let mut inner_edge = Vec::new();
        let mut fill = Vec::new();

        for cell in &region {
            // The pattern's own edge counts as outside: a bead on the border of
            // the board has nothing holding it on that side either.
            let on_border = grid.neighbours8(*cell).len() < 8;
            let neighbours = grid.neighbours8(*cell);
            let touches_exterior = on_border
                || neighbours
                    .iter()
                    .any(|n| is_empty(grid, *n) && !holes.contains(n));
            if touches_exterior {
                outline.push(*cell);
                continue;
            }
            if neighbours.iter().any(|n| holes.contains(n)) {
                inner_edge.push(*cell);
                continue;
            }
            debug_assert!(members.contains(cell));
            fill.push(*cell);
        }

        for (phase, cells) in [
            (Phase::Outline, outline),
            (Phase::InnerEdge, inner_edge),
            (Phase::Fill, fill),
        ] {
            if cells.is_empty() {
                continue;
            }
            groups.push(StepGroup {
                label: format!("Region {region_number} {}", phase.slug()),
                color: single_color(grid, &cells),
                phase: Some(phase),
                cells,
            });
        }
    }
    groups
}

fn is_empty(grid: &PatternGrid, cell: Cell) -> bool {
    matches!(grid.get(cell), Some(None))
}

/// The empty cells a region encloses: everything empty that a four-connected
/// flood from the pattern's edge cannot reach.
fn enclosed_background(grid: &PatternGrid) -> BTreeSet<Cell> {
    let mut reachable: BTreeSet<Cell> = BTreeSet::new();
    let mut queue: VecDeque<Cell> = VecDeque::new();

    let seed = |cell: Cell, reachable: &mut BTreeSet<Cell>, queue: &mut VecDeque<Cell>| {
        if is_empty(grid, cell) && reachable.insert(cell) {
            queue.push_back(cell);
        }
    };
    for x in 0..grid.width() {
        seed(Cell::new(x, 0), &mut reachable, &mut queue);
        seed(
            Cell::new(x, grid.height().saturating_sub(1)),
            &mut reachable,
            &mut queue,
        );
    }
    for y in 0..grid.height() {
        seed(Cell::new(0, y), &mut reachable, &mut queue);
        seed(
            Cell::new(grid.width().saturating_sub(1), y),
            &mut reachable,
            &mut queue,
        );
    }

    while let Some(cell) = queue.pop_front() {
        for neighbour in grid.neighbours4(cell) {
            if is_empty(grid, neighbour) && reachable.insert(neighbour) {
                queue.push_back(neighbour);
            }
        }
    }

    grid.iter()
        .filter(|(cell, slot)| slot.is_none() && !reachable.contains(cell))
        .map(|(cell, _)| cell)
        .collect()
}

/// Breadth-first over edge-sharing filled cells, collected in reading order so
/// the caller never sees the traversal order.
fn flood_filled(
    grid: &PatternGrid,
    start: Cell,
    visited: &mut [bool],
    index: impl Fn(Cell) -> usize,
) -> Vec<Cell> {
    let mut queue = VecDeque::new();
    let mut found = Vec::new();
    visited[index(start)] = true;
    queue.push_back(start);
    while let Some(cell) = queue.pop_front() {
        found.push(cell);
        for neighbour in grid.neighbours4(cell) {
            if visited[index(neighbour)] || is_empty(grid, neighbour) {
                continue;
            }
            visited[index(neighbour)] = true;
            queue.push_back(neighbour);
        }
    }
    found.sort_by_key(|c| (c.y, c.x));
    found
}

/// The one colour every listed cell shares, if there is one.
fn single_color(grid: &PatternGrid, cells: &[Cell]) -> Option<ColorId> {
    let mut found: Option<ColorId> = None;
    for cell in cells {
        let id = (*grid.get(*cell)?)?;
        match found {
            Some(current) if current != id => return None,
            _ => found = Some(id),
        }
    }
    found
}

#[cfg(test)]
mod unit {
    use super::*;
    use crate::grid::Grid;

    #[test]
    fn every_mode_has_a_stable_slug() {
        assert_eq!(
            StepMode::ColorByColor {
                order: ColorOrder::AccentFirst
            }
            .slug(),
            "color-by-color"
        );
        assert_eq!(
            StepMode::Tile {
                board: BoardSpec::square_28()
            }
            .slug(),
            "tile"
        );
        assert_eq!(StepMode::OutlineInfill.slug(), "outline-infill");
        assert_eq!(StepMode::RowByRow.slug(), "row-by-row");
    }

    #[test]
    fn a_hole_is_background_the_edge_cannot_reach() {
        // A 3×3 ring: the centre is enclosed, everything outside it is not.
        let mut grid: PatternGrid = Grid::filled(3, 3, Some(ColorId(0)));
        grid.set(Cell::new(1, 1), None);
        let holes = enclosed_background(&grid);
        assert_eq!(holes.len(), 1);
        assert!(holes.contains(&Cell::new(1, 1)));

        // Break the ring and the same cell is reachable from outside.
        grid.set(Cell::new(1, 0), None);
        assert!(enclosed_background(&grid).is_empty());
    }
}
