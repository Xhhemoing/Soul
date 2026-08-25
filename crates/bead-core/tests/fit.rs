//! The three framing modes and the sampler that reads through them.

use bead_core::color::Rgb;
use bead_core::fit::{
    aspect_fit, fixed_boards, plan, render, scale_crop, BoardSpec, FitError, FitMode, Sampling,
};
use bead_core::grid::Cell;
use bead_core::image::Image;

fn sprite(width: u32, height: u32) -> Image {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(Rgb::new(
                (x * 7 % 256) as u8,
                (y * 11 % 256) as u8,
                ((x + y) * 3 % 256) as u8,
            ));
        }
    }
    Image::from_pixels(width, height, pixels).expect("sprite")
}

fn upscale(image: &Image, factor: u32) -> Image {
    let width = image.width() * factor;
    let height = image.height() * factor;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(image.clamped(i64::from(x / factor), i64::from(y / factor)));
        }
    }
    Image::from_pixels(width, height, pixels).expect("upscaled")
}

#[test]
fn the_standard_boards_are_the_sizes_the_plan_names() {
    assert_eq!(BoardSpec::square_28().width, 28);
    assert_eq!(BoardSpec::square_28().height, 28);
    assert_eq!(BoardSpec::square_28().cells(), 784);
    assert_eq!(BoardSpec::square_56().width, 56);
    assert_eq!(BoardSpec::square_56().height, 56);
    assert_eq!(BoardSpec::square_56().cells(), 3_136);
}

#[test]
fn one_28_board_from_a_square_photo_keeps_all_of_it() {
    let plan = fixed_boards(400, 400, &BoardSpec::square_28(), 1, 1).expect("plan");
    assert_eq!((plan.cells_wide(), plan.cells_high()), (28, 28));
    assert_eq!(plan.total_cells(), 784);
    assert_eq!(plan.tiling.board_count(), 1);
    assert!(plan.cropped_fraction.abs() < 1e-12);
    assert_eq!(plan.source.x, 0.0);
    assert_eq!(plan.source.width, 400.0);
}

#[test]
fn multiple_boards_multiply_the_cell_count() {
    let plan = fixed_boards(1000, 500, &BoardSpec::square_56(), 3, 2).expect("plan");
    assert_eq!((plan.cells_wide(), plan.cells_high()), (168, 112));
    assert_eq!(plan.tiling.board_count(), 6);
    assert_eq!(plan.total_cells(), 168 * 112);
}

/// A square picture onto a 2:1 board rectangle: half of it has to go, and the
/// plan says so rather than silently squashing.
#[test]
fn a_shape_mismatch_is_reported_as_a_centre_crop() {
    let plan = fixed_boards(400, 400, &BoardSpec::square_28(), 2, 1).expect("plan");
    assert_eq!((plan.cells_wide(), plan.cells_high()), (56, 28));
    assert!((plan.source.aspect() - 2.0).abs() < 1e-12);
    assert_eq!(plan.source.width, 400.0);
    assert_eq!(plan.source.height, 200.0);
    assert_eq!(plan.source.y, 100.0, "the crop is centred");
    assert!((plan.cropped_fraction - 0.5).abs() < 1e-12);
}

#[test]
fn aspect_fit_picks_the_board_rectangle_that_matches_the_picture() {
    // 3:2, so three 28-boards across and two down is exact.
    let plan = aspect_fit(600, 400, &BoardSpec::square_28(), 4, 4).expect("plan");
    assert_eq!((plan.tiling.cols, plan.tiling.rows), (3, 2));
    assert!(plan.cropped_fraction.abs() < 1e-9);

    // A portrait picture gets a portrait layout.
    let tall = aspect_fit(400, 600, &BoardSpec::square_28(), 4, 4).expect("plan");
    assert_eq!((tall.tiling.cols, tall.tiling.rows), (2, 3));
}

/// Two layouts that frame the picture equally well are not equally detailed, so
/// the tie goes to the larger bead count.
#[test]
fn aspect_fit_breaks_ties_towards_more_beads() {
    let plan = aspect_fit(500, 500, &BoardSpec::square_28(), 4, 4).expect("plan");
    assert_eq!((plan.tiling.cols, plan.tiling.rows), (4, 4));
    assert_eq!(plan.total_cells(), 112 * 112);
}

