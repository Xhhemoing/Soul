//! T-PAL-1, T-PAL-2, T-PAL-3: the `generic-5mm` fixture and nearest-colour
//! lookup.

use bead_core::color::{ciede2000, Lab, Rgb};
use bead_core::palette::{BeadColor, ColorId, Palette, PaletteError};

mod support;

#[test]
fn the_fixture_is_forty_eight_uniquely_coded_colours() {
    let palette = Palette::generic_5mm();
    assert_eq!(palette.namespace(), "generic-5mm");
    assert_eq!(palette.len(), 48);

    let mut codes: Vec<&str> = palette.iter().map(|(_, c)| c.code.as_str()).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), 48, "every code is distinct");
    assert_eq!(codes.first(), Some(&"G01"));
    assert_eq!(codes.last(), Some(&"G48"));
}

#[test]
fn ids_are_positions_and_codes_resolve_back_to_them() {
    let palette = Palette::generic_5mm();
    for (id, color) in palette.iter() {
        assert_eq!(palette.find_code(&color.code), Some(id));
        assert_eq!(palette.color(id).code, color.code);
    }
    assert_eq!(palette.find_code("nope"), None);
    assert!(palette.get(ColorId(9_999)).is_none());
}

/// The Lab value stored next to each swatch has to be the swatch's own Lab, or
/// every distance in the crate is measured from the wrong place.
#[test]
fn stored_lab_matches_the_swatch() {
    let palette = Palette::generic_5mm();
    for (_, color) in palette.iter() {
        let recomputed = color.rgb.to_lab();
        assert!((color.lab.l - recomputed.l).abs() < 1e-12);
        assert!((color.lab.a - recomputed.a).abs() < 1e-12);
        assert!((color.lab.b - recomputed.b).abs() < 1e-12);
    }
}

/// A fixture with two colours a person cannot tell apart would make the ΔE < 3
/// substitute rule vacuous, so the separation is asserted rather than assumed.
/// The closest pair in the fixture is G43 Light Brown and G48 Skin Deep.
#[test]
fn no_two_fixture_colours_are_within_the_substitute_threshold() {
    let palette = Palette::generic_5mm();
    let mut closest = f64::INFINITY;
    let mut which = (String::new(), String::new());
    for (a_id, a) in palette.iter() {
        for (b_id, b) in palette.iter() {
            if b_id <= a_id {
                continue;
            }
            let delta_e = ciede2000(a.lab, b.lab);
            if delta_e < closest {
                closest = delta_e;
                which = (a.code.clone(), b.code.clone());
            }
        }
    }
    assert!(
        closest > 3.0,
        "{} and {} are only ΔE00 {closest:.4} apart",
        which.0,
        which.1
    );
    assert_eq!(which, ("G43".to_owned(), "G48".to_owned()));
    assert!((closest - 5.3575).abs() < 0.001, "closest was {closest:.4}");
}

/// T-PAL-1.
#[test]
fn every_swatch_finds_itself_at_zero() {
    let palette = Palette::generic_5mm();
    for (id, color) in palette.iter() {
        let found = palette.nearest(color.rgb);
        assert_eq!(found.id, id, "{} matched something else", color.code);
        assert!(found.delta_e < 1e-9);
    }
}

/// The pinned nearest-colour map. These are the answers the browser port in
/// WP-B03 has to reproduce, so a change to any of them is a contract change.
#[test]
fn known_pixels_map_to_known_beads() {
    let palette = Palette::generic_5mm();
    let cases = [
        ("#FEFEFE", "G01", "almost white"),
        ("#F2ECDD", "G02", "off-white paper"),
        ("#D0D0D2", "G03", "light grey"),
        ("#040404", "G06", "near black"),
        ("#E30D30", "G15", "signal red"),
        ("#F58220", "G18", "safety orange"),
        ("#FFD500", "G22", "school-bus yellow"),
        ("#2F9F46", "G26", "leaf green"),
        ("#0C62A5", "G33", "denim blue"),
        ("#7A4A24", "G44", "chocolate brown"),
    ];
    for (hex, code, what) in cases {
        let rgb = Rgb::from_hex(hex).expect("valid hex");
        let found = palette.nearest(rgb);
        let color = palette.color(found.id);
        assert_eq!(
            color.code, code,
            "{what} ({hex}) mapped to {} at ΔE00 {:.4}",
            color.code, found.delta_e
        );
        assert!(
            found.delta_e < 1.5,
            "{what} should be a near miss, was ΔE00 {:.4}",
            found.delta_e
        );
    }
}

