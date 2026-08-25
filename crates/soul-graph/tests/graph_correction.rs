//! GC-1..GC-5 and GC-10: the user overrules a band, and it stays overruled.
//!
//! PRODUCT_LOCK lists the people graph among the things the user can see *and
//! correct*, and non-negotiable constraint 10 rules out an uncorrectable black
//! box. What that costs in practice is three properties, and each of them is a
//! test here: the correction lands, a rebuild does not undo it, and there is a
//! way back out.
//!
//! Everything runs against the real store rather than a fake, so a correction
//! that would fail the frozen contract — an edge citing evidence that does not
//! resolve, a `tie_strength` the schema rejects — fails here rather than on a
//! user's machine.

use uuid::Uuid;

use soul_graph::correct::{corrected_relationship, CORRECTION_ORIGIN, RELEASE_ORIGIN};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{correct_tie, release_tie, Direction, GraphError, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::evidence::EvidenceKind;
use soul_schema::inference::UserVerdict;
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::types::StoreError;
use soul_store_api::{AuditLog, GraphStore, ProfileStore};

const SEED: &str = "wp05 graph correction";

/// A fixed clock. Every correction in this file is recorded at the same
/// instant, so a replay of the file produces the same audit entries.
const NOW: i64 = 1_787_529_600;

/// A UUIDv7-shaped literal, so a failure names the same person every run.
fn id(tail: &str) -> Uuid {
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail:0>3}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn owner() -> Uuid {
    id("001")
}

fn peer() -> Uuid {
    id("002")
}

fn open(dir: &std::path::Path) -> SqlCipherStore {
    SqlCipherStore::open(dir.join("soul.db"), &TestKeyProvider::from_seed(SEED)).expect("open")
}

fn contact(contact_id: Uuid, class: ContactClass) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class: class,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

fn observe(store: &mut SqlCipherStore, evidence_id: Uuid, direction: Direction, at: &str) {
    let observation = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        peer(),
        conversation_ref("test", "c-01"),
        direction,
        Timestamp::new(at),
        Venue::Direct,
    );
    let subject = match direction {
        Direction::Outgoing => Subject::Mixed,
        Direction::Incoming => Subject::ThirdParty,
    };
    store
        .put_evidence(interaction_evidence(evidence_id, subject, &observation))
        .expect("evidence");
}

/// One peer, talked to daily and both ways, which is as close as this fixture
/// gets to a tie the machine has an opinion about.
fn seeded(store: &mut SqlCipherStore) -> Uuid {
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("owner");
    store
        .put_contact(contact(peer(), ContactClass::ThirdParty))
        .expect("peer");

    let mut next = 100u32;
    let mut evidence_id = || {
        next += 1;
        id(&next.to_string())
    };
    for day in 18..24 {
        observe(
            store,
            evidence_id(),
            Direction::Outgoing,
            &format!("2026-08-{day}T09:00:00Z"),
        );
        observe(
            store,
            evidence_id(),
            Direction::Incoming,
            &format!("2026-08-{day}T09:05:00Z"),
        );
    }

    soul_graph::rebuild(store).expect("rebuild");
    let graph = soul_graph::load(store).expect("load");
    graph.edges_for(peer())[0].relationship_id
}

/// The strength on one edge, read back off the store rather than off whatever
/// the last call returned.
fn strength(store: &SqlCipherStore, relationship_id: Uuid) -> soul_graph::TieStrength {
    let graph = soul_graph::load(store).expect("load");
    graph
        .edge(relationship_id)
        .expect("the edge is there")
        .tie_strength
        .clone()
}

fn verdict(store: &SqlCipherStore, relationship_id: Uuid) -> Option<UserVerdict> {
    store
        .list_inferences()
        .expect("inferences")
        .into_iter()
        .find(|inference| {
            inference.target["relationship_id"].as_str() == Some(&relationship_id.to_string())
        })
        .expect("the edge has a tie inference")
        .user_verdict
}