#[test]
fn aspect_fit_within_a_single_board_budget_is_just_that_board() {
    let plan = aspect_fit(1920, 1080, &BoardSpec::square_56(), 1, 1).expect("plan");
    assert_eq!((plan.cells_wide(), plan.cells_high()), (56, 56));
    // 16:9 onto a square: the sides go.
    assert!(plan.cropped_fraction > 0.4);
}

#[test]
fn scale_one_reproduces_the_fixed_board_framing() {
    let board = BoardSpec::square_28();
    let fixed = fixed_boards(400, 300, &board, 1, 1).expect("plan");
    let scaled = scale_crop(400, 300, &board, 1, 1, 1.0, 0.0, 0.0).expect("plan");
    assert_eq!(fixed.source, scaled.source);
    assert_eq!(fixed.tiling, scaled.tiling);
}

#[test]
fn zooming_in_halves_the_source_rectangle() {
    let plan = scale_crop(400, 400, &BoardSpec::square_28(), 1, 1, 2.0, 0.0, 0.0).expect("plan");
    assert_eq!(plan.source.width, 200.0);
    assert_eq!(plan.source.height, 200.0);
    assert_eq!(plan.source.x, 100.0);
    assert_eq!(plan.source.y, 100.0);
    assert!((plan.cropped_fraction - 0.75).abs() < 1e-12);
}

/// Dragging a picture past its own edge should stop at the edge, not fail and
/// not sample outside the image.
#[test]
fn panning_is_clamped_to_the_image() {
    let plan = scale_crop(
        400,
        400,
        &BoardSpec::square_28(),
        1,
        1,
        2.0,
        10_000.0,
        -10_000.0,
    )
    .expect("plan");
    assert_eq!(plan.source.x, 200.0);
    assert_eq!(plan.source.y, 0.0);
    assert!(plan.source.x + plan.source.width <= 400.0);
    assert!(plan.source.y + plan.source.height <= 400.0);
}

#[test]
fn zooming_out_below_one_never_leaves_the_image() {
    let plan = scale_crop(400, 400, &BoardSpec::square_28(), 1, 1, 0.25, 0.0, 0.0).expect("plan");
    assert!(plan.source.width <= 400.0 && plan.source.height <= 400.0);
    assert!(plan.source.x >= 0.0 && plan.source.y >= 0.0);
}

#[test]
fn the_refusals_are_specific() {
    let board = BoardSpec::square_28();
    assert_eq!(
        plan(
            0,
            10,
            &FitMode::FixedBoards {
                board: board.clone(),
                cols: 1,
                rows: 1
            }
        )
        .unwrap_err(),
        FitError::EmptySource
    );
    assert_eq!(
        fixed_boards(10, 10, &board, 1, 0).unwrap_err(),
        FitError::EmptyBoard
    );
    assert_eq!(
        aspect_fit(10, 10, &BoardSpec::new("zero", 0, 28), 2, 2).unwrap_err(),
        FitError::EmptyBoard
    );
    assert_eq!(
        scale_crop(10, 10, &board, 1, 1, f64::NAN, 0.0, 0.0).unwrap_err(),
        FitError::BadScale
    );
    assert_eq!(
        scale_crop(10, 10, &board, 1, 1, -1.0, 0.0, 0.0).unwrap_err(),
        FitError::BadScale
    );
}

#[test]
fn the_mode_enum_routes_to_the_same_answers() {
    let board = BoardSpec::square_28();
    let direct = fixed_boards(300, 200, &board, 2, 1).expect("plan");
    let routed = plan(
        300,
        200,
        &FitMode::FixedBoards {
            board: board.clone(),
            cols: 2,
            rows: 1,
        },
    )
    .expect("plan");
    assert_eq!(direct, routed);

    let direct = aspect_fit(300, 200, &board, 3, 3).expect("plan");
    let routed = plan(
        300,
        200,
        &FitMode::AspectFit {
            board: board.clone(),
            max_cols: 3,
            max_rows: 3,
        },
    )
    .expect("plan");
    assert_eq!(direct, routed);

    let direct = scale_crop(300, 200, &board, 1, 1, 1.5, 4.0, -2.0).expect("plan");
    let routed = plan(
        300,
        200,
        &FitMode::ScaleCrop {
            board,
            cols: 1,
            rows: 1,
            scale: 1.5,
            offset_x: 4.0,
            offset_y: -2.0,
        },
    )
    .expect("plan");
    assert_eq!(direct, routed);
}

