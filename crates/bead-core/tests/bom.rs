//! Bill of materials, stock checking and ΔE < 3 substitutes.

use bead_core::bom::{check_stock, check_stock_within, Bom, Inventory, SUBSTITUTE_MAX_DELTA_E};
use bead_core::color::{ciede2000, Rgb};
use bead_core::grid::{Cell, Grid};
use bead_core::palette::{BeadColor, ColorId, Palette};

/// The same 6×6 fixture the step tests use: A 19, B 16, C 1.
fn fixture(palette: &Palette) -> Grid<ColorId> {
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
    grid
}

#[test]
fn the_bill_is_counted_commonest_first() {
    let palette = Palette::generic_5mm();
    let bom = Bom::from_grid(&fixture(&palette), &palette);

    assert_eq!(bom.palette, "generic-5mm");
    assert_eq!(bom.distinct_colors(), 3);
    assert_eq!(bom.total_beads(), 36);
    assert_eq!(
        bom.lines
            .iter()
            .map(|l| (l.code.as_str(), l.name.as_str(), l.count))
            .collect::<Vec<_>>(),
        vec![("G01", "White", 19), ("G33", "Blue", 16), ("G15", "Red", 1)]
    );
    assert_eq!(bom.count_of("G33"), 16);
    assert_eq!(bom.count_of("G99"), 0);
}

/// Two colours used the same number of times must not swap between runs.
#[test]
fn equal_counts_break_the_tie_on_code() {
    let palette = Palette::generic_5mm();
    let early = palette.find_code("G06").expect("G06");
    let late = palette.find_code("G22").expect("G22");
    let mut grid = Grid::filled(2, 1, late).expect("2x1");
    grid.set(Cell::new(0, 0), early);

    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(
        bom.lines
            .iter()
            .map(|l| l.code.as_str())
            .collect::<Vec<_>>(),
        vec!["G06", "G22"]
    );
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

/// A palette with near neighbours, which is what real brand colour cards look
/// like. `generic-5mm` is deliberately spread out — see `palette.rs` — so the
/// substitute rule needs a palette that actually has close colours in it.
fn close_palette() -> Palette {
    Palette::new(
        "test-close",
        vec![
            BeadColor::new("C01", "Stone", Rgb::new(0x80, 0x80, 0x80)),
            BeadColor::new("C02", "Ash", Rgb::new(0x84, 0x84, 0x84)),
            BeadColor::new("C03", "Pebble", Rgb::new(0x7B, 0x7B, 0x7B)),
            BeadColor::new("C04", "Ink", Rgb::new(0x10, 0x10, 0x10)),
        ],
    )
    .expect("valid palette")
}

#[test]
fn the_close_palette_is_actually_close() {
    let palette = close_palette();
    let stone = palette.color(palette.find_code("C01").expect("C01")).lab;
    for code in ["C02", "C03"] {
        let other = palette.color(palette.find_code(code).expect(code)).lab;
        let delta_e = ciede2000(stone, other);
        assert!(
            delta_e < SUBSTITUTE_MAX_DELTA_E,
            "{code} was ΔE00 {delta_e}"
        );
    }
    let ink = palette.color(palette.find_code("C04").expect("C04")).lab;
    assert!(ciede2000(stone, ink) > 10.0, "C04 is not a stand-in");
}

#[test]
fn a_missing_colour_offers_its_nearest_in_stock_neighbours() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let grid = Grid::filled(4, 4, stone).expect("4x4");
    let bom = Bom::from_grid(&grid, &palette);
    // No C01 at all, but plenty of two colours nobody could tell from it.
    let inventory = Inventory::from_pairs([("C02", 50usize), ("C03", 50), ("C04", 50)]);

    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert_eq!(shortage.missing, 16);

    let offered: Vec<&str> = shortage
        .substitutes
        .iter()
        .map(|s| s.code.as_str())
        .collect();
    // C02 is four code values from C01 and C03 is five, so C02 leads.
    assert_eq!(offered, vec!["C02", "C03"], "nearest first");
    assert!(shortage.substitutes.iter().all(|s| s.available == 50));
    for pair in shortage.substitutes.windows(2) {
        assert!(pair[0].delta_e <= pair[1].delta_e);
    }
    assert!(
        shortage
            .substitutes
            .iter()
            .all(|s| s.delta_e < SUBSTITUTE_MAX_DELTA_E),
        "nothing at or beyond the threshold is offered"
    );
}

/// Suggesting a colour the same pattern is about to use up just moves the
/// shortage, so only the surplus counts as available.
#[test]
fn a_colour_the_pattern_already_needs_is_not_offered() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let ash = palette.find_code("C02").expect("C02");
    let mut grid = Grid::filled(4, 4, stone).expect("4x4");
    for x in 0..4u32 {
        grid.set(Cell::new(x, 0), ash);
    }
    let bom = Bom::from_grid(&grid, &palette);
    assert_eq!(bom.count_of("C01"), 12);
    assert_eq!(bom.count_of("C02"), 4);

    // Exactly enough C02 for the pattern's own use, and spare C03.
    let inventory = Inventory::from_pairs([("C02", 4usize), ("C03", 30)]);
    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert_eq!(
        shortage
            .substitutes
            .iter()
            .map(|s| s.code.as_str())
            .collect::<Vec<_>>(),
        vec!["C03"],
        "C02 is spoken for"
    );

    // One spare bead is enough to be worth mentioning.
    let inventory = Inventory::from_pairs([("C02", 5usize), ("C03", 30)]);
    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    let ash_offer = shortage
        .substitutes
        .iter()
        .find(|s| s.code == "C02")
        .expect("C02 has one spare");
    assert_eq!(ash_offer.available, 1);
}

/// A shortage with nothing close enough on the shelf gets an empty list rather
/// than the least-bad colour in the drawer.
#[test]
fn nothing_close_enough_offers_nothing() {
    let palette = close_palette();
    let stone = palette.find_code("C01").expect("C01");
    let grid = Grid::filled(2, 2, stone).expect("2x2");
    let bom = Bom::from_grid(&grid, &palette);
    let inventory = Inventory::from_pairs([("C04", 500usize)]);

    let check = check_stock(&bom, &palette, &inventory);
    let shortage = check.shortage_of("C01").expect("C01 is short");
    assert!(shortage.substitutes.is_empty());
}

/// `generic-5mm` has no pair inside 3, so widening the threshold is the only
/// way to see a substitute in it — which is exactly what the fixture's
/// separation is supposed to mean.
#[test]
fn a_widened_threshold_reaches_the_spread_out_fixture() {
    let palette = Palette::generic_5mm();
    let light_brown = palette.find_code("G43").expect("G43");
    let grid = Grid::filled(3, 3, light_brown).expect("3x3");
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
