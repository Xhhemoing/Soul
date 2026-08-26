//! T-SRGB-1: the sRGB → Lab (D65, 2°) half of the colour pipeline, kept apart
//! from the CIEDE2000 table in `ciede2000.rs`.
//!
//! Reference values are the standard sRGB primaries and neutrals under D65 with
//! the IEC 61966-2-1 transfer function. The contract allows ±0.01; this pins
//! ±0.001, because the matrix coefficients are fixed to seven places on both
//! sides and there is no reason to leave the extra room unclaimed.
//!
//! White and mid grey are not exactly neutral. Rounding the matrix is what
//! leaves them a hundredth of a unit of chroma, and the contract offers a
//! choice: derive the white point from the matrix row sums and assert exactly
//! zero, or keep the published D65 white point and allow the residue. This
//! crate keeps the published white point — it is the one both implementations
//! can quote from the same standard — so the residue is asserted as a bound
//! rather than wished away.

use bead_core::color::{srgb_to_lab, Lab, Rgb};
use bead_core::palette::Palette;

const TOLERANCE: f64 = 1e-3;

/// The chroma the rounded sRGB matrix leaves on a neutral colour.
const NEUTRAL_RESIDUE: f64 = 0.01;

fn lab_of(hex: &str) -> Lab {
    srgb_to_lab(Rgb::from_hex(hex).expect("valid hex"))
}

fn assert_lab_close(got: Lab, want: Lab, what: &str) {
    assert!(
        (got.l - want.l).abs() <= TOLERANCE
            && (got.a - want.a).abs() <= TOLERANCE
            && (got.b - want.b).abs() <= TOLERANCE,
        "{what}: expected L*a*b* ({:.4}, {:.4}, {:.4}), got ({:.4}, {:.4}, {:.4})",
        want.l,
        want.a,
        want.b,
        got.l,
        got.a,
        got.b
    );
}

#[test]
fn white_is_the_d65_white_point() {
    let got = lab_of("#FFFFFF");
    assert!((got.l - 100.0).abs() <= TOLERANCE, "L* was {}", got.l);
    assert!(got.a.abs() < NEUTRAL_RESIDUE, "a* was {}", got.a);
    assert!(got.b.abs() < NEUTRAL_RESIDUE, "b* was {}", got.b);
}

#[test]
fn black_is_the_origin() {
    let got = lab_of("#000000");
    assert_lab_close(got, Lab::new(0.0, 0.0, 0.0), "#000000");
}

#[test]
fn the_primaries_land_where_srgb_says() {
    assert_lab_close(
        lab_of("#FF0000"),
        Lab::new(53.2408, 80.0925, 67.2032),
        "#FF0000",
    );
    assert_lab_close(
        lab_of("#00FF00"),
        Lab::new(87.7347, -86.1827, 83.1793),
        "#00FF00",
    );
    assert_lab_close(
        lab_of("#0000FF"),
        Lab::new(32.2970, 79.1875, -107.8602),
        "#0000FF",
    );
}

#[test]
fn mid_grey_is_neutral_and_darker_than_half() {
    // 50% in sRGB code value is not 50% lightness: the transfer function is why
    // a naive linear resize of a photograph looks wrong.
    let got = lab_of("#808080");
    assert!((got.l - 53.5850).abs() <= 1e-2, "L* was {}", got.l);
    assert!(got.a.abs() < NEUTRAL_RESIDUE && got.b.abs() < NEUTRAL_RESIDUE);
}

#[test]
fn the_transfer_function_has_a_linear_toe() {
    // Below 0.04045 the curve is linear; #030303 sits inside it, and treating it
    // as a pure power law would put L* noticeably lower.
    let got = lab_of("#030303");
    assert!(got.l > 0.8 && got.l < 1.0, "L* was {}", got.l);
}

/// The second half of what the fixture file asks for: these five colours,
/// through Lab, onto the palette.
#[test]
fn the_primaries_map_to_the_expected_beads() {
    let palette = Palette::generic_5mm();
    // The two saturated primaries do not land on the bead that shares their
    // name, and that is the formula working rather than failing. sRGB pure
    // green is L* 87.7, lighter than anything a manufacturer calls green, so
    // the light G25 wins over G26. sRGB pure blue carries more chroma than any
    // bead here, so the violet G38 wins over the duller G33. Both are pinned
    // because a change in either is a change in what users get handed.
    let expected = [
        ("#FFFFFF", "G01"),
        ("#000000", "G06"),
        ("#FF0000", "G15"),
        ("#00FF00", "G25"),
        ("#0000FF", "G38"),
    ];
    for (hex, code) in expected {
        let rgb = Rgb::from_hex(hex).expect("valid hex");
        let found = palette.nearest(rgb);
        let color = palette.color(found.id);
        assert_eq!(
            color.code, code,
            "{hex} mapped to {} {} at ΔE00 {:.3}",
            color.code, color.name, found.delta_e
        );
    }
}

/// A palette match is only as good as its distance, and the fixture palette is
/// 48 colours: saturated primaries are a long way from any bead.
#[test]
fn the_map_reports_how_far_it_had_to_go() {
    let palette = Palette::generic_5mm();
    let white = palette.nearest(Rgb::from_hex("#FFFFFF").expect("hex"));
    assert!(white.delta_e < 1e-9, "white is in the palette exactly");

    let pure_green = palette.nearest(Rgb::from_hex("#00FF00").expect("hex"));
    assert!(
        pure_green.delta_e > 3.0,
        "no 5 mm bead is sRGB pure green; ΔE00 was {}",
        pure_green.delta_e
    );
}