/// The property that makes pixel art worth detecting: read a 4× export back at
/// its own block size with nearest sampling and the original comes out intact.
#[test]
fn nearest_sampling_recovers_an_upscaled_sprite_exactly() {
    let original = sprite(28, 28);
    let exported = upscale(&original, 4);
    let plan = fixed_boards(
        exported.width(),
        exported.height(),
        &BoardSpec::square_28(),
        1,
        1,
    )
    .expect("plan");
    let recovered = render(&exported, &plan, Sampling::Nearest);

    assert_eq!(recovered.width(), 28);
    assert_eq!(recovered.height(), 28);
    for y in 0..28u32 {
        for x in 0..28u32 {
            let cell = Cell::new(x, y);
            assert_eq!(
                recovered.pixel(cell),
                original.pixel(cell),
                "cell {cell} came back changed"
            );
        }
    }
}

#[test]
fn box_averaging_is_the_mean_of_the_cell() {
    // Four quadrants of one colour each, read down to 2×2: every cell is one
    // quadrant's colour, and averaging cannot invent anything.
    let mut pixels = Vec::new();
    for y in 0..4u32 {
        for x in 0..4u32 {
            pixels.push(match (x < 2, y < 2) {
                (true, true) => Rgb::new(0, 0, 0),
                (false, true) => Rgb::new(255, 0, 0),
                (true, false) => Rgb::new(0, 255, 0),
                (false, false) => Rgb::new(0, 0, 255),
            });
        }
    }
    let image = Image::from_pixels(4, 4, pixels).expect("4x4");
    let plan = fixed_boards(4, 4, &BoardSpec::new("2x2", 2, 2), 1, 1).expect("plan");
    let reduced = render(&image, &plan, Sampling::BoxAverage);

    assert_eq!(reduced.pixel(Cell::new(0, 0)), Some(Rgb::new(0, 0, 0)));
    assert_eq!(reduced.pixel(Cell::new(1, 0)), Some(Rgb::new(255, 0, 0)));
    assert_eq!(reduced.pixel(Cell::new(0, 1)), Some(Rgb::new(0, 255, 0)));
    assert_eq!(reduced.pixel(Cell::new(1, 1)), Some(Rgb::new(0, 0, 255)));
}

/// Averaging a black-and-white checkerboard down to one cell gives mid grey.
/// Nearest sampling on the same input gives one of the two extremes: this is
/// the whole reason the sampler is a choice and not a constant.
#[test]
fn the_two_samplers_answer_differently_on_fine_detail() {
    let mut pixels = Vec::new();
    for y in 0..8u32 {
        for x in 0..8u32 {
            pixels.push(if (x + y) % 2 == 0 {
                Rgb::new(0, 0, 0)
            } else {
                Rgb::new(255, 255, 255)
            });
        }
    }
    let image = Image::from_pixels(8, 8, pixels).expect("8x8");
    let plan = fixed_boards(8, 8, &BoardSpec::new("1x1", 1, 1), 1, 1).expect("plan");

    let averaged = render(&image, &plan, Sampling::BoxAverage)
        .pixel(Cell::new(0, 0))
        .expect("one cell");
    assert_eq!(averaged, Rgb::new(128, 128, 128));

    let nearest = render(&image, &plan, Sampling::Nearest)
        .pixel(Cell::new(0, 0))
        .expect("one cell");
    assert!(nearest == Rgb::new(0, 0, 0) || nearest == Rgb::new(255, 255, 255));
}

/// Upsampling is legal — a 16×16 sprite onto a 28×28 board — and must not read
/// outside the image.
#[test]
fn rendering_more_cells_than_pixels_stays_in_bounds() {
    let image = sprite(16, 16);
    let plan = fixed_boards(16, 16, &BoardSpec::square_28(), 1, 1).expect("plan");
    for sampling in [Sampling::Nearest, Sampling::BoxAverage] {
        let rendered = render(&image, &plan, sampling);
        assert_eq!((rendered.width(), rendered.height()), (28, 28));
        assert_eq!(rendered.as_slice().len(), 784);
    }
}
