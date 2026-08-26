//! The three framing modes and the sampler that reads through them.
//!
//! Covers T-SCL-1 to T-SCL-5 from `docs/bead/reviews/round1-algorithms.md`.

mod support;

use bead_core::color::{Rgb, Rgba};
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
    support::opaque(width, height, &pixels)
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
    assert_eq!((plan.cells_wide, plan.cells_high), (28, 28));
    assert_eq!(plan.total_cells(), 784);
    assert_eq!(plan.board_count(), 1);
    assert!(plan.cropped_fraction.abs() < 1e-12);
    assert_eq!(plan.source.x, 0.0);
    assert_eq!(plan.source.width, 400.0);
}

#[test]
fn multiple_boards_multiply_the_cell_count() {
    let plan = fixed_boards(1000, 500, &BoardSpec::square_56(), 3, 2).expect("plan");
    assert_eq!((plan.cells_wide, plan.cells_high), (168, 112));
    assert_eq!(plan.board_count(), 6);
    assert_eq!(plan.total_cells(), 168 * 112);
}

/// A square picture onto a 2:1 board rectangle: half of it has to go, and the
/// plan says so rather than silently squashing.
#[test]
fn a_shape_mismatch_is_reported_as_a_centre_crop() {
    let plan = fixed_boards(400, 400, &BoardSpec::square_28(), 2, 1).expect("plan");
    assert_eq!((plan.cells_wide, plan.cells_high), (56, 28));
    assert!((plan.source.aspect() - 2.0).abs() < 1e-12);
    assert_eq!(plan.source.width, 400.0);
    assert_eq!(plan.source.height, 200.0);
    assert_eq!(plan.source.y, 100.0, "the crop is centred");
    assert!((plan.cropped_fraction - 0.5).abs() < 1e-12);
}

/// T-SCL-1: the box filter's identity case. Every 2×2 block of the source is
/// one colour, so reading 56×56 down to 28×28 must reproduce the blocks exactly
/// — an average of equal values is that value, and anything else in the sampler
/// (a wider kernel, a half-pixel offset) would show up here as a smear.
#[test]
fn a_block_uniform_source_survives_the_box_filter_intact() {
    for (source_size, board) in [
        (56u32, BoardSpec::square_28()),
        (112, BoardSpec::square_56()),
    ] {
        let cells = board.width;
        let block = source_size / cells;
        let colour = |bx: u32, by: u32| {
            Rgb::new(
                (bx * 9 % 256) as u8,
                (by * 5 % 256) as u8,
                ((bx * by) % 256) as u8,
            )
        };

        let mut pixels = Vec::with_capacity((source_size * source_size) as usize);
        for y in 0..source_size {
            for x in 0..source_size {
                pixels.push(colour(x / block, y / block));
            }
        }
        let image = support::opaque(source_size, source_size, &pixels);
        let plan = fixed_boards(source_size, source_size, &board, 1, 1).expect("plan");
        let reduced = render(&image, &plan, Sampling::BoxAverage);

        assert_eq!((reduced.width(), reduced.height()), (cells, cells));
        for y in 0..cells {
            for x in 0..cells {
                assert_eq!(
                    reduced.pixel(Cell::new(x, y)),
                    Some(Rgba::opaque(colour(x, y))),
                    "cell ({x}, {y}) of the {cells}-cell board changed"
                );
            }
        }
    }
}

/// T-SCL-2: the rounding rule, pinned by the four cases the review names. Half
/// rounds away from zero and no dimension ever reaches zero, so a 3000×1
/// panorama is a one-cell-tall strip rather than nothing at all.
#[test]
fn aspect_fit_rounds_the_way_the_contract_says() {
    let board = BoardSpec::square_28();
    for (w, h, cells_wide, cells_high) in [
        (100u32, 50u32, 28u32, 14u32),
        (50, 100, 14, 28),
        (29, 29, 28, 28),
        (3000, 1, 28, 1),
    ] {
        let plan = aspect_fit(w, h, &board, 1, 1).expect("plan");
        assert_eq!(
            (plan.cells_wide, plan.cells_high),
            (cells_wide, cells_high),
            "{w}x{h} should fit as {cells_wide}x{cells_high}"
        );
    }
}

