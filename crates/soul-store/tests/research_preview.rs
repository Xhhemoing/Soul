//! AC-20: preview what research would see, with third-party rows excluded and
//! nothing written anywhere.
//!
//! The fixture deliberately contains third-party events, including a kind and
//! an hour bucket that no owner event shares. If the exclusion were a constant
//! rather than a filter, that bucket would show up in the rows and the count of
//! excluded candidates would be zero; both are asserted against.
//!
//! The fixture also holds owner rows of both dispositions: `app.foreground`
//! marked `research_export: bucket` the way the collector writes it, and
//! `import.item` marked `deny` the way `soul-import` writes it. Being the
//! owner's is not enough — the row has to say research may count it — and a
//! filter that only looked at `privacy_subject` would publish the imported
//! ones.
//!
//! Three separate things are checked, because passing any one of them alone
//! would still leave the promise broken:
//!
//! * the manifest validates against the frozen `export-manifest` contract, and
//!   says `third_party_rows: 0`, `written_to_disk: false`;
//! * the rendered preview survives the leakage checker with the third-party
//!   prose and names loaded into its corpus;
//! * no file appears, and the module's own source contains no way to make one.

mod common;

use std::collections::BTreeSet;

use common::*;

use soul_schema::contact::ContactClass;
use soul_schema::event::EventKind;
use soul_schema::export_manifest::ExportKind;
use soul_schema::validate::SchemaId;
use soul_schema::{EvidenceBand, SchemaSet, Subject};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::research::{ResearchPreview, ResearchPreviewRequest};
use soul_store_api::{BlobStore, EventStore, GraphStore, ProfileStore, SoulStore};
use soul_testkit::LeakageChecker;

const SEED: &str = "wp02 research";

/// Third-party prose and names that must not reach the preview.
const THEIR_MESSAGE: &str = "李雷说他下周要去上海出差，让我帮忙照看一下猫";
const THEIR_NAME: &str = "李雷";
const THEIR_HANDLE: &str = "@lilei_1990";

/// A kind and hour that only third-party events occupy, so an exclusion that
/// silently stopped working would leave a visible trace in the rows.
const THIRD_PARTY_ONLY_BUCKET: &str = "2026-08-25T22:00Z";

fn seed(store: &mut SqlCipherStore) {
    // Owner events the collector wrote, marked `bucket`: two share one bucket
    // and kind, one sits on its own.
    for (index, ts) in [
        "2026-08-24T09:10:00Z",
        "2026-08-24T09:40:00Z",
        "2026-08-24T13:05:00Z",
    ]
    .iter()
    .enumerate()
    {
        store
            .append_event(bucketed_event(
                id(&format!("1{index:02}")),
                ts,
                EventKind::AppForeground,
                Subject::Owner,
            ))
            .expect("owner event");
    }

    // Owner events an import wrote, marked `deny`. They are the owner's own
    // messages and hours, and `soul-import` stores them with research egress
    // denied; a preview that published them would be reading the subject and
    // ignoring the policy written beside it.
    for (index, ts) in ["2026-08-24T09:20:00Z", "2026-08-24T18:00:00Z"]
        .iter()
        .enumerate()
    {
        store
            .append_event(event(
                id(&format!("1{:02}", index + 10)),
                ts,
                EventKind::ImportItem,
                Subject::Owner,
            ))
            .expect("owner import event");
    }

    // Third-party events, one of them carrying sealed prose.
    let their_body = store
        .seal(third_party_seal(
            id("60"),
            id("120"),
            "body_ref",
            THEIR_MESSAGE,
        ))
        .expect("seal their message");
    let mut observed = event(
        id("120"),
        "2026-08-25T22:15:00Z",
        EventKind::MessageObserved,
        Subject::ThirdParty,
    );
    observed.body_ref = Some(their_body);
    store.append_event(observed).expect("their event");

    store
        .append_event(event(
            id("121"),
            "2026-08-25T22:45:00Z",
            EventKind::MessageObserved,
            Subject::Mixed,
        ))
        .expect("a mixed event is handled as third party");

    // A third-party contact whose display label is sealed prose.
    let label = store
        .seal(third_party_seal(
            id("61"),
            id("40"),
            "display_label_ref",
            THEIR_NAME,
        ))
        .expect("seal their name");
    let mut them = contact(id("40"), ContactClass::ThirdParty);
    them.display_label_ref = Some(label);
    store.put_contact(them).expect("their contact row");

    store
        .put_profile(profile(
            id("30"),
            &[
                (id("31"), EvidenceBand::Moderate),
                // An axis with no evidence behind it is not something research
                // may see, and `none` is not a band the manifest admits.
                (id("32"), EvidenceBand::None),
            ],
        ))
        .expect("profile");
}

