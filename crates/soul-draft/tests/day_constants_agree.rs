//! D52: the day the tie rule demotes on and the day the summary calls a tie
//! dormant are the same day, and neither of them is written down here.
//!
//! Two frozen crates hold a number about silence. `soul_algo_tie` demotes a
//! band after [`DEMOTE_ONE_BAND_DAYS`] of it; `soul_algo_trait`'s A2 renderer
//! adds a dormancy sentence after [`DORMANT_AFTER_DAYS`] of it. They are
//! deliberately different kinds of rule — one changes a verdict, the other adds
//! a line of description — but they are answering the same question about the
//! same edge, and a user who is told 「已经 N 天没有新的往来了」 on one screen
//! and sees an undemoted band on the next has been shown two Souls.
//!
//! The product is where that would surface, so the product is where it is
//! pinned. This file is the only place in the product tree that mentions either
//! constant, and it mentions them by name: nothing in `soul-draft` or
//! `soul-graph` writes the number, so there is no third copy to drift. If the
//! two frozen crates ever need to disagree, this test is the thing that has to
//! be deleted on purpose, with the argument written next to the deletion.

use soul_algo_tie::constants::{DEMOTE_ONE_BAND_DAYS, WEAK_AFTER_SILENT_DAYS};
use soul_algo_trait::a2::DORMANT_AFTER_DAYS;

/// The pin itself. Named after the two constants rather than the value, so a
/// failure says which pair drifted and not which number changed.
#[test]
fn the_demotion_day_and_the_dormancy_day_are_the_same_day() {
    assert_eq!(
        DEMOTE_ONE_BAND_DAYS, DORMANT_AFTER_DAYS,
        "the summary calls a tie dormant on the day the tie rule demotes it, \
         or the user reads two different accounts of the same silence",
    );
}

/// The dormancy line describes the first step, not the last one.
///
/// `WEAK_AFTER_SILENT_DAYS` is the harder cap and is a second threshold A2 does
/// not have. That is fine — A2 says one thing about silence rather than two —
/// but it must stay on the far side of the day A2 does speak on, otherwise the
/// summary would go quiet about a gap that had already cost the tie everything.
#[test]
fn the_dormancy_line_arrives_no_later_than_the_floor() {
    // Read into locals first: the comparison is between two constants, and
    // `assert!` over one of those is a line the compiler deletes.
    let (line, floor) = (DORMANT_AFTER_DAYS, WEAK_AFTER_SILENT_DAYS);
    assert!(line <= floor, "{line} > {floor}");
}

/// Neither number is restated in the product tree.
///
/// The pin above only holds while the two frozen crates are the only authors.
/// A `const DORMANT_DAYS: i64 = 180;` in `soul-draft` or `soul-graph` would
/// satisfy every assertion in this file and still be the drift it is meant to
/// prevent, so the check is on the source rather than on the value.
#[test]
fn no_product_crate_writes_a_silence_threshold_of_its_own() {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/soul-draft sits inside crates/");

    let spelled: Vec<String> = [DEMOTE_ONE_BAND_DAYS, WEAK_AFTER_SILENT_DAYS]
        .iter()
        .map(|days| days.to_string())
        .collect();

    let mut pending = vec![crates.join("soul-draft/src"), crates.join("soul-graph/src")];
    let mut scanned = 0usize;
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path).expect("read a directory") {
                pending.push(entry.expect("an entry").path());
            }
            continue;
        }
        if path.extension().is_none_or(|kind| kind != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read a source file");
        scanned += 1;
        for days in &spelled {
            assert!(
                !text.contains(days.as_str()),
                "{} spells out {days}; read it from soul-algo-tie instead",
                path.display(),
            );
        }
    }
    assert!(scanned > 5, "the scan found almost no source to read");
}