/// T-PAL-2. An exact tie must be decided by palette order, not by whichever
/// entry the loop happened to see last. The contract puts indices 3 and 7 on
/// the same swatch and demands index 3.
#[test]
fn an_exact_tie_goes_to_the_lower_index() {
    let twin = Rgb::new(0x40, 0x80, 0xC0);
    let mut colors: Vec<BeadColor> = (0..10)
        .map(|i| {
            BeadColor::new(
                format!("T{i:02}"),
                format!("filler {i}"),
                Rgb::new(i * 9, 0, 0),
            )
        })
        .collect();
    colors[3] = BeadColor::new("T03", "twin one", twin);
    colors[7] = BeadColor::new("T07", "twin two", twin);
    let palette = Palette::new("twins", colors).expect("valid palette");

    assert_eq!(palette.nearest(twin).id, ColorId(3));
    // And the runner-up really is the other twin, at the same distance.
    let (best, second) = palette.nearest_two(twin.to_lab());
    assert_eq!(best.id, ColorId(3));
    let second = second.expect("a runner-up");
    assert_eq!(second.id, ColorId(7));
    assert_eq!(second.delta_e, best.delta_e);
    assert_eq!(palette.decision_margin(twin.to_lab()), 0.0);
}

/// T-PAL-3. A fixed-seed sweep: whatever comes in, the answer is a real index
/// and nothing panics.
#[test]
fn any_pixel_maps_to_a_real_palette_index() {
    let palette = Palette::generic_5mm();
    let mut random = support::Lcg::new(0xBEAD_5EED);
    for _ in 0..2_000 {
        let rgb = Rgb::new(random.byte(), random.byte(), random.byte());
        let found = palette.nearest(rgb);
        assert!(
            palette.get(found.id).is_some(),
            "{rgb:?} produced {} which is not in the palette",
            found.id
        );
        assert!(
            found.delta_e.is_finite() && found.delta_e >= 0.0,
            "{rgb:?} produced ΔE00 {}",
            found.delta_e
        );
    }
}

#[test]
fn within_returns_candidates_nearest_first() {
    let palette = Palette::generic_5mm();
    let target = palette.color(palette.find_code("G43").expect("G43")).lab;
    let near = palette.within(target, 8.0);
    assert!(near.len() >= 2);
    assert_eq!(palette.color(near[0].id).code, "G43");
    assert!(near[0].delta_e < 1e-9);
    assert_eq!(palette.color(near[1].id).code, "G48");
    for pair in near.windows(2) {
        assert!(pair[0].delta_e <= pair[1].delta_e, "sorted by distance");
    }
}

#[test]
fn an_empty_palette_is_refused() {
    assert_eq!(
        Palette::new("void", Vec::new()).unwrap_err(),
        PaletteError::Empty
    );
}

#[test]
fn nearest_lab_and_nearest_rgb_agree() {
    let palette = Palette::generic_5mm();
    let rgb = Rgb::new(120, 60, 200);
    assert_eq!(
        palette.nearest(rgb).id,
        palette.nearest_lab(rgb.to_lab()).id
    );
    // And a Lab value that came from nowhere in particular still resolves.
    assert!(palette
        .get(palette.nearest_lab(Lab::new(50.0, 0.0, 0.0)).id)
        .is_some());
}
