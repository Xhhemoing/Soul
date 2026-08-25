//! The four assembly orders.

use std::collections::BTreeSet;

use bead_core::fit::BoardSpec;
use bead_core::grid::{Cell, Grid};
use bead_core::palette::{ColorId, Palette};
use bead_core::steps::{plan_steps, ColorOrder, Phase, StepMode, StepOptions, StepPlan};

/// 6×6: a background field, a 4×4 block inset by one, and a single accent bead
/// in the top-left corner.
///
/// ```text
/// C A A A A A
/// A B B B B A
/// A B B B B A
/// A B B B B A
/// A B B B B A
/// A A A A A A
/// ```
///
/// Counts: A 19, B 16, C 1.
fn fixture(palette: &Palette) -> (Grid<ColorId>, ColorId, ColorId, ColorId) {
    let a = palette.find_code("G01").expect("G01");
    let b = palette.find_code("G33").expect("G33");
    let c = palette.find_code("G15").expect("G15");

    let mut grid = Grid::filled(6, 6, a).expect("6x6");
    for y in 1..5u32 {
        for x in 1..5u32 {
            grid.set(Cell::new(x, y), b);
        }
    }
    grid.set(Cell::new(0, 0), c);
    (grid, a, b, c)
}

/// Every mode has to partition the placeable cells: each one exactly once, none
/// invented. Checked for every mode rather than asserted once, because a
/// partition is the property the progress bar depends on.
fn assert_partitions(plan: &StepPlan, expected: &BTreeSet<Cell>) {
    let mut seen: Vec<Cell> = plan.placements().map(|(_, cell)| cell).collect();
    assert_eq!(
        seen.len(),
        expected.len(),
        "{} placed {} cells, expected {}",
        plan.mode,
        seen.len(),
        expected.len()
    );
    seen.sort_by_key(|c| (c.y, c.x));
    let unique: BTreeSet<Cell> = seen.iter().copied().collect();
    assert_eq!(unique.len(), seen.len(), "{} repeated a cell", plan.mode);
    assert_eq!(&unique, expected, "{} placed the wrong cells", plan.mode);
    assert_eq!(plan.total_cells(), expected.len());
    assert!(
        plan.groups.iter().all(|g| !g.is_empty()),
        "{} kept an empty group",
        plan.mode
    );
}

fn all_cells(grid: &Grid<ColorId>) -> BTreeSet<Cell> {
    grid.cells().collect()
}

fn every_mode() -> Vec<StepMode> {
    vec![
        StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
        StepMode::ColorByColor {
            order: ColorOrder::BulkFirst,
        },
        StepMode::Tile {
            board: BoardSpec::new("2x2", 2, 2),
        },
        StepMode::OutlineInfill,
        StepMode::RowByRow,
    ]
}

#[test]
fn every_mode_partitions_the_pattern() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let expected = all_cells(&grid);
    for mode in every_mode() {
        let plan = plan_steps(&grid, &palette, &mode, &StepOptions::default());
        assert_partitions(&plan, &expected);
    }
}

#[test]
fn colour_by_colour_starts_with_the_rarest_colour() {
    let palette = Palette::generic_5mm();
    let (grid, a, b, c) = fixture(&palette);
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
        &StepOptions::default(),
    );

    assert_eq!(plan.mode, "color-by-color");
    assert_eq!(plan.groups.len(), 3);
    assert_eq!(
        plan.groups.iter().map(|g| g.color).collect::<Vec<_>>(),
        vec![Some(c), Some(b), Some(a)]
    );
    assert_eq!(
        plan.groups.iter().map(|g| g.len()).collect::<Vec<_>>(),
        vec![1, 16, 19]
    );
    assert_eq!(plan.groups[0].label, "G15 Red ×1");
}

#[test]
fn bulk_first_is_the_same_groups_reversed() {
    let palette = Palette::generic_5mm();
    let (grid, a, b, c) = fixture(&palette);
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::ColorByColor {
            order: ColorOrder::BulkFirst,
        },
        &StepOptions::default(),
    );
    assert_eq!(
        plan.groups.iter().map(|g| g.color).collect::<Vec<_>>(),
        vec![Some(a), Some(b), Some(c)]
    );
}