/// GC-1: the correction lands, and a rebuild leaves it alone.
///
/// Three things have to survive the rebuild together — the effective band, the
/// machine's own reading beside it, and the verdict on the inference. Any one
/// of them alone is a lock that looks kept and is not: a band with no verdict
/// loses the record that a person decided it, and a verdict with no band is the
/// decorative `user_verdict` this graph shipped with.
#[test]
fn a_corrected_band_and_its_verdict_survive_a_rebuild() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);

    let machine_band = strength(&store, relationship_id).band;
    assert_eq!(
        machine_band,
        SupportedBand::Strong,
        "the fixture is meant to be a tie the machine calls strong",
    );

    let correction =
        correct_tie(&mut store, relationship_id, SupportedBand::Moderate, NOW).expect("correct");
    assert_eq!(correction.band, SupportedBand::Moderate);
    assert_eq!(correction.machine_band, SupportedBand::Strong);
    assert!(correction.overrides_machine());

    for stage in ["straight after the correction", "after a rebuild"] {
        let held = strength(&store, relationship_id);
        assert_eq!(held.band, SupportedBand::Moderate, "band, {stage}");
        assert_eq!(
            held.user_band,
            Some(SupportedBand::Moderate),
            "user, {stage}"
        );
        assert_eq!(
            held.machine_band,
            Some(SupportedBand::Strong),
            "machine, {stage}",
        );
        assert!(held.is_locked_by_user(), "lock, {stage}");
        assert_eq!(
            verdict(&store, relationship_id),
            Some(UserVerdict::Corrected),
            "verdict, {stage}",
        );
        soul_graph::rebuild(&mut store).expect("rebuild");
    }
}

/// GC-2: the counts are not what the user corrected, so they keep moving.
///
/// The band is pinned; the numbers under it are observations, and an
/// observation does not stop being one because the user disagreed with the
/// word derived from it. The machine's reading is asserted against the
/// inference the same rebuild wrote rather than against a literal, because the
/// scorer is frozen elsewhere and this test is not a second copy of it.
#[test]
fn a_locked_edge_keeps_counting_and_keeps_its_band() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);
    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("correct");
    let before = strength(&store, relationship_id).interaction_count;

    for minute in 0..20 {
        observe(
            &mut store,
            id(&format!("2{minute:02}")),
            match minute % 2 {
                0 => Direction::Outgoing,
                _ => Direction::Incoming,
            },
            &format!("2026-08-26T10:{minute:02}:00Z"),
        );
    }
    soul_graph::rebuild(&mut store).expect("rebuild");

    let held = strength(&store, relationship_id);
    assert_eq!(
        held.interaction_count,
        before + 20,
        "a lock is on the band, not on the counting",
    );
    assert_eq!(
        held.band,
        SupportedBand::Weak,
        "the user's band is still it"
    );
    assert_eq!(held.user_band, Some(SupportedBand::Weak));

    let machine = store
        .list_inferences()
        .expect("inferences")
        .into_iter()
        .find(|inference| {
            inference.target["relationship_id"].as_str() == Some(&relationship_id.to_string())
        })
        .expect("tie inference");
    assert_eq!(
        held.machine_band,
        Some(machine.evidence_band),
        "the machine's reading on the edge and on its inference are one reading",
    );
    assert!(
        machine
            .statement_key
            .ends_with(match machine.evidence_band {
                SupportedBand::Weak => "weak",
                SupportedBand::Moderate => "moderate",
                SupportedBand::Strong => "strong",
            }),
        "the statement the machine still makes names the machine's band",
    );
}

/// GC-3: a lock with no way out is a lock the user cannot undo.
#[test]
fn releasing_a_tie_hands_the_band_back_to_the_counts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);
    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("correct");

    let release = release_tie(&mut store, relationship_id, NOW).expect("release");
    assert_eq!(release.band, SupportedBand::Strong);
    assert_eq!(release.machine_band, SupportedBand::Strong);
    assert!(!release.overrides_machine());

    // Immediately, not at the next rebuild: the user is looking at the edge.
    for stage in ["straight after the release", "after a rebuild"] {
        let held = strength(&store, relationship_id);
        assert_eq!(held.band, SupportedBand::Strong, "band, {stage}");
        assert_eq!(held.locked_by_user, None, "lock flag, {stage}");
        assert_eq!(held.user_band, None, "user band, {stage}");
        assert!(!held.is_locked_by_user(), "lock, {stage}");
        assert_eq!(
            verdict(&store, relationship_id),
            Some(UserVerdict::Unreviewed),
            "verdict, {stage}",
        );
        soul_graph::rebuild(&mut store).expect("rebuild");
    }
}

