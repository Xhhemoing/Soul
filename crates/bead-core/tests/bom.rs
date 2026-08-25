//! Bill of materials, stock checking and ΔE00 < 3 substitutes.
//!
//! Covers T-BOM-1 to T-BOM-3 and T-SUB-1 to T-SUB-3 from
//! `docs/bead/reviews/round1-algorithms.md`.

mod support;

use bead_core::bom::{check_stock, check_stock_within, Bom, Inventory, SUBSTITUTE_MAX_DELTA_E};
use bead_core::color::{ciede2000, within_delta_e, Lab, Rgb};
use bead_core::grid::{Cell, Grid};
use bead_core::palette::{BeadColor, ColorId, Palette};
use bead_core::quantize::PatternGrid;

/// The same 6×6 fixture the step tests use: A 19, B 16, C 1.
fn fixture(palette: &Palette) -> PatternGrid {
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
    grid
}

/// T-BOM-1: the exact rows, and the invariant that ties the bill to the step
/// plans — every bead counted is a bead somebody has to place.
#[test]
fn the_bill_is_counted_commonest_first() {
    let palette = Palette::generic_5mm();
    let grid = fixture(&palette);
    let bom = Bom::from_grid(&grid, &palette);

    assert_eq!(bom.palette, "generic-5mm");
    assert_eq!(bom.distinct_colors(), 3);
    assert_eq!(
        bom.lines
            .iter()
            .map(|l| (l.code.as_str(), l.name.as_str(), l.count))
            .collect::<Vec<_>>(),
        vec![("G01", "White", 19), ("G33", "Blue", 16), ("G15", "Red", 1)]
    );

    let filled = grid.iter().filter(|(_, slot)| slot.is_some()).count();
    assert_eq!(bom.total_beads(), filled);
    assert_eq!(bom.total_beads(), 36);
    assert_eq!(bom.count_of("G33"), 16);
    assert_eq!(bom.count_of("G99"), 0);
}

/// Empty cells are holes in the board, not beads, so they are not on the bill.
#[test]
fn empty_cells_are_not_counted() {
    let palette = Palette::generic_5mm();
    let mut grid = fixture(&palette);
    for x in 0..6u32 {
        grid.set(Cell::new(x, 5), None);
    }
    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(bom.total_beads(), 30);
    assert_eq!(bom.count_of("G01"), 13);
}

/// T-BOM-2: the degenerate patterns.
#[test]
fn an_empty_pattern_has_an_empty_bill() {
    let palette = Palette::generic_5mm();

    let nothing: PatternGrid = Grid::filled(0, 0, None);
    let bom = Bom::from_grid(&nothing, &palette);
    assert!(bom.lines.is_empty());
    assert_eq!(bom.total_beads(), 0);

    let all_empty: PatternGrid = Grid::filled(9, 4, None);
    let bom = Bom::from_grid(&all_empty, &palette);
    assert!(bom.lines.is_empty());
    assert_eq!(bom.total_beads(), 0);

    let one_colour: PatternGrid = Grid::filled(3, 3, Some(palette.find_code("G22").expect("G22")));
    let bom = Bom::from_grid(&one_colour, &palette);
    assert_eq!(bom.distinct_colors(), 1);
    assert_eq!(bom.lines[0].count, 9);
}

/// T-BOM-3: two colours used the same number of times must not swap between
/// runs, so the tie goes to the lower palette index.
#[test]
fn equal_counts_break_the_tie_on_the_palette_index() {
    let palette = Palette::generic_5mm();
    let early = palette.find_code("G06").expect("G06");
    let late = palette.find_code("G22").expect("G22");
    assert!(early < late);

    // Written late-first so a stable sort on insertion order would fail here.
    let grid: PatternGrid =
        Grid::from_vec(4, 1, vec![Some(late), Some(late), Some(early), Some(early)]).expect("4x1");
    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(
        bom.lines
            .iter()
            .map(|l| (l.code.as_str(), l.count))
            .collect::<Vec<_>>(),
        vec![("G06", 2), ("G22", 2)]
    );
}

