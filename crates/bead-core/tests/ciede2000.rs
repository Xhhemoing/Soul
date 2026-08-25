//! The CIEDE2000 acceptance table from `docs/bead/fixtures-ciede2000.md`.
//!
//! Source: Sharma, Wu and Dalal, *The CIEDE2000 Color-Difference Formula:
//! Implementation Notes, Supplementary Test Data, and Mathematical
//! Observations*, Color Research and Application 30(1), 2005, Table I.
//!
//! These pairs are stated in Lab and are fed to the formula in Lab. Pushing
//! them through an sRGB conversion first would add that conversion's error to
//! the paper's four decimal places and make a failure here ambiguous, so the
//! sRGB pipeline is pinned separately in `srgb_lab.rs`.

use bead_core::color::{ciede2000, Lab};

/// The fixture file's stated tolerance.
const TOLERANCE: f64 = 0.0001;

/// `(L1, a1, b1, L2, a2, b2, expected ΔE00)`.
const SHARMA_TABLE: &[(f64, f64, f64, f64, f64, f64, f64)] = &[
    (50.0000, 2.6772, -79.7751, 50.0000, 0.0000, -82.7485, 2.0425),
    (50.0000, 3.1571, -77.2803, 50.0000, 0.0000, -82.7485, 2.8615),
    (50.0000, 2.8361, -74.0200, 50.0000, 0.0000, -82.7485, 3.4412),
    (
        50.0000, -1.3802, -84.2814, 50.0000, 0.0000, -82.7485, 1.0000,
    ),
    (
        50.0000, -1.1848, -84.8006, 50.0000, 0.0000, -82.7485, 1.0000,
    ),
    (
        50.0000, -0.9009, -85.5211, 50.0000, 0.0000, -82.7485, 1.0000,
    ),
    (50.0000, 0.0000, 0.0000, 50.0000, -1.0000, 2.0000, 2.3669),
    (50.0000, -1.0000, 2.0000, 50.0000, 0.0000, 0.0000, 2.3669),
    (50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0009, 7.1792),
    (50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0010, 7.1792),
    (50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0011, 7.2195),
    (50.0000, 2.4900, -0.0010, 50.0000, -2.4900, 0.0012, 7.2195),
    (50.0000, -0.0010, 2.4900, 50.0000, 0.0009, -2.4900, 4.8045),
    (50.0000, -0.0010, 2.4900, 50.0000, 0.0010, -2.4900, 4.8045),
    (50.0000, -0.0010, 2.4900, 50.0000, 0.0011, -2.4900, 4.7461),
    (50.0000, 2.5000, 0.0000, 50.0000, 0.0000, -2.5000, 4.3065),
    (50.0000, 2.5000, 0.0000, 73.0000, 25.0000, -18.0000, 27.1492),
    (50.0000, 2.5000, 0.0000, 61.0000, -5.0000, 29.0000, 22.8977),
    (50.0000, 2.5000, 0.0000, 56.0000, -27.0000, -3.0000, 31.9030),
    (50.0000, 2.5000, 0.0000, 58.0000, 24.0000, 15.0000, 19.4535),
];

#[test]
fn matches_the_published_table() {
    assert_eq!(SHARMA_TABLE.len(), 20, "the fixture file lists 20 pairs");
    for (row, (l1, a1, b1, l2, a2, b2, expected)) in SHARMA_TABLE.iter().enumerate() {
        let got = ciede2000(Lab::new(*l1, *a1, *b1), Lab::new(*l2, *a2, *b2));
        assert!(
            (got - expected).abs() <= TOLERANCE,
            "pair {}: expected {expected}, got {got} (off by {})",
            row + 1,
            (got - expected).abs()
        );
    }
}

/// Pairs 9 to 15 are the paper's discontinuity cases: a hue difference that
/// crosses the 0/360 seam, where a naive mean hue jumps by 180 degrees. Listing
/// them again on their own means a regression there names itself.
#[test]
fn handles_the_hue_seam_cases() {
    let seam = [
        (
            Lab::new(50.0, 2.49, -0.001),
            Lab::new(50.0, -2.49, 0.0009),
            7.1792,
        ),
        (
            Lab::new(50.0, 2.49, -0.001),
            Lab::new(50.0, -2.49, 0.0011),
            7.2195,
        ),
        (
            Lab::new(50.0, -0.001, 2.49),
            Lab::new(50.0, 0.0011, -2.49),
            4.7461,
        ),
    ];
    for (a, b, expected) in seam {
        assert!((ciede2000(a, b) - expected).abs() <= TOLERANCE);
    }
}

/// With one chroma at zero the hue difference is undefined and the notes fix it
/// at zero. Pairs 7 and 8 exercise that from both sides, so the formula has to
/// stay symmetric across it.
#[test]
fn is_symmetric() {
    for (l1, a1, b1, l2, a2, b2, _) in SHARMA_TABLE {
        let forward = ciede2000(Lab::new(*l1, *a1, *b1), Lab::new(*l2, *a2, *b2));
        let backward = ciede2000(Lab::new(*l2, *a2, *b2), Lab::new(*l1, *a1, *b1));
        assert!((forward - backward).abs() < 1e-12);
    }
}

#[test]
fn a_colour_is_zero_from_itself() {
    for (l1, a1, b1, ..) in SHARMA_TABLE {
        let colour = Lab::new(*l1, *a1, *b1);
        assert_eq!(ciede2000(colour, colour), 0.0);
    }
}

#[test]
fn a_grey_pair_reduces_to_scaled_lightness() {
    // Both chromas are zero, so only the lightness term survives and S_L is
    // 1 + 0.015*(L̄-50)^2 / sqrt(20 + (L̄-50)^2).
    let got = ciede2000(Lab::new(40.0, 0.0, 0.0), Lab::new(60.0, 0.0, 0.0));
    let s_l = 1.0 + (0.015 * 0.0) / (20.0f64 + 0.0).sqrt();
    assert!((got - 20.0 / s_l).abs() < 1e-12);
}
