//! The whole conversion, end to end, and the defaults it picks.

mod support;

use bead_core::bom::{check_stock, Inventory};
use bead_core::color::{Rgb, Rgba};
use bead_core::detect::ImageKind;
use bead_core::fit::{BoardSpec, FitError, FitMode, Sampling};
use bead_core::grid::Cell;
use bead_core::image::Image;
use bead_core::palette::Palette;
use bead_core::pipeline::{to_pattern, PatternOptions};
use bead_core::quantize::Dither;
use bead_core::steps::{plan_steps, ColorOrder, StepMode};

/// A 28×28 drawing in four palette-exact colours, and the same drawing exported
/// at 4×.
fn exported_sprite() -> (Image, Image) {
    let palette = Palette::generic_5mm();
    let border = palette.color(palette.find_code("G06").expect("G06")).rgb;
    let body = palette.color(palette.find_code("G22").expect("G22")).rgb;
    let eye = palette.color(palette.find_code("G33").expect("G33")).rgb;
    let mouth = palette.color(palette.find_code("G15").expect("G15")).rgb;

    let mut pixels = Vec::with_capacity(28 * 28);
    for y in 0..28u32 {
        for x in 0..28u32 {
            pixels.push(if x == 0 || y == 0 || x == 27 || y == 27 {
                border
            } else if (y == 9 || y == 10) && (x == 9 || x == 18) {
                eye
            } else if y == 19 && (7..=20).contains(&x) {
                mouth
            } else {
                body
            });
        }
    }
    let original = support::opaque(28, 28, &pixels);
    let exported = support::upscale(&original, 4);
    (original, exported)
}

fn one_board() -> PatternOptions {
    PatternOptions::new(FitMode::FixedBoards {
        board: BoardSpec::square_28(),
        cols: 1,
        rows: 1,
    })
}

/// The end-to-end property that makes the detector worth having: a drawing
/// exported at 4× round-trips back to exactly the beads it was drawn with.
#[test]
fn an_exported_sprite_round_trips_to_its_own_colours() {
    let palette = Palette::generic_5mm();
    let (original, exported) = exported_sprite();
    let pattern = to_pattern(&exported, &palette, &one_board()).expect("a pattern");

    assert_eq!(pattern.analysis.kind, ImageKind::PixelArt { block_size: 4 });
    assert_eq!(pattern.sampling, Sampling::Nearest);
    assert_eq!(pattern.dither, Dither::None, "a drawing is not dithered");
    assert_eq!((pattern.grid.width(), pattern.grid.height()), (28, 28));

    for y in 0..28u32 {
        for x in 0..28u32 {
            let cell = Cell::new(x, y);
            let id = pattern.grid.get(cell).expect("in bounds").expect("a bead");
            assert_eq!(
                Rgba::opaque(palette.color(id).rgb),
                original.pixel(cell).expect("in bounds"),
                "cell {cell} changed colour on the way through"
            );
        }
    }
}

#[test]
fn the_bill_matches_the_grid_it_came_from() {
    let palette = Palette::generic_5mm();
    let (_, exported) = exported_sprite();
    let pattern = to_pattern(&exported, &palette, &one_board()).expect("a pattern");

    assert_eq!(pattern.bom.total_beads(), 784);
    assert_eq!(pattern.bom.distinct_colors(), 4);
    assert_eq!(pattern.bom.palette, "generic-5mm");
    assert_eq!(
        pattern
            .bom
            .lines
            .iter()
            .map(|l| (l.code.as_str(), l.count))
            .collect::<Vec<_>>(),
        vec![("G22", 658), ("G06", 108), ("G15", 14), ("G33", 4)]
    );

    for line in &pattern.bom.lines {
        let counted = pattern
            .grid
            .iter()
            .filter(|(_, slot)| slot.is_some_and(|id| palette.color(id).code == line.code))
            .count();
        assert_eq!(counted, line.count, "{} was miscounted", line.code);
    }
}

#[test]
fn a_photograph_gets_averaged_and_dithered_by_default() {
    let palette = Palette::generic_5mm();
    let mut pixels = Vec::with_capacity(200 * 200);
    for y in 0..200u32 {
        for x in 0..200u32 {
            pixels.push(Rgb::new((x + y) as u8, (y * 5 / 4) as u8, (255 - x) as u8));
        }
    }
    let photo = support::opaque(200, 200, &pixels);
    let pattern = to_pattern(&photo, &palette, &one_board()).expect("a pattern");

    assert_eq!(pattern.analysis.kind, ImageKind::Photo);
    assert_eq!(pattern.sampling, Sampling::BoxAverage);
    assert_eq!(pattern.dither, Dither::FloydSteinberg);
    assert_eq!(pattern.bom.total_beads(), 784);
    assert!(
        pattern.bom.distinct_colors() > 4,
        "a ramp should reach for several beads, used {}",
        pattern.bom.distinct_colors()
    );
}