/// An id from some other palette is reported rather than dropped, so a mismatch
/// shows up as a strange line on the bill instead of a quietly short one.
#[test]
fn an_unknown_id_is_still_counted() {
    let palette = Palette::generic_5mm();
    let grid: PatternGrid = Grid::filled(2, 1, Some(ColorId(9_000)));
    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(bom.lines.len(), 1);
    assert_eq!(bom.lines[0].code, "#9000");
    assert_eq!(bom.lines[0].name, "unknown");
    assert_eq!(bom.total_beads(), 2);
}

#[test]
fn a_full_shelf_needs_nothing() {
    let palette = Palette::generic_5mm();
    let bom = Bom::from_grid(&fixture(&palette), &palette);
    let inventory = Inventory::from_pairs([("G01", 100usize), ("G33", 100), ("G15", 100)]);

    let check = check_stock(&bom, &palette, &inventory);
    assert!(check.is_satisfied());
    assert_eq!(check.missing_beads, 0);
    assert!(check.shortages.is_empty());
}

#[test]
fn a_partial_shelf_reports_only_the_gap() {
    let palette = Palette::generic_5mm();
    let bom = Bom::from_grid(&fixture(&palette), &palette);
    let inventory = Inventory::from_pairs([("G01", 10usize), ("G33", 100), ("G15", 0)]);

    let check = check_stock(&bom, &palette, &inventory);
    assert!(!check.is_satisfied());
    assert_eq!(check.missing_beads, 9 + 1);

    let white = check.shortage_of("G01").expect("G01 is short");
    assert_eq!((white.needed, white.on_hand, white.missing), (19, 10, 9));
    let red = check.shortage_of("G15").expect("G15 is short");
    assert_eq!((red.needed, red.on_hand, red.missing), (1, 0, 1));
    assert!(check.shortage_of("G33").is_none(), "blue is covered");
}

/// A colour absent from the inventory entirely is a shortage of the whole
/// amount, not a silent zero.
#[test]
fn a_colour_that_was_never_stocked_is_still_a_shortage() {
    let palette = Palette::generic_5mm();
    let bom = Bom::from_grid(&fixture(&palette), &palette);
    let check = check_stock(&bom, &palette, &Inventory::new());
    assert_eq!(check.shortages.len(), 3);
    assert_eq!(check.missing_beads, 36);
}

/// The threshold is asserted rather than assumed: 3 is what the product
/// promises, and `SUBSTITUTE_MAX_DELTA_E` is the only place it is written down.
#[test]
fn the_substitute_threshold_is_three() {
    assert_eq!(SUBSTITUTE_MAX_DELTA_E, 3.0);
}

/// T-SUB-1, first half: the boundary on the Lab-level predicate both the
/// palette search and the substitute search go through.
///
/// Rows 2 and 3 of the Sharma table straddle 3 by design — 2.8615 and 3.4412 —
/// so they say which side of the comparison the implementation is on without
/// depending on any palette or any sRGB conversion. The exactly-3 case is
/// spelled out too, because "below 3" and "at most 3" only differ there.
#[test]
fn the_substitute_boundary_is_strictly_below_three() {
    let sharma_2 = (
        Lab::new(50.0, 3.1571, -77.2803),
        Lab::new(50.0, 0.0, -82.7485),
    );
    let sharma_3 = (
        Lab::new(50.0, 2.8361, -74.0200),
        Lab::new(50.0, 0.0, -82.7485),
    );
    assert!((ciede2000(sharma_2.0, sharma_2.1) - 2.8615).abs() < 1e-4);
    assert!((ciede2000(sharma_3.0, sharma_3.1) - 3.4412).abs() < 1e-4);

    assert!(within_delta_e(
        sharma_2.0,
        sharma_2.1,
        SUBSTITUTE_MAX_DELTA_E
    ));
    assert!(!within_delta_e(
        sharma_3.0,
        sharma_3.1,
        SUBSTITUTE_MAX_DELTA_E
    ));

    let grey = Lab::new(50.0, 0.0, 0.0);
    assert!(!within_delta_e(grey, grey, 0.0), "at the threshold is out");
    assert!(within_delta_e(grey, grey, f64::MIN_POSITIVE));
}

