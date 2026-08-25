//! D52: the two frozen crates keep their own day constants, and the workspace
//! pins them equal.
//!
//! `soul-algo-tie` demotes a band after a run of silent days;
//! `soul-algo-trait` prefixes the people summary with a warning that the
//! archive below is history rather than a current frequency, after a run of
//! silent days. They are two crates and two constants deliberately — merging
//! them was postponed, not decided — but they describe the same moment, and
//! this is the crate where a divergence would surface as a product defect: a
//! summary that still reads as current about a tie the graph has already
//! demoted, or the reverse.
//!
//! The equality is asserted between the two names. No number is written down
//! here, because writing one would be the third copy the decision forbids, and
//! the second test is what keeps that ban true of the product crates.

use std::path::{Path, PathBuf};

use soul_algo_tie::constants::DEMOTE_ONE_BAND_DAYS;
use soul_algo_trait::DORMANT_AFTER_DAYS;

/// The crates that ship, plus the shell. None of them may hold a day
/// threshold of its own; they read the band and the silent-day count off the
/// edge the rebuild wrote.
const PRODUCT_SOURCES: &[&str] = &[
    "crates/soul-graph/src",
    "crates/soul-draft/src",
    "crates/soul-profile/src",
    "crates/soul-import/src",
    "crates/soulcore/src",
    "apps/desktop/src",
];

#[test]
fn the_demotion_clock_and_the_dormancy_clock_are_the_same_day() {
    assert_eq!(
        DEMOTE_ONE_BAND_DAYS, DORMANT_AFTER_DAYS,
        "the graph demotes a tie on one day and the summary calls it dormant on \
         another; a user would read a band and a warning that disagree",
    );
}

/// The other half of D52: neither number appears a third time.
///
/// A product crate that spelled the threshold out would go on compiling after
/// the frozen crates moved theirs, and the test above would stay green while
/// the screen showed the old cut-off. So the ban is checked where it applies —
/// on the shipped sources, not on the tests, which are entitled to name the
/// day they are pinning.
#[test]
fn no_product_crate_writes_a_day_threshold_of_its_own() {
    let forbidden = [
        DEMOTE_ONE_BAND_DAYS.to_string(),
        soul_algo_tie::constants::WEAK_AFTER_SILENT_DAYS.to_string(),
    ];
    let root = soul_testkit::fixtures::repo_root();

    let mut offences = Vec::new();
    for relative in PRODUCT_SOURCES {
        for path in sources_under(&root.join(relative)) {
            let text = std::fs::read_to_string(&path).expect("read");
            for (number, line) in text
                .lines()
                .enumerate()
                .filter(|(_, line)| !is_comment(line))
                .flat_map(|(number, line)| forbidden.iter().map(move |n| (number, line, n)))
                .filter(|(_, line, wanted)| contains_number(line, wanted))
                .map(|(number, line, _)| (number, line))
            {
                offences.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(&root).unwrap_or(&path).display(),
                    number + 1,
                    line.trim(),
                ));
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a day threshold is spelled out in a product crate; read it off the edge \
         instead:\n{}",
        offences.join("\n"),
    );
}

/// Rust and TypeScript sources under one directory, tests excluded: a `.test`
/// file is allowed to name the day it is pinning.
fn sources_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let is_source =
                name.ends_with(".rs") || name.ends_with(".ts") || name.ends_with(".tsx");
            if is_source && !name.contains(".test.") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// A whole-number match, so `1800` and `2360` are not the threshold.
fn contains_number(line: &str, wanted: &str) -> bool {
    line.match_indices(wanted).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + wanted.len()..].chars().next();
        let boundary = |c: Option<char>| !c.is_some_and(|c| c.is_ascii_digit() || c == '_');
        boundary(before) && boundary(after)
    })
}

fn is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with('*')
}