fn leakage_checker() -> LeakageChecker {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("their-message", THEIR_MESSAGE);
    checker.add_known_identifier("their-name", THEIR_NAME);
    checker.add_known_identifier("their-handle", THEIR_HANDLE);
    checker
}

#[test]
fn the_preview_reports_real_rows_and_excludes_the_third_party_ones() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("research.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    seed(&mut store);

    let report = store
        .research_preview(&ResearchPreviewRequest::default().with_manifest_id(id("aa")))
        .expect("preview");
    let manifest = &report.manifest;

    assert!(
        !manifest.rows.is_empty(),
        "rows are required and must come from the database",
    );
    assert!(
        report.third_party_rows_excluded > 0,
        "the fixture has third-party events in it; a zero here means nothing was excluded \
         because nothing was looked at",
    );
    assert!(
        report.deny_rows_excluded > 0,
        "the fixture has owner rows stored `research_export: deny` in it; a zero here means \
         the disposition was never read",
    );
    assert_eq!(
        report.candidate_rows_total,
        report.third_party_rows_excluded + report.deny_rows_excluded + manifest.rows.len() as u64,
        "every candidate row is either published or excluded, and it is one of the two",
    );
    assert!(
        manifest
            .rows
            .iter()
            .all(|row| row.event_kind.as_deref() != Some("import.item")),
        "the imported rows are the owner's own and are stored `deny`; being about the owner \
         is not what decides this",
    );

    // The two owner events in the same hour and of the same kind are one row
    // with a count of two, which no constant would produce.
    let morning = manifest
        .rows
        .iter()
        .find(|row| row.time_bucket_utc.as_deref() == Some("2026-08-24T09:00Z"))
        .expect("the morning bucket must be present");
    assert_eq!(morning.aggregate_count, Some(2));
    assert_eq!(morning.event_kind.as_deref(), Some("app.foreground"));

    let buckets: BTreeSet<&str> = manifest
        .rows
        .iter()
        .filter_map(|row| row.time_bucket_utc.as_deref())
        .collect();
    assert!(
        !buckets.contains(THIRD_PARTY_ONLY_BUCKET),
        "an hour that only third-party events occupy must not appear at all",
    );
    assert!(
        manifest
            .rows
            .iter()
            .all(|row| row.event_kind.as_deref() != Some("message.observed")),
        "the only observed messages in the fixture are somebody else's",
    );

    // The owner's trait axis is exported; the one with no evidence is not.
    let axes: Vec<&str> = manifest
        .rows
        .iter()
        .filter_map(|row| row.self_trait_axis.as_deref())
        .collect();
    assert_eq!(axes, vec![id("31").to_string()]);

    assert_eq!(manifest.export_kind, ExportKind::ResearchPreview);
    assert!(
        !manifest.written_to_disk,
        "a research preview may not claim to have been written",
    );
}