/// Two colours with the same count must not swap places between runs, so the
/// tie breaks on the palette code.
#[test]
fn equal_counts_break_the_tie_on_code() {
    let palette = Palette::generic_5mm();
    let left = palette.find_code("G22").expect("G22");
    let right = palette.find_code("G06").expect("G06");
    let mut grid = Grid::filled(2, 1, left).expect("2x1");
    grid.set(Cell::new(1, 0), right);

    for order in [ColorOrder::AccentFirst, ColorOrder::BulkFirst] {
        let plan = plan_steps(
            &grid,
            &palette,
            &StepMode::ColorByColor { order },
            &StepOptions::default(),
        );
        assert_eq!(
            plan.groups.iter().map(|g| g.color).collect::<Vec<_>>(),
            vec![Some(right), Some(left)],
            "G06 sorts before G22 whichever direction was asked for"
        );
    }
}

#[test]
fn tiles_are_boards_left_to_right_then_top_to_bottom() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::Tile {
            board: BoardSpec::new("2x2", 2, 2),
        },
        &StepOptions::default(),
    );

    assert_eq!(plan.mode, "tile");
    assert_eq!(plan.groups.len(), 9);
    assert!(plan.groups.iter().all(|g| g.len() == 4));
    assert_eq!(plan.groups[0].label, "Board 2x2 r1c1");
    assert_eq!(plan.groups[0].cells[0], Cell::new(0, 0));
    assert_eq!(
        plan.groups[1].cells[0],
        Cell::new(2, 0),
        "next board across"
    );
    assert_eq!(plan.groups[3].cells[0], Cell::new(0, 2), "then down a row");
}

/// A pattern wider than the board leaves a partial column, and a partial board
/// is still a board.
#[test]
fn a_partial_board_keeps_only_the_cells_that_exist() {
    let palette = Palette::generic_5mm();
    let grid = Grid::filled(5, 3, palette.find_code("G01").expect("G01")).expect("5x3");
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::Tile {
            board: BoardSpec::new("4x4", 4, 4),
        },
        &StepOptions::default(),
    );
    assert_eq!(plan.groups.len(), 2);
    assert_eq!(plan.groups[0].len(), 12, "the full 4×3 slice");
    assert_eq!(plan.groups[1].len(), 3, "the leftover column");
    assert_eq!(plan.total_cells(), 15);
}

#[test]
fn a_28_board_tiling_matches_the_standard_preset() {
    let palette = Palette::generic_5mm();
    let grid = Grid::filled(56, 56, palette.find_code("G01").expect("G01")).expect("56x56");
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::Tile {
            board: BoardSpec::square_28(),
        },
        &StepOptions::default(),
    );
    assert_eq!(plan.groups.len(), 4);
    assert!(plan.groups.iter().all(|g| g.len() == 784));
    assert_eq!(plan.groups[3].label, "Board 28x28 r2c2");
}

#[test]
fn rows_run_top_to_bottom_and_left_to_right() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::RowByRow,
        &StepOptions::default(),
    );

    assert_eq!(plan.mode, "row-by-row");
    assert_eq!(plan.groups.len(), 6);
    assert_eq!(plan.groups[0].label, "Row 1");
    for (y, group) in plan.groups.iter().enumerate() {
        assert_eq!(group.len(), 6);
        for (x, cell) in group.cells.iter().enumerate() {
            assert_eq!(*cell, Cell::new(x as u32, y as u32));
        }
    }
}

#[test]
fn outline_then_inner_edge_then_fill_per_region() {
    let palette = Palette::generic_5mm();
    let (grid, a, b, c) = fixture(&palette);
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::OutlineInfill,
        &StepOptions::default(),
    );

    assert_eq!(plan.mode, "outline-infill");
    // Regions are found in reading order: the corner accent, the background
    // field, then the inset block.
    let shape: Vec<(Option<ColorId>, Option<Phase>, usize)> = plan
        .groups
        .iter()
        .map(|g| (g.color, g.phase, g.len()))
        .collect();
    assert_eq!(
        shape,
        vec![
            (Some(c), Some(Phase::Outline), 1),
            (Some(a), Some(Phase::Outline), 19),
            (Some(b), Some(Phase::Outline), 12),
            (Some(b), Some(Phase::InnerEdge), 4),
        ]
    );
    assert_eq!(plan.groups[2].label, "G33 Blue region 3 outline");
}

