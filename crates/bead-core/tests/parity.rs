//! The shared fixtures WP-B03 is held to (T-PAR-1 to T-PAR-3).
//!
//! `crates/bead-core/fixtures/parity/*.json` is the contract between this crate
//! and its TypeScript reimplementation: same bytes in, same colour codes, same
//! bill, same four step sequences out. These tests say that the oracle still
//! produces exactly the committed files, so a change to the algorithms shows up
//! as a diff someone has to justify rather than as a browser that quietly
//! disagrees.
//!
//! Regenerate with:
//!
//! ```text
//! cargo run --example emit-parity-fixtures --manifest-path crates/bead-core/Cargo.toml
//! ```

use bead_core::color::{to_channel, Rgba};
use bead_core::detect::ImageKind;
use bead_core::fit::Sampling;
use bead_core::image::Image;
use bead_core::palette::{BeadColor, Palette};
use bead_core::parity::{
    build, cases, detected_kind, ParityCase, NEAR_TIE_MARGIN, ROUNDING_RAMP_GREY,
    ROUNDING_SEED_GREY,
};
use bead_core::quantize::{map_image_traced, Dither, MapOptions, FLOYD_STEINBERG_KERNEL};
use bead_core::Rgb;

/// The committed bytes, pulled in at compile time so the test needs no
/// filesystem of its own.
fn committed(name: &str) -> &'static str {
    match name {
        "pixel-art" => include_str!("../fixtures/parity/pixel-art.json"),
        "photo-flat" => include_str!("../fixtures/parity/photo-flat.json"),
        "photo-dithered" => include_str!("../fixtures/parity/photo-dithered.json"),
        "with-transparency" => include_str!("../fixtures/parity/with-transparency.json"),
        "dither-rounding" => include_str!("../fixtures/parity/dither-rounding.json"),
        other => panic!("`{other}` has no committed fixture; run the emit example"),
    }
}

/// Every committed case, in the order `cases()` names them.
const CASE_NAMES: [&str; 5] = [
    "pixel-art",
    "photo-flat",
    "photo-dithered",
    "with-transparency",
    "dither-rounding",
];

/// T-PAR-1: every case reproduces its committed file byte for byte.
#[test]
fn every_case_matches_its_committed_fixture() {
    let palette = Palette::generic_5mm();
    for case in cases() {
        let fixture = build(&case, &palette).expect("a valid framing");
        let produced = fixture.to_json(&palette);
        let expected = committed(&fixture.name);
        assert_eq!(
            produced, expected,
            "`{}` no longer matches its committed fixture — if that is intended, \
             regenerate with the emit-parity-fixtures example and explain the diff",
            fixture.name
        );
    }
}

/// T-PAR-1, coverage: the review names the paths the fixture set has to reach.
#[test]
fn the_fixture_set_covers_the_paths_the_review_names() {
    let names: Vec<String> = cases().into_iter().map(|c| c.name).collect();
    assert_eq!(names, CASE_NAMES);

    let by_name = |name: &str| cases().into_iter().find(|c| c.name == name).expect(name);
    assert_eq!(
        detected_kind(&by_name("pixel-art")),
        ImageKind::PixelArt { block_size: 4 },
        "the pixel-art case has to actually take the pixel-art path"
    );
    for photo in ["photo-flat", "photo-dithered"] {
        assert_eq!(detected_kind(&by_name(photo)), ImageKind::Photo, "{photo}");
    }
    assert_eq!(by_name("photo-flat").dither, Dither::None);
    assert_eq!(by_name("photo-dithered").dither, Dither::FloydSteinberg);
    assert!(
        by_name("with-transparency")
            .image
            .as_slice()
            .iter()
            .any(|p| !p.is_opaque()),
        "the transparency case has to contain a hole"
    );

    let rounding = by_name("dither-rounding");
    assert_eq!(rounding.dither, Dither::FloydSteinberg);
    assert_eq!(
        rounding.sampling,
        Sampling::Nearest,
        "the ramp has to reach the dither unresampled, or the rounding it pins \
         would be the box filter's rather than the lookup's"
    );
}