#[test]
fn aspect_fit_keeps_the_whole_picture() {
    // 3:2 into a 4×4 budget of 28-boards: 112 wide would need 74.67 high, so
    // the height is the binding constraint and nothing is cropped either way.
    let plan = aspect_fit(600, 400, &BoardSpec::square_28(), 4, 4).expect("plan");
    assert_eq!((plan.cells_wide, plan.cells_high), (112, 75));
    assert!(plan.cropped_fraction.abs() < 1e-12);
    assert_eq!(plan.source.width, 600.0);
    assert_eq!(plan.source.height, 400.0);

    let tall = aspect_fit(400, 600, &BoardSpec::square_28(), 4, 4).expect("plan");
    assert_eq!((tall.cells_wide, tall.cells_high), (75, 112));
}

/// The board count is what the user has to buy, so a pattern that only part
/// fills its last board still needs that board.
#[test]
fn a_part_filled_board_still_counts_as_a_board() {
    let plan = aspect_fit(100, 50, &BoardSpec::square_28(), 1, 1).expect("plan");
    assert_eq!((plan.cells_wide, plan.cells_high), (28, 14));
    assert_eq!((plan.boards_across, plan.boards_down), (1, 1));
    assert_eq!(plan.board_count(), 1);

    let wide = aspect_fit(300, 100, &BoardSpec::square_28(), 4, 4).expect("plan");
    assert_eq!((wide.cells_wide, wide.cells_high), (112, 37));
    assert_eq!((wide.boards_across, wide.boards_down), (4, 2));
}

#[test]
fn scale_one_reproduces_the_fixed_board_framing_on_a_matching_shape() {
    let board = BoardSpec::square_28();
    let fixed = fixed_boards(280, 280, &board, 1, 1).expect("plan");
    let scaled = scale_crop(280, 280, &board, 1, 1, 10.0, 0.0, 0.0).expect("plan");
    assert_eq!(fixed.source, scaled.source);
    assert_eq!(
        (fixed.cells_wide, fixed.cells_high),
        (scaled.cells_wide, scaled.cells_high)
    );
}

/// T-SCL-3, first case: a crop wholly inside the image.
#[test]
fn a_crop_inside_the_image_samples_only_the_crop() {
    let plan =
        scale_crop(400, 400, &BoardSpec::square_28(), 1, 1, 2.0, 100.0, 100.0).expect("plan");
    assert_eq!(plan.source.width, 56.0);
    assert_eq!(plan.source.height, 56.0);
    assert_eq!((plan.source.x, plan.source.y), (100.0, 100.0));
    // 56×56 of 400×400 kept.
    let kept = (56.0 * 56.0) / (400.0 * 400.0);
    assert!((plan.cropped_fraction - (1.0 - kept)).abs() < 1e-12);
}

/// T-SCL-3, second case: a crop that hangs over the edge is legal, and the part
/// that is off the image comes back as empty cells rather than a smeared edge
/// colour. Clamping the crop instead would move the picture under the user's
/// hands, which is worse than showing them the hole they dragged.
#[test]
fn a_crop_over_the_edge_is_transparent_outside_the_image() {
    let image = Image::filled(8, 8, Rgba::new(200, 60, 40, 255)).expect("8x8");
    let board = BoardSpec::new("4x4", 4, 4);
    let plan = scale_crop(8, 8, &board, 1, 1, 2.0, 4.0, 4.0).expect("plan");
    assert_eq!((plan.source.x, plan.source.y), (4.0, 4.0));
    assert_eq!((plan.source.width, plan.source.height), (8.0, 8.0));

    for sampling in [Sampling::Nearest, Sampling::BoxAverage] {
        let rendered = render(&image, &plan, sampling);
        for y in 0..4u32 {
            for x in 0..4u32 {
                let pixel = rendered.pixel(Cell::new(x, y)).expect("in bounds");
                // The crop starts halfway across, so only the top-left quadrant
                // of the viewport still lands on the picture.
                let over_the_image = x < 2 && y < 2;
                assert_eq!(
                    pixel.is_opaque(),
                    over_the_image,
                    "cell ({x}, {y}) under {sampling:?}"
                );
            }
        }
    }
}

/// T-SCL-3, third case.
#[test]
fn a_crop_entirely_off_the_image_is_a_typed_error() {
    let board = BoardSpec::square_28();
    assert_eq!(
        scale_crop(400, 400, &board, 1, 1, 1.0, 400.0, 0.0).unwrap_err(),
        FitError::CropOutsideImage
    );
    assert_eq!(
        scale_crop(400, 400, &board, 1, 1, 1.0, 0.0, -28.0).unwrap_err(),
        FitError::CropOutsideImage
    );
}