/// Two rows that differ in nothing but the disposition written on them.
///
/// Same owner, same kind, same hour. The one marked `bucket` is published and
/// the one marked `deny` is not, which is only possible if the disposition is
/// read per row rather than inferred from the subject or from the kind.
#[test]
fn two_owner_rows_of_one_kind_and_hour_part_on_the_disposition_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("split.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");

    store
        .append_event(bucketed_event(
            id("70"),
            "2026-08-24T09:10:00Z",
            EventKind::AppForeground,
            Subject::Owner,
        ))
        .expect("a collected hour");
    store
        .append_event(event(
            id("71"),
            "2026-08-24T09:40:00Z",
            EventKind::AppForeground,
            Subject::Owner,
        ))
        .expect("the same hour, stored `deny`");

    let report = store
        .research_preview(&ResearchPreviewRequest::default().with_manifest_id(id("ac")))
        .expect("preview");

    assert_eq!(report.candidate_rows_total, 2, "one group per disposition");
    assert_eq!(report.third_party_rows_excluded, 0, "nobody else is here");
    assert_eq!(report.deny_rows_excluded, 1);
    assert_eq!(
        report.manifest.rows.len(),
        1,
        "the denied hour was published too: {:?}",
        report.manifest.rows,
    );
    assert_eq!(
        report.manifest.rows[0].aggregate_count,
        Some(1),
        "a count of two is the two hours added together, which is the bug",
    );
}

#[test]
fn the_preview_validates_against_the_frozen_export_contract() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("research.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    seed(&mut store);

    let report = store
        .research_preview(&ResearchPreviewRequest::default().with_manifest_id(id("ab")))
        .expect("preview");

    let schemas = SchemaSet::load().expect("the frozen contracts compile");
    let rendered = schemas
        .validate_model(SchemaId::ExportManifest, &report.manifest)
        .expect("the preview must satisfy export-manifest.schema.json");

    assert_eq!(rendered["third_party_rows"], serde_json::json!(0));
    assert_eq!(rendered["written_to_disk"], serde_json::json!(false));
    assert_eq!(
        rendered["redaction_profile"]["third_party_body"],
        serde_json::json!("excluded"),
    );

    let text = serde_json::to_string(&rendered).expect("serialize the preview");
    leakage_checker().assert_clean("the research preview", &text);
}

#[test]
fn a_preview_leaves_no_new_file_behind() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("research.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");
    seed(&mut store);
    store.flush().expect("settle the database files first");

    let before = entries(dir.path());
    for _ in 0..3 {
        store
            .research_preview(&ResearchPreviewRequest::default())
            .expect("preview");
    }
    let after = entries(dir.path());

    assert_eq!(
        before, after,
        "a research preview must not create, or grow into, any file",
    );
}

/// The runtime check above can only see the paths a test happens to exercise.
/// This one reads the module back and shows there is no way to reach the
/// filesystem from it at all.
#[test]
fn the_research_module_cannot_reach_the_filesystem() {
    const SOURCE: &str = include_str!("../src/research_preview.rs");

    let forbidden = [
        "fs::write",
        "fs::create_dir",
        "create_dir_all",
        "File::create",
        "OpenOptions",
        "fs::copy",
        "fs::rename",
        "BufWriter",
        "std::io::Write",
        "tempfile",
        "PathBuf",
    ];
    for needle in forbidden {
        assert!(
            !SOURCE.contains(needle),
            "research_preview.rs mentions `{needle}`; v0.1 research must not write anything",
        );
    }

    // Guard against the assertion above passing because the file moved.
    assert!(
        SOURCE.contains("fn research_preview"),
        "the source being scanned should be the research preview module",
    );
}

/// Name, size and modification-agnostic snapshot: names plus byte lengths.
fn entries(dir: &std::path::Path) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = std::fs::read_dir(dir)
        .expect("read the store directory")
        .map(|entry| {
            let entry = entry.expect("directory entry");
            let length = entry.metadata().expect("metadata").len();
            (entry.file_name().to_string_lossy().into_owned(), length)
        })
        .collect();
    out.sort();
    out
}