/// The two photo cases differ only in the dither switch, so the fixture pair
/// isolates what dithering does rather than confounding it with the input.
#[test]
fn the_two_photo_cases_share_an_image() {
    let flat = cases()
        .into_iter()
        .find(|c| c.name == "photo-flat")
        .unwrap();
    let dithered = cases()
        .into_iter()
        .find(|c| c.name == "photo-dithered")
        .unwrap();
    assert_eq!(flat.image, dithered.image);
    assert_eq!(flat.fit, dithered.fit);
    assert_eq!(flat.sampling, dithered.sampling);
    assert_ne!(flat.dither, dithered.dither);

    let palette = Palette::generic_5mm();
    let flat = build(&flat, &palette).expect("a valid framing");
    let dithered = build(&dithered, &palette).expect("a valid framing");
    assert_ne!(
        flat.grid, dithered.grid,
        "if dithering changed nothing here the pair would pin nothing"
    );
}

/// AL-4: the `dither-rounding` fixture is only worth committing if it would say
/// something different when the lookup stops rounding.
///
/// The lookup clamps *and* rounds (`color::to_channel`). The original four
/// fixtures agree with a lookup that only clamped, so on their own they let a
/// port drop the rounding and still reproduce every committed byte. This walks
/// the fork by hand at the cell where the ramp straddles it, and then checks
/// that the committed grid took the rounded branch.
#[test]
fn the_dither_ramp_turns_on_how_the_lookup_rounds() {
    let palette = Palette::generic_5mm();

    let seed = Rgb::new(ROUNDING_SEED_GREY, ROUNDING_SEED_GREY, ROUNDING_SEED_GREY);
    let slate = palette.color(palette.nearest(seed).id);
    assert_eq!(slate.code, "G08", "the ramp's seed grey quantises to Slate");

    // The only kernel weight that reaches the cell to the right.
    let (dx, dy, weight) = FLOYD_STEINBERG_KERNEL[0];
    assert_eq!((dx, dy), (1, 0));
    let carried = |placed: u8| {
        f64::from(ROUNDING_RAMP_GREY) + weight * (f64::from(ROUNDING_SEED_GREY) - f64::from(placed))
    };
    let carried = [
        carried(slate.rgb.r),
        carried(slate.rgb.g),
        carried(slate.rgb.b),
    ];
    assert_eq!(carried, [241.875, 235.3125, 230.5]);

    let rounded = Rgb::new(
        to_channel(carried[0]),
        to_channel(carried[1]),
        to_channel(carried[2]),
    );
    assert_eq!(rounded, Rgb::new(242, 235, 231));
    assert_eq!(palette.color(palette.nearest(rounded).id).code, "G01");

    // What a lookup that only clamped would have asked for instead. Every
    // channel here is already inside `0..=255`, so clamping is the identity and
    // the cast to `u8` is all that is left of it.
    let clamped_only = Rgb::new(carried[0] as u8, carried[1] as u8, carried[2] as u8);
    assert_eq!(clamped_only, Rgb::new(241, 235, 230));
    assert_eq!(palette.color(palette.nearest(clamped_only).id).code, "G02");

    let case = cases()
        .into_iter()
        .find(|c| c.name == "dither-rounding")
        .expect("dither-rounding");
    let fixture = build(&case, &palette).expect("a valid framing");
    let second = fixture.grid.as_slice()[1].expect("the second cell is a bead");
    assert_eq!(
        palette.color(second).code,
        "G01",
        "the committed grid has to be on the rounded side of the fork, or the \
         fixture pins nothing about rounding"
    );
}

/// T-PAR-2: the input is raw RGBA and nothing else. No PNG, no JPEG, no ICC
/// profile — a browser applies one on decode and Rust's decoders do not, which
/// would break the comparison before either implementation had run.
///
/// Asserted by round-tripping each case's image through the flat byte form the
/// fixture stores, which is also the form a browser's `ImageData` arrives in.
#[test]
fn the_source_is_plain_rgba_bytes() {
    for case in cases() {
        let bytes: Vec<u8> = case
            .image
            .as_slice()
            .iter()
            .flat_map(|p| [p.r, p.g, p.b, p.a])
            .collect();
        assert_eq!(
            bytes.len(),
            (case.image.width() * case.image.height() * 4) as usize
        );
        let rebuilt = Image::from_rgba8(case.image.width(), case.image.height(), &bytes)
            .expect("four bytes a pixel");
        assert_eq!(rebuilt, case.image, "`{}` did not round-trip", case.name);
    }
}

