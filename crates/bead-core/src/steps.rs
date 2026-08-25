//! Turning a finished pattern into an order to place the beads in.
//!
//! Four modes, one shape of answer: an ordered list of [`StepGroup`]s, each an
//! ordered list of cells. A group is what the guidance screen highlights at
//! once — a colour, a board, a region's outline, a row — and every mode
//! partitions the placeable cells exactly, so "how far along am I" is a count
//! and never an estimate.
//!
//! Cells whose colour is listed in [`StepOptions::skip`] produce no step. That
//! is how transparency and a deliberate background hole are expressed without
//! giving the grid a second cell type.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::fit::BoardSpec;
use crate::grid::{Cell, Grid};
use crate::palette::{ColorId, Palette};

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
    /// Per same-colour region: outer edge, then the ring inside it, then the
    /// middle. Building the edge first gives the infill something to sit
    /// against, which is what stops a large field from drifting.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Touches something that is not this region, or the edge of the pattern.
    Outline,
    /// Touches the outline from inside.
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

/// Colours that are not placed at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StepOptions {
    pub skip: BTreeSet<ColorId>,
}

impl StepOptions {
    pub fn skipping(skip: impl IntoIterator<Item = ColorId>) -> Self {
        Self {
            skip: skip.into_iter().collect(),
        }
    }

    fn is_skipped(&self, id: ColorId) -> bool {
        self.skip.contains(&id)
    }
}

/// One highlighted stretch of work.
#[derive(Debug, Clone, PartialEq)]
pub struct StepGroup {
    pub label: String,
    /// The single colour of the group, when it has one.
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
pub fn plan_steps(
    grid: &Grid<ColorId>,
    palette: &Palette,
    mode: &StepMode,
    options: &StepOptions,
) -> StepPlan {
    let groups = match mode {
        StepMode::ColorByColor { order } => color_by_color(grid, palette, *order, options),
        StepMode::Tile { board } => tile(grid, board, options),
        StepMode::OutlineInfill => outline_infill(grid, palette, options),
        StepMode::RowByRow => row_by_row(grid, options),
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

fn sort_key(palette: &Palette, id: ColorId) -> String {
    palette
        .get(id)
        .map(|c| c.code.clone())
        .unwrap_or_else(|| id.to_string())
}

fn color_by_color(
    grid: &Grid<ColorId>,
    palette: &Palette,
    order: ColorOrder,
    options: &StepOptions,
) -> Vec<StepGroup> {
    let mut by_color: BTreeMap<ColorId, Vec<Cell>> = BTreeMap::new();
    for (cell, id) in grid.iter() {
        if options.is_skipped(*id) {
            continue;
        }
        by_color.entry(*id).or_default().push(cell);
    }

    let mut colors: Vec<(ColorId, Vec<Cell>)> = by_color.into_iter().collect();
    // Ties break on the palette code so two colours with the same count always
    // come out in the same order, whichever direction was asked for.
    colors.sort_by(|(a_id, a_cells), (b_id, b_cells)| {
        let primary = match order {
            ColorOrder::AccentFirst => a_cells.len().cmp(&b_cells.len()),
            ColorOrder::BulkFirst => b_cells.len().cmp(&a_cells.len()),
        };
        primary.then_with(|| sort_key(palette, *a_id).cmp(&sort_key(palette, *b_id)))
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

fn tile(grid: &Grid<ColorId>, board: &BoardSpec, options: &StepOptions) -> Vec<StepGroup> {
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
                    match grid.get(cell) {
                        Some(id) if !options.is_skipped(*id) => cells.push(cell),
                        _ => {}
                    }
                }
            }
            groups.push(StepGroup {
                label: format!("Board {} r{}c{}", board.name, row + 1, col + 1),
                color: None,
                phase: None,
                cells,
            });
        }
    }
    groups
}

fn row_by_row(grid: &Grid<ColorId>, options: &StepOptions) -> Vec<StepGroup> {
    (0..grid.height())
        .map(|y| {
            let cells = (0..grid.width())
                .map(|x| Cell::new(x, y))
                .filter(|cell| grid.get(*cell).is_some_and(|id| !options.is_skipped(*id)))
                .collect();
            StepGroup {
                label: format!("Row {}", y + 1),
                color: None,
                phase: None,
                cells,
            }
        })
        .collect()
}

fn outline_infill(
    grid: &Grid<ColorId>,
    palette: &Palette,
    options: &StepOptions,
) -> Vec<StepGroup> {
    let mut visited: Vec<bool> = vec![false; grid.len()];
    let index = |cell: Cell| cell.y as usize * grid.width() as usize + cell.x as usize;

    let mut groups = Vec::new();
    let mut region_number = 0usize;

    for (start, id) in grid.iter() {
        if visited[index(start)] || options.is_skipped(*id) {
            continue;
        }
        let region = flood(grid, start, *id, &mut visited, index);
        region_number += 1;

        let members: BTreeSet<Cell> = region.iter().copied().collect();
        let mut outline = Vec::new();
        let mut interior = Vec::new();
        for cell in &region {
            let touches_outside = grid.neighbours4(*cell).len() < 4
                || grid.neighbours4(*cell).iter().any(|n| !members.contains(n));
            if touches_outside {
                outline.push(*cell);
            } else {
                interior.push(*cell);
            }
        }

        let outline_set: BTreeSet<Cell> = outline.iter().copied().collect();
        let mut inner_edge = Vec::new();
        let mut fill = Vec::new();
        for cell in interior {
            if grid
                .neighbours4(cell)
                .iter()
                .any(|n| outline_set.contains(n))
            {
                inner_edge.push(cell);
            } else {
                fill.push(cell);
            }
        }

        let name = describe(palette, *id);
        for (phase, cells) in [
            (Phase::Outline, outline),
            (Phase::InnerEdge, inner_edge),
            (Phase::Fill, fill),
        ] {
            if cells.is_empty() {
                continue;
            }
            groups.push(StepGroup {
                label: format!("{name} region {region_number} {}", phase.slug()),
                color: Some(*id),
                phase: Some(phase),
                cells,
            });
        }
    }
    groups
}

/// Breadth-first over edge-sharing cells of the same colour, collected in
/// reading order so the caller never sees the traversal order.
fn flood(
    grid: &Grid<ColorId>,
    start: Cell,
    id: ColorId,
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
            if visited[index(neighbour)] {
                continue;
            }
            if grid.get(neighbour) != Some(&id) {
                continue;
            }
            visited[index(neighbour)] = true;
            queue.push_back(neighbour);
        }
    }
    found.sort_by_key(|c| (c.y, c.x));
    found
}

#[cfg(test)]
mod unit {
    use super::*;

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
}