/// T-SCL-4: every refusal is a value, never a panic.
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
        FitError::InvalidDimensions {
            width: 0,
            height: 10
        }
    );
    assert_eq!(
        aspect_fit(10, 0, &board, 1, 1).unwrap_err(),
        FitError::InvalidDimensions {
            width: 10,
            height: 0
        }
    );
    assert_eq!(
        fixed_boards(10, 10, &board, 1, 0).unwrap_err(),
        FitError::EmptyBoard
    );
    assert_eq!(
        aspect_fit(10, 10, &BoardSpec::new("zero", 0, 28), 2, 2).unwrap_err(),
        FitError::EmptyBoard
    );
    for bad_scale in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert_eq!(
            scale_crop(10, 10, &board, 1, 1, bad_scale, 0.0, 0.0).unwrap_err(),
            FitError::BadScale,
            "pixels_per_cell {bad_scale} should be refused"
        );
    }
    assert_eq!(
        scale_crop(10, 10, &board, 1, 1, 1.0, f64::NAN, 0.0).unwrap_err(),
        FitError::BadScale
    );
}

/// T-SCL-5: the one fixture that tells the two candidate resampling spaces
/// apart. Black and white averaged in linear light encode back to 188; averaged
/// as sRGB code values they would give 128. This crate does the former, and a
/// port that gets 128 here has picked the other one.
#[test]
fn the_box_filter_averages_in_linear_light() {
    let image = support::opaque(2, 1, &[Rgb::new(0, 0, 0), Rgb::new(255, 255, 255)]);
    let plan = fixed_boards(2, 1, &BoardSpec::new("1x1", 1, 1), 1, 1).expect("plan");
    let reduced = render(&image, &plan, Sampling::BoxAverage);
    assert_eq!(
        reduced.pixel(Cell::new(0, 0)),
        Some(Rgba::new(188, 188, 188, 255))
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
            pixels_per_cell: 1.5,
            crop_x: 4.0,
            crop_y: -2.0,
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
    let exported = support::upscale(&original, 4);
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

/// Averaging a black-and-white checkerboard down to one cell gives the linear
/// mid grey. Nearest sampling on the same input gives one of the two extremes:
/// this is the whole reason the sampler is a choice and not a constant.
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
    let image = support::opaque(8, 8, &pixels);
    let plan = fixed_boards(8, 8, &BoardSpec::new("1x1", 1, 1), 1, 1).expect("plan");

    let averaged = render(&image, &plan, Sampling::BoxAverage)
        .pixel(Cell::new(0, 0))
        .expect("one cell");
    assert_eq!(averaged, Rgba::new(188, 188, 188, 255));

    let nearest = render(&image, &plan, Sampling::Nearest)
        .pixel(Cell::new(0, 0))
        .expect("one cell");
    assert!(nearest == Rgba::new(0, 0, 0, 255) || nearest == Rgba::new(255, 255, 255, 255));
}

/// A cell whose covered pixels are mostly transparent comes back empty, and one
/// with a solid majority takes the colour of the solid part only. Averaging the
/// invisible pixels' colours in would drag every edge cell towards whatever
/// happens to sit behind the alpha.
#[test]
fn averaging_ignores_the_colour_behind_transparent_pixels() {
    let solid = Rgba::new(220, 30, 30, 255);
    let ghost = Rgba::new(0, 0, 0, 0);
    let image = Image::from_pixels(2, 2, vec![solid, solid, solid, ghost]).expect("2x2");
    let plan = fixed_boards(2, 2, &BoardSpec::new("1x1", 1, 1), 1, 1).expect("plan");
    let reduced = render(&image, &plan, Sampling::BoxAverage);
    assert_eq!(reduced.pixel(Cell::new(0, 0)), Some(solid));

    let mostly_gone = Image::from_pixels(2, 2, vec![solid, ghost, ghost, ghost]).expect("2x2");
    let reduced = render(&mostly_gone, &plan, Sampling::BoxAverage);
    assert_eq!(
        reduced.pixel(Cell::new(0, 0)).map(|p| p.is_opaque()),
        Some(false)
    );
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