/// T-PAR-3: the near-tie sentinel. `powf`, `cbrt` and `atan2` can differ in the
/// last place between Rust's libm and JavaScript's `Math`; a pixel whose best
/// and second-best beads are closer together than that gap can flip. No
/// committed fixture may contain one.
#[test]
fn no_committed_fixture_rests_on_a_near_tie() {
    let palette = Palette::generic_5mm();
    for case in cases() {
        let fixture = build(&case, &palette).expect("a valid framing");
        assert!(
            fixture.near_ties.is_empty(),
            "`{}` has pixels too close to call: {:?}",
            fixture.name,
            fixture.near_ties
        );
    }
}

/// The margins the sentinel measures are comfortably clear of it, so the
/// fixtures are not one rounding away from being coin tosses.
#[test]
fn the_margins_are_not_marginal() {
    let palette = Palette::generic_5mm();
    for case in cases() {
        let fixture = build(&case, &palette).expect("a valid framing");
        let sampled = bead_core::fit::render(
            &case.image,
            &bead_core::fit::plan(case.image.width(), case.image.height(), &case.fit)
                .expect("a valid framing"),
            case.sampling,
        );
        let traced = map_image_traced(&sampled, &palette, MapOptions::with_dither(case.dither));
        let smallest = traced
            .margins
            .iter()
            .filter_map(|m| *m)
            .fold(f64::INFINITY, f64::min);
        assert!(
            smallest > 1e-3,
            "`{}` decides one of its cells by only {smallest}",
            fixture.name
        );
    }
}

/// A sentinel that never fires is not a sentinel. A palette holding the same
/// colour twice makes every decision an exact tie, and the generator has to say
/// so rather than pick the lower index and call it settled.
#[test]
fn the_sentinel_catches_a_tie_that_is_really_there() {
    let doubled = Palette::new(
        "test-doubled",
        vec![
            BeadColor::new("D01", "Stone", Rgb::new(0x80, 0x80, 0x80)),
            BeadColor::new("D02", "Stone again", Rgb::new(0x80, 0x80, 0x80)),
        ],
    )
    .expect("valid palette");

    let case: ParityCase = cases()
        .into_iter()
        .find(|c| c.name == "photo-flat")
        .expect("photo-flat");
    let fixture = build(&case, &doubled).expect("a valid framing");
    assert_eq!(
        fixture.near_ties.len(),
        fixture.grid.len(),
        "every cell of a doubled palette is a tie"
    );
    assert!(fixture
        .near_ties
        .iter()
        .all(|t| t.margin >= 0.0 && t.margin <= NEAR_TIE_MARGIN));
}

/// An empty cell has no decision to record, so the sentinel has nothing to say
/// about it — and must not report a margin of zero as if it were a tie.
#[test]
fn an_empty_cell_is_not_a_near_tie() {
    let palette = Palette::generic_5mm();
    let case = cases()
        .into_iter()
        .find(|c| c.name == "with-transparency")
        .expect("with-transparency");
    let fixture = build(&case, &palette).expect("a valid framing");
    let empty = fixture
        .grid
        .as_slice()
        .iter()
        .filter(|c| c.is_none())
        .count();
    assert_eq!(empty, 4, "the hole is four cells wide at this framing");
    assert!(fixture.near_ties.is_empty());
}

/// The emitted JSON is the plain, float-free form the review asks for: parsing
/// it must not require anyone to agree about rounding.
#[test]
fn the_fixture_format_has_no_floating_point_in_it() {
    for name in CASE_NAMES {
        let text = committed(name);
        assert!(text.ends_with("}\n"), "{name} should end with a newline");
        for (line_number, line) in text.lines().enumerate() {
            let outside_strings: String = strip_strings(line);
            assert!(
                !outside_strings.contains('.') && !outside_strings.contains('e'),
                "{name} line {} has a number that is not an integer: {line}",
                line_number + 1
            );
        }
    }
}

/// Everything outside double quotes, for a document with no escaped quotes in
/// it — which the fixture's own contents guarantee.
fn strip_strings(line: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in line.chars() {
        if c == '"' {
            inside = !inside;
            continue;
        }
        if !inside {
            out.push(c);
        }
    }
    out
}

/// The transparent pixels in the fixtures are fully transparent rather than
/// merely below the threshold, so a port that reads alpha with a different
/// comparison still gets the same answer on these files. The threshold itself
/// is pinned in `color.rs`, not here.
#[test]
fn the_fixture_alpha_values_are_unambiguous() {
    for case in cases() {
        for pixel in case.image.as_slice() {
            assert!(
                *pixel == Rgba::TRANSPARENT || pixel.a == 255,
                "`{}` has a pixel at alpha {}",
                case.name,
                pixel.a
            );
        }
    }
}
