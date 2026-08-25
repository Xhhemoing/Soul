//! The four assembly orders.
//!
//! Covers T-SPL-0 to T-SPL-2, T-CBC-1/2, T-TIL-1/2, T-OUT-1 to T-OUT-4 and
//! T-ROW-1 from `docs/bead/reviews/round1-algorithms.md`.

mod support;

use std::collections::BTreeSet;

use bead_core::fit::BoardSpec;
use bead_core::grid::{Cell, Grid};
use bead_core::palette::{ColorId, Palette};
use bead_core::quantize::PatternGrid;
use bead_core::steps::{plan_steps, ColorOrder, Phase, StepMode, StepPlan};

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
fn fixture(palette: &Palette) -> (PatternGrid, ColorId, ColorId, ColorId) {
    let a = palette.find_code("G01").expect("G01");
    let b = palette.find_code("G33").expect("G33");
    let c = palette.find_code("G15").expect("G15");

    let mut grid: PatternGrid = Grid::filled(6, 6, Some(a));
    for y in 1..5u32 {
        for x in 1..5u32 {
            grid.set(Cell::new(x, y), Some(b));
        }
    }
    grid.set(Cell::new(0, 0), Some(c));
    (grid, a, b, c)
}

/// Every mode has to partition the filled cells: each one exactly once, none
/// invented, no empty cell placed. Checked for every mode rather than asserted
/// once, because a partition is the property the progress bar depends on.
fn assert_partitions(plan: &StepPlan, expected: &BTreeSet<Cell>) {
    let seen: Vec<Cell> = plan.placements().map(|(_, cell)| cell).collect();
    assert_eq!(
        seen.len(),
        expected.len(),
        "{} placed {} cells, expected {}",
        plan.mode,
        seen.len(),
        expected.len()
    );
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

fn filled_cells(grid: &PatternGrid) -> BTreeSet<Cell> {
    grid.iter()
        .filter(|(_, slot)| slot.is_some())
        .map(|(cell, _)| cell)
        .collect()
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
    let expected = filled_cells(&grid);
    for mode in every_mode() {
        let plan = plan_steps(&grid, &palette, &mode);
        assert_partitions(&plan, &expected);
    }
}

/// T-SPL-0: the partition property, on a hundred pseudo-random grids that all
/// contain empty cells. The seed is fixed, so a failure here is reproducible
/// rather than a story about a build that went red once.
#[test]
fn the_partition_holds_on_random_grids() {
    let palette = Palette::generic_5mm();
    let mut rng = support::Lcg::new(0x5EED_1234);

    for case in 0..100u32 {
        let width = 1 + rng.below(9);
        let height = 1 + rng.below(9);
        let cells: Vec<Option<ColorId>> = (0..width * height)
            .map(|_| {
                // A third empty on average: enough that most grids have holes,
                // few enough that most grids still have regions.
                if rng.below(3) == 0 {
                    None
                } else {
                    Some(ColorId(rng.below(48) as u16))
                }
            })
            .collect();
        let grid: PatternGrid = Grid::from_vec(width, height, cells).expect("rectangular");
        let expected = filled_cells(&grid);

        for mode in every_mode() {
            let plan = plan_steps(&grid, &palette, &mode);
            assert_partitions(&plan, &expected);
            assert!(
                plan.placements()
                    .all(|(_, cell)| matches!(grid.get(cell), Some(Some(_)))),
                "case {case}: {} placed a bead on an empty cell",
                plan.mode
            );
        }
    }
}

/// T-SPL-1: nothing to do is not an error.
#[test]
fn an_empty_pattern_produces_no_steps() {
    let palette = Palette::generic_5mm();
    let nothing: PatternGrid = Grid::filled(0, 0, None);
    let all_empty: PatternGrid = Grid::filled(7, 5, None);

    for grid in [nothing, all_empty] {
        for mode in every_mode() {
            let plan = plan_steps(&grid, &palette, &mode);
            assert!(
                plan.groups.is_empty(),
                "{} produced groups for an empty pattern",
                plan.mode
            );
            assert_eq!(plan.total_cells(), 0);
        }
    }
}

/// T-SPL-2: two runs, byte-identical plans. The plan derives from `BTreeMap`
/// and explicit sorts rather than hash iteration, and this is what says so.
#[test]
fn planning_twice_gives_the_same_plan() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    for mode in every_mode() {
        let first = plan_steps(&grid, &palette, &mode);
        let second = plan_steps(&grid, &palette, &mode);
        assert_eq!(first, second, "{} is not deterministic", first.mode);
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

/// T-CBC-1: counts {A: 1, B: 5, C: 5} with B's palette index below C's. The
/// primary key flips with the order; the tie between B and C does not.
#[test]
fn equal_counts_always_break_on_the_palette_index() {
    let palette = Palette::generic_5mm();
    let a = palette.find_code("G22").expect("G22");
    let b = palette.find_code("G06").expect("G06");
    let c = palette.find_code("G15").expect("G15");
    assert!(b < c, "the fixture needs B to sort before C");

    let mut cells = vec![Some(a)];
    cells.extend(std::iter::repeat_n(Some(b), 5));
    cells.extend(std::iter::repeat_n(Some(c), 5));
    let grid: PatternGrid = Grid::from_vec(11, 1, cells).expect("11x1");

    let accent = plan_steps(
        &grid,
        &palette,
        &StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
    );
    assert_eq!(
        accent.groups.iter().map(|g| g.color).collect::<Vec<_>>(),
        vec![Some(a), Some(b), Some(c)]
    );

    let bulk = plan_steps(
        &grid,
        &palette,
        &StepMode::ColorByColor {
            order: ColorOrder::BulkFirst,
        },
    );
    assert_eq!(
        bulk.groups.iter().map(|g| g.color).collect::<Vec<_>>(),
        vec![Some(b), Some(c), Some(a)]
    );
}

/// T-CBC-2: one colour, one step, cells in reading order.
#[test]
fn a_single_colour_pattern_is_one_step_in_reading_order() {
    let palette = Palette::generic_5mm();
    let only = palette.find_code("G26").expect("G26");
    let grid: PatternGrid = Grid::filled(3, 2, Some(only));
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
    );
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(
        plan.groups[0].cells,
        vec![
            Cell::new(0, 0),
            Cell::new(1, 0),
            Cell::new(2, 0),
            Cell::new(0, 1),
            Cell::new(1, 1),
            Cell::new(2, 1),
        ]
    );
}

/// T-TIL-1: four 28×28 boards out of a full 56×56 pattern, in the order they
/// sit on the table.
#[test]
fn a_56_square_pattern_is_four_28_boards() {
    let palette = Palette::generic_5mm();
    let grid: PatternGrid = Grid::filled(56, 56, Some(palette.find_code("G01").expect("G01")));
    let plan = plan_steps(
        &grid,
        &palette,
        &StepMode::Tile {
            board: BoardSpec::square_28(),
        },
    );

    assert_eq!(plan.mode, "tile");
    assert_eq!(plan.groups.len(), 4);
    assert!(plan.groups.iter().all(|g| g.len() == 784));
    assert_eq!(
        plan.groups.iter().map(|g| g.cells[0]).collect::<Vec<_>>(),
        vec![
            Cell::new(0, 0),
            Cell::new(28, 0),
            Cell::new(0, 28),
            Cell::new(28, 28)
        ]
    );
    assert_eq!(plan.groups[0].label, "Board 28x28 r1c1");
    assert_eq!(plan.groups[3].label, "Board 28x28 r2c2");
    // Reading order within a board, not across the whole pattern.
    assert_eq!(plan.groups[1].cells[1], Cell::new(29, 0));
    assert_eq!(plan.groups[1].cells[28], Cell::new(28, 1));
}

/// T-TIL-2: a pattern that does not divide evenly leaves partial boards, and
/// they stay partial. Padding them out would tell someone to place beads that
/// are not in the picture.
#[test]
fn partial_boards_are_not_padded() {
    let palette = Palette::generic_5mm();
    let colour = palette.find_code("G01").expect("G01");
    let mut grid: PatternGrid = Grid::filled(30, 29, Some(colour));
    let mode = StepMode::Tile {
        board: BoardSpec::square_28(),
    };

    let plan = plan_steps(&grid, &palette, &mode);
    assert_eq!(
        plan.groups.iter().map(|g| g.len()).collect::<Vec<_>>(),
        vec![28 * 28, 2 * 28, 28, 2],
        "the four boards are 28×28, 2×28, 28×1 and 2×1"
    );

    // Empty the bottom-right board and it stops being a step at all.
    grid.set(Cell::new(28, 28), None);
    grid.set(Cell::new(29, 28), None);
    let plan = plan_steps(&grid, &palette, &mode);
    assert_eq!(
        plan.groups.iter().map(|g| g.len()).collect::<Vec<_>>(),
        vec![28 * 28, 2 * 28, 28],
        "a board with nothing on it is skipped rather than shown empty"
    );
}

/// T-ROW-1: one row per non-empty row, top to bottom, left to right.
#[test]
fn rows_run_top_to_bottom_and_skip_the_empty_ones() {
    let palette = Palette::generic_5mm();
    let colour = palette.find_code("G01").expect("G01");
    let mut grid: PatternGrid = Grid::filled(4, 3, Some(colour));
    for x in 0..4u32 {
        grid.set(Cell::new(x, 1), None);
    }
    grid.set(Cell::new(0, 2), None);

    let plan = plan_steps(&grid, &palette, &StepMode::RowByRow);
    assert_eq!(plan.mode, "row-by-row");
    assert_eq!(plan.groups.len(), 2, "the blank row produces no step");
    assert_eq!(plan.groups[0].label, "Row 1");
    assert_eq!(
        plan.groups[0].cells,
        vec![
            Cell::new(0, 0),
            Cell::new(1, 0),
            Cell::new(2, 0),
            Cell::new(3, 0)
        ]
    );
    assert_eq!(plan.groups[1].label, "Row 3");
    assert_eq!(
        plan.groups[1].cells,
        vec![Cell::new(1, 2), Cell::new(2, 2), Cell::new(3, 2)]
    );
}

/// T-OUT-1: a solid 4×4 — the twelve cells that touch the outside, then the
/// four in the middle. There is no hole, so nothing is an inner edge.
#[test]
fn a_solid_block_is_its_border_then_its_middle() {
    let palette = Palette::generic_5mm();
    let grid: PatternGrid = Grid::filled(4, 4, Some(palette.find_code("G01").expect("G01")));
    let plan = plan_steps(&grid, &palette, &StepMode::OutlineInfill);

    assert_eq!(plan.mode, "outline-infill");
    assert_eq!(
        plan.groups
            .iter()
            .map(|g| (g.phase, g.len()))
            .collect::<Vec<_>>(),
        vec![(Some(Phase::Outline), 12), (Some(Phase::Fill), 4)]
    );
    assert_eq!(plan.groups[0].label, "Region 1 outline");
    assert_eq!(plan.groups[0].cells[0], Cell::new(0, 0));
    assert_eq!(
        plan.groups[1].cells,
        vec![
            Cell::new(1, 1),
            Cell::new(2, 1),
            Cell::new(1, 2),
            Cell::new(2, 2)
        ]
    );
    assert_eq!(Phase::Outline.slug(), "outline");
    assert_eq!(Phase::InnerEdge.slug(), "inner-edge");
    assert_eq!(Phase::Fill.slug(), "fill");
}

/// T-OUT-2: a ring two beads thick. The outer circuit is the outline, the ring
/// facing the hole is the inner edge, and there is no middle left over.
#[test]
fn a_ring_has_an_outer_and_an_inner_edge() {
    let palette = Palette::generic_5mm();
    let colour = palette.find_code("G33").expect("G33");
    let mut grid: PatternGrid = Grid::filled(7, 7, Some(colour));
    for y in 2..5u32 {
        for x in 2..5u32 {
            grid.set(Cell::new(x, y), None);
        }
    }

    let plan = plan_steps(&grid, &palette, &StepMode::OutlineInfill);
    assert_eq!(
        plan.groups
            .iter()
            .map(|g| (g.phase, g.len()))
            .collect::<Vec<_>>(),
        vec![(Some(Phase::Outline), 24), (Some(Phase::InnerEdge), 16)]
    );
}

/// The same shape one bead thick: outline and inner edge coincide, and G4 says
/// outline wins, so every cell is placed exactly once.
#[test]
fn a_one_bead_ring_is_all_outline() {
    let palette = Palette::generic_5mm();
    let colour = palette.find_code("G33").expect("G33");
    let mut grid: PatternGrid = Grid::filled(5, 5, Some(colour));
    for y in 1..4u32 {
        for x in 1..4u32 {
            grid.set(Cell::new(x, y), None);
        }
    }

    let plan = plan_steps(&grid, &palette, &StepMode::OutlineInfill);
    assert_eq!(
        plan.groups
            .iter()
            .map(|g| (g.phase, g.len()))
            .collect::<Vec<_>>(),
        vec![(Some(Phase::Outline), 16)]
    );
    assert_partitions(&plan, &filled_cells(&grid));
}

/// T-OUT-3: diagonal touching is not connection. Fuse beads sit in a square
/// lattice, and a bead resting on a corner is not holding anything up.
#[test]
fn regions_are_four_connected() {
    let palette = Palette::generic_5mm();
    let colour = palette.find_code("G15").expect("G15");
    let mut grid: PatternGrid = Grid::filled(2, 2, None);
    grid.set(Cell::new(0, 0), Some(colour));
    grid.set(Cell::new(1, 1), Some(colour));

    let plan = plan_steps(&grid, &palette, &StepMode::OutlineInfill);
    assert_eq!(plan.groups.len(), 2, "two regions, not one");
    assert_eq!(plan.groups[0].cells, vec![Cell::new(0, 0)]);
    assert_eq!(plan.groups[1].cells, vec![Cell::new(1, 1)]);
}

/// T-OUT-4: regions come in the order of their topmost-then-leftmost cell.
#[test]
fn regions_are_ordered_by_their_first_cell() {
    let palette = Palette::generic_5mm();
    let left = palette.find_code("G15").expect("G15");
    let right = palette.find_code("G26").expect("G26");
    // Two 2×2 blobs, the right one a row higher than the left one.
    let grid = support::grid_from_codes(
        &palette,
        6,
        &[
            ".  .  .  .  G26 G26",
            ".  .  .  .  G26 G26",
            "G15 G15 .  .  .   .",
            "G15 G15 .  .  .   .",
        ],
    );

    let plan = plan_steps(&grid, &palette, &StepMode::OutlineInfill);
    assert_eq!(plan.groups.len(), 2);
    assert_eq!(
        plan.groups[0].color,
        Some(right),
        "the higher blob is first"
    );
    assert_eq!(plan.groups[0].cells[0], Cell::new(4, 0));
    assert_eq!(plan.groups[1].color, Some(left));
    assert_eq!(plan.groups[1].cells[0], Cell::new(0, 2));
}

/// A region is a run of touching beads whatever their colours: what holds a
/// fused panel together is the beads meeting, not the beads matching. So the
/// fixture's three colours form one region, and its groups have no single
/// colour to name.
#[test]
fn a_region_spans_the_colours_that_touch() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let plan = plan_steps(&grid, &palette, &StepMode::OutlineInfill);
    assert_eq!(
        plan.groups
            .iter()
            .map(|g| (g.phase, g.len(), g.color.is_some()))
            .collect::<Vec<_>>(),
        vec![
            (Some(Phase::Outline), 20, false),
            (Some(Phase::Fill), 16, true),
        ]
    );
    assert_partitions(&plan, &filled_cells(&grid));
}

#[test]
fn placements_are_numbered_by_group() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let plan = plan_steps(&grid, &palette, &StepMode::RowByRow);
    let placements: Vec<(usize, Cell)> = plan.placements().collect();
    assert_eq!(placements.len(), 36);
    assert_eq!(placements[0], (0, Cell::new(0, 0)));
    assert_eq!(placements[6], (1, Cell::new(0, 1)));
    assert_eq!(placements[35], (5, Cell::new(5, 5)));
}

#[test]
fn every_mode_has_a_stable_slug() {
    let palette = Palette::generic_5mm();
    let (grid, ..) = fixture(&palette);
    let slugs: Vec<&str> = every_mode()
        .iter()
        .map(|mode| plan_steps(&grid, &palette, mode).mode)
        .collect();
    assert_eq!(
        slugs,
        vec![
            "color-by-color",
            "color-by-color",
            "tile",
            "outline-infill",
            "row-by-row"
        ]
    );
}