/// GC-4: v0.1 cannot invent an edge, so a correction to nothing is refused
/// before anything is written rather than half-applied.
#[test]
fn correcting_an_edge_that_does_not_exist_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    seeded(&mut store);

    let before = (
        store.list_evidence().expect("evidence").len(),
        store.list_relationships().expect("edges").len(),
        store.list_inferences().expect("inferences").len(),
        store.list_audit().expect("audit").len(),
    );

    let refused = correct_tie(&mut store, id("999"), SupportedBand::Weak, NOW);
    assert!(
        matches!(refused, Err(GraphError::Store(StoreError::NotFound { .. }))),
        "expected the store's own NotFound, got {refused:?}",
    );

    let after = (
        store.list_evidence().expect("evidence").len(),
        store.list_relationships().expect("edges").len(),
        store.list_inferences().expect("inferences").len(),
        store.list_audit().expect("audit").len(),
    );
    assert_eq!(before, after, "a refused correction leaves no trace");
}

/// GC-5: the correction is evidence, and evidence has to dereference. AC-06
/// does not make an exception for a row the user wrote.
#[test]
fn the_correction_row_is_evidence_the_edge_cites_and_resolves() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);

    let correction =
        correct_tie(&mut store, relationship_id, SupportedBand::Moderate, NOW).expect("correct");

    // Twice: a rebuild rewrites the edge whole, and the row explaining its band
    // has to come back with it. An edge that kept the user's band and dropped
    // the reason for it would be the black box constraint 10 rules out.
    soul_graph::rebuild(&mut store).expect("rebuild");
    let graph = soul_graph::load(&store).expect("load");
    let edge = graph.edge(relationship_id).expect("edge");
    assert!(
        edge.evidence_ids.contains(&correction.evidence_id),
        "the edge cites the row that changed its band, rebuild or no rebuild",
    );

    let resolved = soul_graph::resolve_evidence(&store, edge).expect("resolve");
    let row = resolved
        .iter()
        .find(|row| row.evidence_id == correction.evidence_id)
        .expect("the correction row resolves");
    assert_eq!(row.kind, EvidenceKind::UserCorrection);
    assert_eq!(row.subject, Subject::Owner);
    assert_eq!(row.strength, SupportedBand::Strong);
    assert_eq!(row.exportable_to_research, Some(false));
    assert_eq!(
        row.source_refs[0]["origin"].as_str(),
        Some(CORRECTION_ORIGIN),
    );
    assert_eq!(row.source_refs[0]["band"].as_str(), Some("moderate"));
    assert_eq!(corrected_relationship(row), Some(relationship_id));

    // Three keys, all of them an identifier or a vocabulary word. There is
    // nothing on this row for a redactor to have to work on.
    let reference = row.source_refs[0].as_object().expect("one object");
    let mut keys: Vec<&str> = reference.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["band", "origin", "relationship_id"]);
    assert_eq!(row.source_refs.len(), 1);

    let release = release_tie(&mut store, relationship_id, NOW).expect("release");
    let released = store.get_evidence(release.evidence_id).expect("row");
    assert_eq!(
        released.source_refs[0]["origin"].as_str(),
        Some(RELEASE_ORIGIN)
    );
    assert_eq!(
        corrected_relationship(&released),
        Some(relationship_id),
        "a release is a correction too, and belongs to the same edge",
    );
}

