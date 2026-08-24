//! AC-16 on the summary side: every reading is a count somebody could check,
//! and every claim names evidence that resolves.
//!
//! The observations are written the way an importer writes them and the edges
//! come out of a real `soul_graph::rebuild` against a real store, so the
//! numbers in the readings are the numbers in the rows. Hand-built edges would
//! prove the sentence formatter works and nothing else.

use std::collections::BTreeSet;

use uuid::Uuid;

use soul_draft::{summarize, DraftError, ResolvedTie};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::model::TieEdge;
use soul_graph::{Direction, Venue};
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::evidence::SoulEvidence;
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{GraphStore, ProfileStore};
use soul_testkit::fixtures;

const SEED: &str = "wp10 people summary";

/// A UUIDv7-shaped literal, so a failure names the same person every run.
fn id(tail: &str) -> Uuid {
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail:0>3}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn owner() -> Uuid {
    id("001")
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

fn observe(
    store: &mut SqlCipherStore,
    evidence_id: Uuid,
    peer: Uuid,
    conversation: &str,
    direction: Direction,
    at: &str,
    venue: Venue,
) {
    let observation = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        peer,
        conversation_ref("test", conversation),
        direction,
        Timestamp::new(at),
        venue,
    );
    let subject = match direction {
        Direction::Outgoing => Subject::Mixed,
        Direction::Incoming => Subject::ThirdParty,
    };
    store
        .put_evidence(interaction_evidence(evidence_id, subject, &observation))
        .expect("evidence");
}

/// One person talked to on six days, one seen twice in a group.
fn seed(store: &mut SqlCipherStore) -> (Uuid, Uuid) {
    let (close, distant) = (id("002"), id("003"));
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("owner");
    for peer in [close, distant] {
        store
            .put_contact(contact(peer, ContactClass::ThirdParty))
            .expect("peer");
    }

    let mut next = 100u32;
    let mut evidence_id = || {
        next += 1;
        id(&next.to_string())
    };

    for day in 18..24 {
        observe(
            store,
            evidence_id(),
            close,
            "c-01",
            Direction::Outgoing,
            &format!("2026-08-{day}T09:00:00Z"),
            Venue::Direct,
        );
        observe(
            store,
            evidence_id(),
            close,
            "c-01",
            Direction::Incoming,
            &format!("2026-08-{day}T09:05:00Z"),
            Venue::Direct,
        );
    }

    observe(
        store,
        evidence_id(),
        distant,
        "c-05",
        Direction::Outgoing,
        "2026-08-23T20:11:00Z",
        Venue::Group,
    );
    observe(
        store,
        evidence_id(),
        distant,
        "c-05",
        Direction::Incoming,
        "2026-08-23T20:15:00Z",
        Venue::Group,
    );

    soul_graph::rebuild(store).expect("rebuild");
    (close, distant)
}

/// Every edge one person is on, with its evidence already resolved.
fn ties_for(store: &SqlCipherStore, contact_id: Uuid) -> (Vec<TieEdge>, Vec<Vec<SoulEvidence>>) {
    let graph = soul_graph::load(store).expect("load");
    let edges: Vec<TieEdge> = graph.edges_for(contact_id).into_iter().cloned().collect();
    let resolved = edges
        .iter()
        .map(|edge| soul_graph::resolve_evidence(store, edge).expect("resolve"))
        .collect();
    (edges, resolved)
}

fn resolved_ties<'a>(
    edges: &'a [TieEdge],
    evidence: &'a [Vec<SoulEvidence>],
) -> Vec<ResolvedTie<'a>> {
    edges
        .iter()
        .zip(evidence.iter())
        .map(|(edge, rows)| ResolvedTie::new(edge, rows))
        .collect()
}

/// AC-16 forwards: the ids on a claim lead to rows in the store.
#[test]
fn every_claim_has_resolvable_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (close, _) = seed(&mut store);

    let (edges, evidence) = ties_for(&store, close);
    let summary = summarize(close, &resolved_ties(&edges, &evidence)).expect("summary");

    assert!(!summary.claims().is_empty(), "a summary with no claim");
    for claim in summary.claims() {
        assert!(
            !claim.evidence_ids().is_empty(),
            "a claim with no evidence is a claim nothing supports",
        );
        for evidence_id in claim.evidence_ids() {
            store
                .get_evidence(*evidence_id)
                .expect("every cited id resolves to a row");
        }
        assert_eq!(claim.band(), SupportedBand::Strong);
    }
}