/// A region big enough to have a middle produces all three phases, and the
/// counts are the concentric rings you would draw by hand.
#[test]
fn a_large_region_reaches_the_fill_phase() {
    let palette = Palette::generic_5mm();
    let grid = Grid::filled(6, 6, palette.find_code("G01").expect("G01")).expect("6x6");
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::OutlineInfill,
        &StepOptions::default(),
    );
    assert_eq!(
        plan.groups
            .iter()
            .map(|g| (g.phase, g.len()))
            .collect::<Vec<_>>(),
        vec![
            (Some(Phase::Outline), 20),
            (Some(Phase::InnerEdge), 12),
            (Some(Phase::Fill), 4),
        ]
    );
    assert_eq!(Phase::Outline.slug(), "outline");
    assert_eq!(Phase::InnerEdge.slug(), "inner-edge");
    assert_eq!(Phase::Fill.slug(), "fill");
}

/// Same colour, two separate blobs: two regions, not one, or the guidance would
/// tell someone to jump across the board mid-outline.
#[test]
fn disconnected_areas_of_one_colour_are_separate_regions() {
    let palette = Palette::generic_5mm();
    let background = palette.find_code("G01").expect("G01");
    let spot = palette.find_code("G15").expect("G15");
    let mut grid = Grid::filled(5, 1, background).expect("5x1");
    grid.set(Cell::new(0, 0), spot);
    grid.set(Cell::new(4, 0), spot);

    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::OutlineInfill,
        &StepOptions::default(),
    );
    let spot_groups: Vec<&str> = plan
        .groups
        .iter()
        .filter(|g| g.color == Some(spot))
        .map(|g| g.label.as_str())
        .collect();
    assert_eq!(
        spot_groups,
        vec!["G15 Red region 1 outline", "G15 Red region 3 outline"]
    );
}

/// Diagonal touching is not connection: fuse beads sit in a square lattice and
/// a diagonal neighbour is not holding anything up.
#[test]
fn regions_are_four_connected() {
    let palette = Palette::generic_5mm();
    let background = palette.find_code("G01").expect("G01");
    let spot = palette.find_code("G15").expect("G15");
    let mut grid = Grid::filled(2, 2, background).expect("2x2");
    grid.set(Cell::new(0, 0), spot);
    grid.set(Cell::new(1, 1), spot);

    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::OutlineInfill,
        &StepOptions::default(),
    );
    assert_eq!(
        plan.groups.iter().filter(|g| g.color == Some(spot)).count(),
        2
    );
}

#[test]
fn a_skipped_colour_produces_no_steps_in_any_mode() {
    let palette = Palette::generic_5mm();
    let (grid, a, ..) = fixture(&palette);
    let options = StepOptions::skipping([a]);
    let expected: BTreeSet<Cell> = grid
        .iter()
        .filter(|(_, id)| **id != a)
        .map(|(cell, _)| cell)
        .collect();
    assert_eq!(expected.len(), 17);

    for mode in every_mode() {
        let plan = plan_steps(&grid, &palette, &mode, &options);
        assert_partitions(&plan, &expected);
        assert!(
            plan.groups.iter().all(|g| g.color != Some(a)),
            "{} still placed the skipped colour",
            plan.mode
        );
    }
}

/// Skipping everything is legal and produces nothing, rather than an empty
/// group per row or per board.
#[test]
fn skipping_every_colour_leaves_an_empty_plan() {
    let palette = Palette::generic_5mm();
    let (grid, a, b, c) = fixture(&palette);
    let options = StepOptions::skipping([a, b, c]);
    for mode in every_mode() {
        let plan = plan_steps(&grid, &palette, &mode, &options);
        assert!(plan.groups.is_empty(), "{} produced groups", plan.mode);
        assert_eq!(plan.total_cells(), 0);
    }
}

#[test]
fn placements_are_numbered_by_group() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::RowByRow,
        &StepOptions::default(),
    );
    let placements: Vec<(usize, Cell)> = plan.placements().collect();
    assert_eq!(placements.len(), 36);
    assert_eq!(placements[0], (0, Cell::new(0, 0)));
    assert_eq!(placements[6], (1, Cell::new(0, 1)));
    assert_eq!(placements[35], (5, Cell::new(5, 5)));
}
