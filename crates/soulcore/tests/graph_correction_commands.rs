//! GC-8: correcting a tie through the command surface, and what the chain says
//! about it.
//!
//! The correction itself is proved in `soul-graph`. What is checked here is the
//! part only this layer does — the audit entry, written against the caller's
//! clock so a replay produces the same one — and the part only this layer
//! shows: a tie view that hands the interface the band, who set it, and what
//! the machine makes of the same counts, as three tokens and no sentence.

use uuid::Uuid;

use soul_schema::audit::{AuditAction, AuditDecision};
use soul_schema::common::SupportedBand;
use soul_store_api::AuditLog;
use soul_testkit::fixtures;
use soulcore::commands::graph::TieEdgeView;
use soulcore::commands::{graph as graph_commands, import as import_commands};

const SEED: &str = "soulcore graph correction commands";
const AT: i64 = 1_787_500_000;

/// Partners imported from the fixture, and the edge to the first of them.
fn seeded(dir: &std::path::Path) -> (soul_store::SqlCipherStore, Uuid) {
    let mut store = soulcore::commands::store::open_test_store(dir, SEED).expect("open");
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");
    let staged = import_commands::read_soul_import_v1(&text).expect("the fixture is valid");
    import_commands::commit(&mut store, &staged, AT).expect("commit");
    graph_commands::rebuild(&mut store, AT).expect("rebuild");

    let view = graph_commands::people_view(&store).expect("view");
    let relationship_id = view.ties[0]
        .relationship_id
        .parse()
        .expect("the view spells a uuid");
    (store, relationship_id)
}

fn tie(store: &soul_store::SqlCipherStore, relationship_id: Uuid) -> TieEdgeView {
    graph_commands::people_view(store)
        .expect("view")
        .ties
        .into_iter()
        .find(|tie| tie.relationship_id == relationship_id.to_string())
        .expect("the tie is in the view")
}

/// The chain records who was corrected and what row says so, and nothing else.
#[test]
fn a_correction_lands_on_the_chain_naming_the_edge_and_the_row() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, relationship_id) = seeded(dir.path());
    let before = store.list_audit().expect("chain").len();

    let correction =
        graph_commands::correct_tie(&mut store, relationship_id, SupportedBand::Moderate, AT)
            .expect("correct");

    let chain = store.list_audit().expect("chain");
    assert_eq!(chain.len(), before + 1, "one action, one entry");
    let entry = chain.last().expect("the entry");
    assert_eq!(entry.action, AuditAction::ProfileCorrect);
    assert_eq!(entry.decision, AuditDecision::Allowed);
    assert_eq!(
        entry.subject_refs,
        Some(vec![relationship_id, correction.evidence_id]),
        "the edge and the row that changed it, in that order",
    );
    store
        .verify_audit_chain()
        .expect("the chain still verifies");

    // Ids and vocabulary. A band word is not prose, and there is nothing else.
    let encoded = serde_json::to_string(&chain).expect("serialize");
    assert!(!encoded.contains("moderate"));
    assert!(encoded.contains(&relationship_id.to_string()));
}

/// Same call, same clock, same entry: an audit chain that moved with the wall
/// clock could not be replayed and compared.
#[test]
fn two_runs_of_the_same_correction_record_the_same_entry() {
    let left = tempfile::tempdir().expect("temp dir");
    let right = tempfile::tempdir().expect("temp dir");
    let (mut first, first_edge) = seeded(left.path());
    let (mut second, second_edge) = seeded(right.path());

    graph_commands::correct_tie(&mut first, first_edge, SupportedBand::Weak, AT).expect("correct");
    graph_commands::correct_tie(&mut second, second_edge, SupportedBand::Weak, AT)
        .expect("correct");

    let entry = |store: &soul_store::SqlCipherStore| {
        let chain = store.list_audit().expect("chain");
        let last = chain.last().expect("entry").clone();
        (
            last.action,
            last.decision,
            last.reason_code,
            last.ts,
            last.subject_refs.is_some(),
        )
    };
    assert_eq!(entry(&first), entry(&second));
}

/// The view says what the band is, who set it, and what the counts say —
/// three tokens, so the interface can draw the disagreement without this crate
/// having to write a sentence about it.
#[test]
fn the_tie_view_shows_the_band_the_lock_and_the_machines_reading() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (mut store, relationship_id) = seeded(dir.path());

    let before = tie(&store, relationship_id);
    assert!(!before.locked_by_user);
    assert_eq!(before.user_band, None);
    assert!(
        before.venue_split_measured,
        "a rebuilt edge carries the venue split the band was decided on",
    );
    assert_eq!(
        before.direct_count + before.group_count,
        before.interaction_count,
        "the two venue totals are the whole of the interaction count",
    );

    graph_commands::correct_tie(&mut store, relationship_id, SupportedBand::Strong, AT)
        .expect("correct");

    let after = tie(&store, relationship_id);
    assert!(after.locked_by_user);
    assert_eq!(after.band, "strong", "the effective band is the user's");
    assert_eq!(after.user_band.as_deref(), Some("strong"));
    assert_eq!(
        after.machine_band.as_deref(),
        Some(before.band.as_str()),
        "what the machine said before the correction is kept beside it",
    );

    // The correction row is one of the rows the view resolves for this tie.
    assert!(
        after
            .evidence
            .iter()
            .any(|row| row.kind == "user_correction"),
        "the row that changed the band is evidence like any other",
    );

    // Nothing in the view is a sentence. COPY_ZH is frozen and holds no wording
    // for a user-set band, so a tie ships as identifiers and vocabulary words
    // and the interface composes what the user reads out of frozen copy. The
    // check is for any Chinese character at all rather than for one phrase,
    // because the tempting fix is a phrase nobody has thought of yet.
    let encoded = serde_json::to_string(&after).expect("serialize");
    assert!(
        !encoded
            .chars()
            .any(|glyph| ('\u{4e00}'..='\u{9fff}').contains(&glyph)),
        "a view is not a place to invent copy: {encoded}",
    );

    let released = graph_commands::release_tie(&mut store, relationship_id, AT).expect("release");
    let back = tie(&store, relationship_id);
    assert!(!back.locked_by_user);
    assert_eq!(back.user_band, None);
    assert_eq!(back.band, graph_commands::band_word(released.machine_band));
}

/// Three words in, three bands out. Anything else is not a band this product
/// has, and the shell finds that out before it reaches the store.
#[test]
fn only_the_three_band_words_name_a_band() {
    for (key, band) in [
        ("weak", SupportedBand::Weak),
        ("moderate", SupportedBand::Moderate),
        ("strong", SupportedBand::Strong),
    ] {
        assert_eq!(graph_commands::band_named(key), Some(band));
        assert_eq!(graph_commands::band_word(band), key, "round trip");
    }
    for refused in ["", "Strong", "unknown", "very_strong", "强"] {
        assert_eq!(graph_commands::band_named(refused), None, "{refused}");
    }
}
