//! The clock the caller hands the graph command is the audit clock, and only
//! that.
//!
//! `rebuild(store, at_unix_seconds)` needs a time to stamp the chain entry it
//! appends. The band underneath is scored against an instant derived from the
//! evidence itself, so two callers whose clocks disagree — a replay, a machine
//! whose time is wrong, a test that passes zero — still get the same graph.

use soul_schema::common::Timestamp;
use soul_store_api::AuditLog;
use soul_testkit::fixtures;
use soulcore::commands::{graph as graph_commands, import as import_commands};

const SEED: &str = "soulcore graph audit clock";
const EPOCH: i64 = 0;
const FAR_FUTURE: i64 = 4_102_444_800;

/// What a rebuild decided, as the store holds it afterwards.
fn bands_and_as_of(store: &soul_store::SqlCipherStore) -> Vec<(String, String, Option<String>)> {
    let mut rows: Vec<(String, String, Option<String>)> = graph_commands::load(store)
        .expect("load")
        .edges
        .iter()
        .map(|edge| {
            (
                edge.relationship_id.to_string(),
                graph_commands::band_word(edge.tie_strength.band).to_owned(),
                edge.tie_strength
                    .as_of_utc
                    .as_ref()
                    .map(Timestamp::as_str)
                    .map(str::to_owned),
            )
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn the_audit_clock_is_not_the_scoring_clock() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = soulcore::commands::store::open_test_store(dir.path(), SEED).expect("open");

    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");
    let staged = import_commands::read_soul_import_v1(&text).expect("the fixture is valid");
    import_commands::commit(&mut store, &staged, EPOCH).expect("commit");

    graph_commands::rebuild(&mut store, EPOCH).expect("rebuild at the epoch");
    let at_epoch = bands_and_as_of(&store);
    assert!(!at_epoch.is_empty(), "the fixture produces edges");
    assert!(
        at_epoch.iter().all(|(_, _, as_of)| as_of.is_some()),
        "every edge records the instant it was scored against",
    );

    graph_commands::rebuild(&mut store, FAR_FUTURE).expect("rebuild in the far future");
    assert_eq!(
        bands_and_as_of(&store),
        at_epoch,
        "moving the caller's clock by seventy years changed the graph",
    );

    let stamps: Vec<String> = store
        .list_audit()
        .expect("chain")
        .into_iter()
        .map(|entry| entry.ts.as_str().to_owned())
        .collect();
    assert!(
        stamps.iter().any(|ts| ts.starts_with("2100-")),
        "the caller's clock does reach the chain entry: {stamps:?}",
    );
}