/// A palette with near neighbours, which is what real brand colour cards look
/// like. `generic-5mm` is deliberately spread out — see `palette.rs` — so the
/// substitute rule needs a palette that actually has close colours in it.
///
/// `C02` and `C03` are the oracle-found pair that straddles the threshold from
/// `C01`: 2.9999 and 3.0021. `C05` is `C01`'s colour under another code, which
/// is what an inventory of two brands looks like.
fn close_palette() -> Palette {
    Palette::new(
        "test-close",
        vec![
            BeadColor::new("C01", "Stone", Rgb::new(0x80, 0x80, 0x80)),
            BeadColor::new("C02", "Ash", Rgb::new(0x7A, 0x7A, 0x77)),
            BeadColor::new("C03", "Slate", Rgb::new(0x81, 0x83, 0x88)),
            BeadColor::new("C04", "Ink", Rgb::new(0x10, 0x10, 0x10)),
            BeadColor::new("C05", "Echo", Rgb::new(0x80, 0x80, 0x80)),
        ],
    )
    .expect("valid palette")
}

fn delta_from_stone(palette: &Palette, code: &str) -> f64 {
    let stone = palette.color(palette.find_code("C01").expect("C01")).lab;
    let other = palette.color(palette.find_code(code).expect(code)).lab;
    ciede2000(stone, other)
}

/// T-SUB-1, second half: the same boundary through the whole stock check, on
/// two colours the oracle placed either side of it by a couple of thousandths.
#[test]
fn the_close_palette_straddles_the_threshold() {
    let palette = close_palette();
    assert!((delta_from_stone(&palette, "C02") - 2.9999).abs() < 1e-3);
    assert!((delta_from_stone(&palette, "C03") - 3.0021).abs() < 1e-3);
    assert!(delta_from_stone(&palette, "C02") < 3.0);
    assert!(delta_from_stone(&palette, "C03") > 3.0);
    assert_eq!(delta_from_stone(&palette, "C05"), 0.0);
    assert!(delta_from_stone(&palette, "C04") > 10.0);
}

#[test]
fn only_the_colour_below_the_threshold_is_offered() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let grid: PatternGrid = Grid::filled(4, 4, Some(stone));
    let bom = Bom::from_grid(&grid, &palette);
    // Everything in stock except the colour the pattern actually needs.
    let inventory = Inventory::from_pairs([("C02", 50usize), ("C03", 50), ("C04", 50)]);

    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert_eq!(shortage.missing, 16);
    assert_eq!(
        shortage
            .substitutes
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>(),
        vec!["C02"],
        "C03 is 0.0021 too far"
    );
    assert!(shortage.substitutes[0].available == 50);
}

/// T-SUB-2: nearest first, ties on the palette index, and the same answer twice.
#[test]
fn substitutes_are_sorted_and_deterministic() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let grid: PatternGrid = Grid::filled(4, 4, Some(stone));
    let bom = Bom::from_grid(&grid, &palette);
    let inventory = Inventory::from_pairs([("C02", 50usize), ("C03", 50), ("C05", 50)]);

    let first = check_stock(&bom, &palette, &inventory);
    let second = check_stock(&bom, &palette, &inventory);
    assert_eq!(first, second, "two runs, one answer");

    let shortage = first.shortage_of("C01").expect("C01 is short");
    assert_eq!(
        shortage
            .substitutes
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>(),
        vec!["C05", "C02"],
        "the identical colour first, then the near one"
    );
    for pair in shortage.substitutes.windows(2) {
        assert!(pair[0].delta_e <= pair[1].delta_e);
        if pair[0].delta_e == pair[1].delta_e {
            assert!(pair[0].id < pair[1].id, "equal distance sorts on the index");
        }
    }
}

/// T-SUB-3, second half: a colour identical to the missing one is a
/// zero-distance substitute like any other. Whether "the same colour under
/// another code" should count as having it in stock is a question for whoever
/// is looking at two bags of beads; the core function does not decide it.
#[test]
fn an_identical_colour_under_another_code_leads_the_list() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let grid: PatternGrid = Grid::filled(2, 2, Some(stone));
    let bom = Bom::from_grid(&grid, &palette);
    let inventory = Inventory::from_pairs([("C05", 10usize)]);

    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert_eq!(shortage.missing, 4);
    assert_eq!(shortage.substitutes.len(), 1);
    assert_eq!(shortage.substitutes[0].code, "C05");
    assert_eq!(shortage.substitutes[0].delta_e, 0.0);
}