/// AC-16 backwards, three ways. None of them is a shorter list.
#[test]
fn a_dangling_evidence_id_is_an_error_not_an_empty_list() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (close, _) = seed(&mut store);
    let (edges, evidence) = ties_for(&store, close);

    // One row missing from what the caller resolved: the ids on the edge are
    // what the claim would cite, so this has to be caught before rendering.
    let mut short = evidence[0].clone();
    let dropped = short.pop().expect("the edge rests on more than one row");
    let ties = vec![ResolvedTie::new(&edges[0], &short)];
    match summarize(close, &ties).expect_err("a missing row is an error") {
        DraftError::DanglingEvidence {
            relationship_id,
            evidence_id,
        } => {
            assert_eq!(relationship_id, edges[0].relationship_id);
            assert_eq!(evidence_id, dropped.evidence_id);
        }
        other => panic!("wrong refusal: {other}"),
    }

    // An edge that cites nothing at all.
    let mut unsupported = edges[0].clone();
    unsupported.evidence_ids.clear();
    let empty: Vec<SoulEvidence> = Vec::new();
    let ties = vec![ResolvedTie::new(&unsupported, &empty)];
    assert!(matches!(
        summarize(close, &ties).expect_err("a claim with no evidence is refused"),
        DraftError::ClaimWithoutEvidence { .. },
    ));

    // Somebody nothing was ever observed about.
    let stranger = id("999");
    assert!(matches!(
        summarize(stranger, &[]).expect_err("an unobserved contact has no summary"),
        DraftError::NothingObserved { contact_id } if contact_id == stranger,
    ));
}

/// The numbers come from the rows: two people the user treated differently
/// read differently. A constant summary fails this one.
#[test]
fn two_contacts_get_different_counts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (close, distant) = seed(&mut store);

    let (close_edges, close_evidence) = ties_for(&store, close);
    let (distant_edges, distant_evidence) = ties_for(&store, distant);

    assert_eq!(close_edges[0].tie_strength.interaction_count, 12);
    assert_eq!(distant_edges[0].tie_strength.interaction_count, 2);
    assert_eq!(close_edges[0].tie_strength.band, SupportedBand::Strong);
    assert_eq!(distant_edges[0].tie_strength.band, SupportedBand::Weak);

    let close_summary =
        summarize(close, &resolved_ties(&close_edges, &close_evidence)).expect("summary");
    let distant_summary =
        summarize(distant, &resolved_ties(&distant_edges, &distant_evidence)).expect("summary");

    let close_text = close_summary.render().expect("render");
    let distant_text = distant_summary.render().expect("render");
    assert_ne!(
        close_text, distant_text,
        "two different histories produced the same summary",
    );
    assert!(close_text.contains("12"), "{close_text}");
    assert!(distant_text.contains('2'), "{distant_text}");
    assert!(
        distant_text.contains("只在群里遇到"),
        "a tie only ever seen in a group says so: {distant_text}",
    );

    // The ids differ too, so the two summaries are not the same rows twice.
    let ids = |summary: &soul_draft::PeopleSummary| -> BTreeSet<Uuid> {
        summary
            .claims()
            .iter()
            .flat_map(|claim| claim.evidence_ids().to_vec())
            .collect()
    };
    assert!(ids(&close_summary).is_disjoint(&ids(&distant_summary)));
}

/// The notice is the last line, and the whole thing is scanned against the
/// denylist rather than against a reviewer's memory of it.
#[test]
fn summary_carries_the_hypothesis_notice_and_is_non_clinical() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());
    let (close, _) = seed(&mut store);
    let (edges, evidence) = ties_for(&store, close);

    let text = summarize(close, &resolved_ties(&edges, &evidence))
        .expect("summary")
        .render()
        .expect("render");

    assert!(
        text.ends_with(WORKING_HYPOTHESIS_NOTICE),
        "the notice is the last thing the reader sees: {text}",
    );
    assert_non_clinical(&text).expect("the summary says nothing clinical");

    let lowered = text.to_lowercase();
    for term in fixtures::denylist_terms().expect("the denylist fixture") {
        assert!(
            !lowered.contains(&term.to_lowercase()),
            "`{term}` reached a summary: {text}",
        );
    }
}
