//! The whole conversion, end to end, and the defaults it picks.

use bead_core::bom::{check_stock, Inventory};
use bead_core::color::Rgb;
use bead_core::detect::ImageKind;
use bead_core::fit::{BoardSpec, FitMode, Sampling};
use bead_core::grid::Cell;
use bead_core::image::Image;
use bead_core::palette::Palette;
use bead_core::pipeline::{to_pattern, PatternOptions};
use bead_core::quantize::Dither;
use bead_core::steps::{plan_steps, ColorOrder, StepMode, StepOptions};

/// A 28×28 drawing in four palette-exact colours, exported at 4×.
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
    let original = Image::from_pixels(28, 28, pixels).expect("28x28");

    let factor = 4;
    let width = 28 * factor;
    let mut upscaled = Vec::with_capacity((width * width) as usize);
    for y in 0..width {
        for x in 0..width {
            upscaled.push(original.clamped(i64::from(x / factor), i64::from(y / factor)));
        }
    }
    let exported = Image::from_pixels(width, width, upscaled).expect("112x112");
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
            let id = *pattern.grid.get(cell).expect("in bounds");
            assert_eq!(
                palette.color(id).rgb,
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
            .filter(|(_, id)| palette.color(**id).code == line.code)
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
    let photo = Image::from_pixels(200, 200, pixels).expect("200x200");
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
        .with_dither(Dither::FloydSteinbergSerpentine)
        .with_sampling(Sampling::BoxAverage);
    let pattern = to_pattern(&exported, &palette, &options).expect("a pattern");

    assert_eq!(pattern.analysis.kind, ImageKind::PixelArt { block_size: 4 });
    assert_eq!(pattern.sampling, Sampling::BoxAverage);
    assert_eq!(pattern.dither, Dither::FloydSteinbergSerpentine);
}

#[test]
fn a_bad_framing_is_reported_rather_than_guessed() {
    let palette = Palette::generic_5mm();
    let image = Image::filled(10, 10, Rgb::new(0, 0, 0)).expect("10x10");
    let options = PatternOptions::new(FitMode::ScaleCrop {
        board: BoardSpec::square_28(),
        cols: 1,
        rows: 1,
        scale: 0.0,
        offset_x: 0.0,
        offset_y: 0.0,
    });
    assert!(to_pattern(&image, &palette, &options).is_err());
}

#[test]
fn the_conversion_is_reproducible() {
    let palette = Palette::generic_5mm();
    let (_, exported) = exported_sprite();
    let first = to_pattern(&exported, &palette, &one_board()).expect("a pattern");
    let second = to_pattern(&exported, &palette, &one_board()).expect("a pattern");
    assert_eq!(first.grid, second.grid);
    assert_eq!(first.bom, second.bom);
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
        let plan = plan_steps(&pattern.grid, &palette, &mode, &StepOptions::default());
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