/// Suggesting a colour the same pattern is about to use up just moves the
/// shortage, so only the surplus counts as available.
#[test]
fn a_colour_the_pattern_already_needs_is_not_offered() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let ash = palette.find_code("C02").expect("C02");
    let mut grid: PatternGrid = Grid::filled(4, 4, Some(stone));
    for x in 0..4u32 {
        grid.set(Cell::new(x, 0), Some(ash));
    }
    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(bom.count_of("C01"), 12);
    assert_eq!(bom.count_of("C02"), 4);

    // Exactly enough C02 for the pattern's own use, and spare C05.
    let inventory = Inventory::from_pairs([("C02", 4usize), ("C05", 30)]);
    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert_eq!(
        shortage
            .substitutes
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>(),
        vec!["C05"],
        "C02 is spoken for"
    );

    // One spare bead is enough to be worth mentioning.
    let inventory = Inventory::from_pairs([("C02", 5usize), ("C05", 30)]);
    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    let ash_offer = shortage
        .substitutes
        .iter()
        .find(|s| s.code == "C02")
        .expect("C02 has one spare");
    assert_eq!(ash_offer.available, 1);
}

/// T-SUB-3, first half: a shortage with nothing close enough on the shelf gets
/// an empty list rather than the least-bad colour in the drawer, and an empty
/// list is not an error.
#[test]
fn nothing_close_enough_offers_nothing() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let grid: PatternGrid = Grid::filled(2, 2, Some(stone));
    let bom = Bom::from_grid(&grid, &palette);
    let inventory = Inventory::from_pairs([("C04", 500usize)]);

    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert!(shortage.substitutes.is_empty());
    assert_eq!(shortage.missing, 4);
}

/// `generic-5mm` has no pair inside 3, so widening the threshold is the only
/// way to see a substitute in it — which is exactly what the fixture's
/// separation is supposed to mean.
#[test]
fn a_widened_threshold_reaches_the_spread_out_fixture() {
    let palette = Palette::generic_5mm();
    let light_brown = palette.find_code("G43").expect("G43");
    let grid: PatternGrid = Grid::filled(3, 3, Some(light_brown));
    let bom = Bom::from_grid(&grid, &palette);
    let inventory = Inventory::from_pairs([("G48", 100usize), ("G01", 100)]);

    let strict = check_stock(&bom, &palette, &inventory);
    assert!(strict
        .shortage_of("G43")
        .expect("short")
        .substitutes
        .is_empty());

    let relaxed = check_stock_within(&bom, &palette, &inventory, 6.0);
    let shortage = relaxed.shortage_of("G43").expect("short");
    assert_eq!(
        shortage
            .substitutes
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>(),
        vec!["G48"]
    );
    assert!((shortage.substitutes[0].delta_e - 5.3575).abs() < 0.001);
}

#[test]
fn inventory_accumulates_and_reports() {
    let mut inventory = Inventory::new();
    assert!(inventory.is_empty());
    inventory.add("G01", 10);
    inventory.add("G01", 5);
    inventory.set("G02", 3);
    assert_eq!(inventory.on_hand("G01"), 15);
    assert_eq!(inventory.on_hand("G02"), 3);
    assert_eq!(inventory.codes().collect::<Vec<_>>(), vec!["G01", "G02"]);
}

/// The bill counts what the plans place, whichever plan is used. Same invariant
/// as T-SPL-0, asserted from the other end.
#[test]
fn the_bill_agrees_with_every_step_plan() {
    use bead_core::fit::BoardSpec;
    use bead_core::steps::{plan_steps, ColorOrder, StepMode};

    let palette = Palette::generic_5mm();
    let grid = support::grid_from_codes(
        &palette,
        4,
        &[
            "G01 G01 .   G15",
            "G01 .   G15 G15",
            ".   G33 G33 .",
            "G33 G33 .   G01",
        ],
    );
    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(bom.total_beads(), 11);

    for mode in [
        StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
        StepMode::Tile {
            board: BoardSpec::new("2x2", 2, 2),
        },
        StepMode::OutlineInfill,
        StepMode::RowByRow,
    ] {
        let plan = plan_steps(&grid, &palette, &mode);
        assert_eq!(plan.total_cells(), bom.total_beads(), "{}", plan.mode);
    }
}