#[test]
fn the_defaults_can_be_overridden() {
    let palette = Palette::generic_5mm();
    let (_, exported) = exported_sprite();
    let options = one_board()
        .with_dither(Dither::FloydSteinberg)
        .with_sampling(Sampling::BoxAverage);
    let pattern = to_pattern(&exported, &palette, &options).expect("a pattern");

    assert_eq!(pattern.analysis.kind, ImageKind::PixelArt { block_size: 4 });
    assert_eq!(pattern.sampling, Sampling::BoxAverage);
    assert_eq!(pattern.dither, Dither::FloydSteinberg);
}

#[test]
fn a_bad_framing_is_reported_rather_than_guessed() {
    let palette = Palette::generic_5mm();
    let image = Image::filled(10, 10, Rgba::new(0, 0, 0, 255)).expect("10x10");
    let options = PatternOptions::new(FitMode::ScaleCrop {
        board: BoardSpec::square_28(),
        cols: 1,
        rows: 1,
        pixels_per_cell: 0.0,
        crop_x: 0.0,
        crop_y: 0.0,
    });
    assert_eq!(
        to_pattern(&image, &palette, &options).unwrap_err(),
        FitError::BadScale
    );
}

/// A picture with a hole in it keeps the hole: the empty cells are not beads,
/// so they are neither on the bill nor in any of the four sets of instructions.
#[test]
fn transparency_survives_the_whole_pipeline() {
    let palette = Palette::generic_5mm();
    let image = support::ring(16, 16, Some((6, 6, 4, 4)), Rgba::new(0x0B, 0x61, 0xA4, 255));
    let options = PatternOptions::new(FitMode::FixedBoards {
        board: BoardSpec::new("16x16", 16, 16),
        cols: 1,
        rows: 1,
    })
    .with_sampling(Sampling::Nearest);
    let pattern = to_pattern(&image, &palette, &options).expect("a pattern");

    let filled = pattern
        .grid
        .iter()
        .filter(|(_, slot)| slot.is_some())
        .count();
    assert_eq!(filled, 16 * 16 - 16);
    assert_eq!(pattern.bom.total_beads(), filled);
    assert_eq!(pattern.grid.get(Cell::new(7, 7)), Some(&None));

    for mode in [
        StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
        StepMode::Tile {
            board: BoardSpec::new("8x8", 8, 8),
        },
        StepMode::OutlineInfill,
        StepMode::RowByRow,
    ] {
        let plan = plan_steps(&pattern.grid, &palette, &mode);
        assert_eq!(plan.total_cells(), filled, "{}", plan.mode);
        assert!(
            plan.placements()
                .all(|(_, cell)| matches!(pattern.grid.get(cell), Some(Some(_)))),
            "{} placed a bead in the hole",
            plan.mode
        );
    }
}

#[test]
fn the_conversion_is_reproducible() {
    let palette = Palette::generic_5mm();
    let (_, exported) = exported_sprite();
    let first = to_pattern(&exported, &palette, &one_board()).expect("a pattern");
    let second = to_pattern(&exported, &palette, &one_board()).expect("a pattern");
    assert_eq!(first, second);
}

/// Picture in, shopping list and four sets of instructions out. This is the v0
/// loop the product is built around, so it is worth one test that walks all of
/// it rather than only the pieces.
#[test]
fn the_whole_loop_runs_from_a_picture_to_instructions() {
    let palette = Palette::generic_5mm();
    let (_, exported) = exported_sprite();
    let pattern = to_pattern(&exported, &palette, &one_board()).expect("a pattern");

    let modes = [
        StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
        StepMode::Tile {
            board: BoardSpec::square_28(),
        },
        StepMode::OutlineInfill,
        StepMode::RowByRow,
    ];
    for mode in modes {
        let plan = plan_steps(&pattern.grid, &palette, &mode);
        assert_eq!(
            plan.total_cells(),
            784,
            "{} did not cover the pattern",
            plan.mode
        );
    }

    // A drawer with the body colour and nothing else.
    let inventory = Inventory::from_pairs([("G22", 1_000usize)]);
    let check = check_stock(&pattern.bom, &palette, &inventory);
    assert!(!check.is_satisfied());
    assert_eq!(check.shortages.len(), 3, "black, red and blue are missing");
    assert!(
        check.shortages.iter().all(|s| s.substitutes.is_empty()),
        "nothing in a yellow-only drawer is within ΔE00 3 of black, red or blue"
    );
}