/// The machine's own reading is support for the machine's own statement, and a
/// verdict overruling it is not.
///
/// The two lists come apart the moment a correction exists, and each one has to
/// hold what actually backs the claim it is attached to. An inference citing the
/// row that rejected it would be circular.
#[test]
fn the_inference_cites_the_observations_and_the_edge_cites_the_correction_too() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);
    let correction =
        correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("correct");
    soul_graph::rebuild(&mut store).expect("rebuild");

    let edge = store.get_relationship(relationship_id).expect("edge");
    let inference = store
        .list_inferences()
        .expect("inferences")
        .into_iter()
        .find(|inference| {
            inference.target["relationship_id"].as_str() == Some(&relationship_id.to_string())
        })
        .expect("tie inference");

    assert!(edge.evidence_ids.contains(&correction.evidence_id));
    assert!(
        !inference.evidence_ids.contains(&correction.evidence_id),
        "the machine's statement does not rest on the user rejecting it",
    );
    assert_eq!(
        edge.evidence_ids.len(),
        inference.evidence_ids.len() + 1,
        "one correction, and otherwise the same rows",
    );
}

/// A rebuilt edge always says what the counts made of it, corrected or not.
///
/// The spec drafted this as "all three lock fields absent while unlocked", and
/// the implementation went the other way: `machine_band` is written on every
/// rebuilt edge, so an interface never has to ask whether the absence of the
/// field means "unlocked" or "written before this existed". The lock is
/// `user_band`, and that one really is absent while unlocked. Pinned here
/// because it is a decision, not an accident.
#[test]
fn an_unlocked_edge_still_records_what_the_counts_say() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);

    let held = strength(&store, relationship_id);
    assert!(!held.is_locked_by_user());
    assert_eq!(held.locked_by_user, None);
    assert_eq!(held.user_band, None);
    assert_eq!(
        held.machine_band,
        Some(held.band),
        "on an unlocked edge the effective band is the machine's, and it says so",
    );
}

/// Two corner calls, decided rather than left to whoever reads the code next.
///
/// Correcting to the band the machine already chose still locks: the user said
/// this band is theirs, and a later import that would have moved it must not.
/// Releasing an edge nobody locked leaves the band alone and is still recorded,
/// because the postcondition the user asked for — the counts decide this edge —
/// is true afterwards either way.
#[test]
fn agreeing_with_the_machine_still_locks_and_releasing_twice_is_harmless() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);
    let machine_band = strength(&store, relationship_id).band;

    let agreement = correct_tie(&mut store, relationship_id, machine_band, NOW).expect("correct");
    assert!(!agreement.overrides_machine());
    assert!(
        strength(&store, relationship_id).is_locked_by_user(),
        "choosing the same word is still choosing it",
    );

    release_tie(&mut store, relationship_id, NOW).expect("first release");
    let second = release_tie(&mut store, relationship_id, NOW).expect("second release");
    let held = strength(&store, relationship_id);
    assert!(!held.is_locked_by_user());
    assert_eq!(held.band, machine_band);
    assert_eq!(second.band, machine_band);
    assert_eq!(
        verdict(&store, relationship_id),
        Some(UserVerdict::Unreviewed),
    );
}

/// GC-10: with nothing new imported, two rebuilds over a locked edge produce
/// the same rows, byte for byte. A lock that drifts on every rebuild would
/// look identical on screen and be a different row underneath.
#[test]
fn rebuilding_twice_over_a_locked_edge_changes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let relationship_id = seeded(&mut store);
    correct_tie(&mut store, relationship_id, SupportedBand::Moderate, NOW).expect("correct");
    soul_graph::rebuild(&mut store).expect("settle");

    let snapshot = |store: &SqlCipherStore| {
        let edge = store.get_relationship(relationship_id).expect("edge");
        let inference = store
            .list_inferences()
            .expect("inferences")
            .into_iter()
            .find(|inference| {
                inference.target["relationship_id"].as_str() == Some(&relationship_id.to_string())
            })
            .expect("tie inference");
        (
            serde_json::to_string(&edge).expect("edge json"),
            serde_json::to_string(&inference).expect("inference json"),
        )
    };

    let before = snapshot(&store);
    soul_graph::rebuild(&mut store).expect("rebuild");
    assert_eq!(before, snapshot(&store));
}
