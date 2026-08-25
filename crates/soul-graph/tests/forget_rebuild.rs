//! A rebuild after a forget must not put the forgotten person back.
//!
//! Forgetting a contact destroys the content keys their words were sealed
//! under, tombstones the contact row and demotes the inferences that rested on
//! their evidence to `orphaned`. What it deliberately leaves alone is the
//! evidence rows and the relationship row: an edge whose ids no longer resolve
//! is one `resolve_evidence` refuses, and refusing there takes the whole graph
//! view down with it.
//!
//! That leaves the observations sitting in the store, and a rebuild runs on
//! every import commit. Grouping them by peer without asking whether the peer
//! is still active writes their edge again and hands `put_inference` a fresh
//! tie inference — which is filed `live` unconditionally — so the second import
//! after a forget quietly undoes it. This is that path, start to finish, over
//! the real encrypted store.

use std::collections::BTreeSet;

use uuid::Uuid;

use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_schema::common::{SchemaVersion, SealedSubject, Subject, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::{ForgetOps, ForgetUnit};
use soul_store_api::types::{InferenceState, SealRequest};
use soul_store_api::{BlobStore, GraphStore, ProfileStore};

const SEED: &str = "p1-3 rebuild after a contact forget";

/// A UUIDv7-shaped literal, so a failure names the same person every run.
fn id(tail: &str) -> Uuid {
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail:0>3}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn owner() -> Uuid {
    id("001")
}

/// The person who gets forgotten, and the one nobody touches.
fn gone() -> Uuid {
    id("002")
}

fn kept() -> Uuid {
    id("003")
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

/// One observation, written the way an importer writes it.
fn observe(store: &mut SqlCipherStore, evidence_id: Uuid, peer: Uuid, at: &str) {
    let direction = match evidence_id.as_u128() % 2 {
        0 => Direction::Outgoing,
        _ => Direction::Incoming,
    };
    let observation = InteractionRef::new(
        Uuid::now_v7(),
        owner(),
        peer,
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

/// Anchor a content key to a contact's own row, which is what an import of a
/// file carrying no display names leaves behind — and what makes forgetting
/// that contact reach anything at all.
fn anchor_key(store: &mut SqlCipherStore, content_key_id: Uuid, contact_id: Uuid) {
    store
        .seal(
            SealRequest::new(
                content_key_id,
                contact_id,
                "content_key_anchor",
                SealedSubject::ThirdParty,
                contact_id.to_string().into_bytes(),
            )
            .with_placeholder("[第三人内容已占位]"),
        )
        .expect("anchor their key to their row");
}

/// Two partners, three exchanges each, each owning one content key.
fn seed(store: &mut SqlCipherStore) {
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("owner");
    for (peer, key) in [(gone(), id("060")), (kept(), id("061"))] {
        store
            .put_contact(contact(peer, ContactClass::ThirdParty))
            .expect("peer");
        anchor_key(store, key, peer);
    }

    let mut next = 100u32;
    let mut evidence_id = || {
        next += 1;
        id(&next.to_string())
    };
    for day in 18..21 {
        observe(
            store,
            evidence_id(),
            gone(),
            &format!("2026-08-{day}T09:00:00Z"),
        );
        observe(
            store,
            evidence_id(),
            kept(),
            &format!("2026-08-{day}T11:00:00Z"),
        );
    }
}

/// The edge to `peer`, as the store holds it.
fn edge_to(store: &SqlCipherStore, peer: Uuid) -> soul_schema::relationship::SoulRelationship {
    store
        .list_relationships()
        .expect("relationships")
        .into_iter()
        .find(|edge| edge.from_contact_id == peer || edge.to_contact_id == peer)
        .expect("an edge to that person")
}

/// The tie inference about `relationship_id`, as the store holds it.
fn tie_inference(
    store: &SqlCipherStore,
    relationship_id: Uuid,
) -> soul_schema::inference::SoulInference {
    store
        .list_inferences()
        .expect("inferences")
        .into_iter()
        .find(|inference| {
            inference
                .target
                .get("relationship_id")
                .and_then(serde_json::Value::as_str)
                == Some(relationship_id.to_string().as_str())
        })
        .expect("a tie inference about that edge")
}

/// The whole of it: forget one of two partners, rebuild, and find the forget
/// still in force.
#[test]
fn a_rebuild_after_a_forget_writes_no_live_tie_for_the_person_who_was_forgotten() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store =
        SqlCipherStore::open(dir.path().join("soul.db"), &TestKeyProvider::from_seed(SEED))
            .expect("open");
    seed(&mut store);

    let first = soul_graph::rebuild(&mut store).expect("the first rebuild");
    assert_eq!(first.edges_written.len(), 2, "one edge per partner");
    assert_eq!(first.peers_unresolved, 0, "both partners are known");

    let their_edge = edge_to(&store, gone());
    let their_evidence: BTreeSet<Uuid> = their_edge.evidence_ids.iter().copied().collect();
    let their_inference = tie_inference(&store, their_edge.relationship_id).inference_id;
    let kept_edge = edge_to(&store, kept());
    assert_eq!(
        store.inference_state(their_inference).expect("state"),
        InferenceState::Live,
        "the fixture has to start live, or the assertion below proves nothing",
    );

    let receipt = store
        .execute_forget(ForgetUnit::Contact(gone()))
        .expect("forget them");
    assert_eq!(receipt.impact.contacts_affected, 1);
    assert_eq!(
        receipt.impact.inferences_orphaned, 1,
        "the tie inference rested on their evidence, so the forget demoted it",
    );
    assert_eq!(
        store.get_contact(gone()).expect("row").forget_state,
        ForgetState::Forgotten,
    );

    assert_eq!(
        store.inference_state(their_inference).expect("state"),
        InferenceState::Orphaned,
        "the forget itself has to demote it, or the rebuild below is not what is under test",
    );

    // The import commit that comes next. Nothing new was written; this is the
    // rebuild alone.
    let again = soul_graph::rebuild(&mut store).expect("the second rebuild");

    assert_eq!(
        store.inference_state(their_inference).expect("state"),
        InferenceState::Orphaned,
        "the rebuild filed the forgotten person's tie as live again, which undoes the forget",
    );
    assert_eq!(
        again.peers_unresolved, 3,
        "a tombstoned peer's three observations have to be reported as unresolved rather \
         than derived from",
    );
    assert!(
        !again.edges_written.contains(&their_edge.relationship_id),
        "the forgotten person's edge was written again",
    );
    assert_eq!(
        again.edges_written,
        vec![kept_edge.relationship_id],
        "the person nobody forgot still gets their edge rebuilt",
    );

    // Nothing written by this rebuild may cite what the forgotten person's
    // observations say — a fresh edge carrying their evidence ids is the same
    // resurrection wearing another id.
    for relationship_id in &again.edges_written {
        let edge = store.get_relationship(*relationship_id).expect("edge");
        for cited in &edge.evidence_ids {
            assert!(
                !their_evidence.contains(cited),
                "edge {relationship_id} cites evidence behind a forgotten person",
            );
        }
    }
    for inference_id in &again.inferences_written {
        let inference = store.get_inference(*inference_id).expect("inference");
        for cited in &inference.evidence_ids {
            assert!(
                !their_evidence.contains(cited),
                "inference {inference_id} cites evidence behind a forgotten person",
            );
        }
    }
}

/// The stale edge is left where it is, and the graph still resolves.
///
/// Deleting the forgotten person's relationship row would be the tidy-looking
/// fix and is the one thing this must not do: `resolve_evidence` fails rather
/// than returning a short list, and `soulcore::commands::graph::people_view`
/// runs it over every edge — so a graph missing rows an edge cites stops
/// rendering at all. Skipping the peer during a rebuild leaves the row exactly
/// as the last live rebuild wrote it, which still resolves.
#[test]
fn the_forgotten_persons_stale_edge_stays_and_the_graph_still_resolves() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store =
        SqlCipherStore::open(dir.path().join("soul.db"), &TestKeyProvider::from_seed(SEED))
            .expect("open");
    seed(&mut store);
    soul_graph::rebuild(&mut store).expect("the first rebuild");

    let before = edge_to(&store, gone());
    store
        .execute_forget(ForgetUnit::Contact(gone()))
        .expect("forget them");
    soul_graph::rebuild(&mut store).expect("the second rebuild");

    let after = edge_to(&store, gone());
    assert_eq!(
        after, before,
        "the stale edge is left as it was; nothing here deletes derived rows",
    );

    let graph = soul_graph::load(&store).expect("the graph loads");
    for edge in &graph.edges {
        soul_graph::resolve_evidence(&store, edge)
            .expect("every edge the view draws still resolves what it cites");
    }
    assert_eq!(
        graph
            .node(gone())
            .expect("the tombstone is still a node")
            .forget_state,
        ForgetState::Forgotten,
        "the state that says they are gone is on the node, which is where readers look",
    );
}
