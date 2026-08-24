//! AC-16: every point in a people summary cites evidence, and none of it
//! reads like a diagnosis.
//!
//! The graph here is derived rather than hand-assembled — contacts and
//! interaction evidence go into a store, `soul_graph::rebuild` produces the
//! edge, and the summary is taken off what came back. That matters because
//! the claim under test is "the summary rests on evidence that resolves", and
//! a `TieEdge` literal with three invented UUIDs in it would satisfy every
//! assertion below while resting on nothing.
//!
//! The negative cases are the useful half: an empty evidence list, a cited
//! row that does not resolve, and a rephrasing that comes back with a
//! forbidden word in it. Each one has to be refused rather than tidied up.

use uuid::Uuid;

use soul_draft::analysis::{self, PersonSummary, SummaryPoint, SummarySource};
use soul_draft::draft::ReplyGenerator;
use soul_draft::error::{DraftError, GenerationRefused};
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{Direction, Venue};
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_policy::redactor::{KnownIdentifiers, RedactedBody, Redactor};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::evidence::SoulEvidence;
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};

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
fn observe(
    store: &mut FakeStore,
    evidence_id: Uuid,
    conversation: &str,
    direction: Direction,
    at: &str,
    venue: Venue,
) {
    let observation = InteractionRef::new(
        id("900"),
        owner(),
        peer(),
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
        .expect("the evidence row is written");
}

/// Six exchanges over three days, both directions, one to one.
fn store_with_one_partner() -> FakeStore {
    let mut store = FakeStore::new();
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("the owner");
    store
        .put_contact(contact(peer(), ContactClass::ThirdParty))
        .expect("the other person");

    let days = ["2026-03-02", "2026-03-05", "2026-03-09"];
    let mut next = 100u32;
    for (index, day) in days.iter().enumerate() {
        for direction in [Direction::Outgoing, Direction::Incoming] {
            next += 1;
            observe(
                &mut store,
                id(&next.to_string()),
                "chat-1",
                direction,
                &format!("{day}T0{index}:15:00Z"),
                Venue::Direct,
            );
        }
    }
    soul_graph::rebuild(&mut store).expect("the graph derives");
    store
}

/// The summary the store's own evidence supports.
fn summarize(store: &FakeStore) -> (PersonSummary, Vec<SoulEvidence>) {
    let graph = soul_graph::load(store).expect("the graph loads");
    let edge = graph
        .edges_for(peer())
        .first()
        .copied()
        .expect("one edge")
        .clone();
    let resolved = soul_graph::resolve_evidence(store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");
    (summary, resolved)
}

/// A generator that answers with whatever it was told to, without a socket.
///
/// Used for the rephrasing path only: what reaches the wire is covered in
/// `tests/wire.rs`, and what is at stake here is whether a returned narrative
/// is allowed to weaken the points.
#[derive(Debug)]
struct Canned {
    answer: String,
    last_body: Option<String>,
}

impl Canned {
    fn saying(text: &str) -> Canned {
        Canned {
            answer: serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": text } }]
            })
            .to_string(),
            last_body: None,
        }
    }

    fn raw(answer: &str) -> Canned {
        Canned {
            answer: answer.to_owned(),
            last_body: None,
        }
    }
}

impl ReplyGenerator for Canned {
    fn generate(&mut self, body: RedactedBody) -> Result<String, GenerationRefused> {
        self.last_body = Some(body.into_string());
        Ok(self.answer.clone())
    }
}

// ---------------------------------------------------------------- AC-16 ---

#[test]
fn every_point_cites_evidence_that_resolves() {
    let store = store_with_one_partner();
    let (summary, resolved) = summarize(&store);

    assert!(!summary.points.is_empty(), "a summary with nothing in it");
    let known: Vec<Uuid> = resolved.iter().map(|row| row.evidence_id).collect();

    for point in &summary.points {
        assert!(
            !point.evidence_ids().is_empty(),
            "`{}` cites nothing",
            point.statement(),
        );
        for cited in point.evidence_ids() {
            assert!(
                known.contains(cited),
                "`{}` cites {cited}, which is not a row that exists",
                point.statement(),
            );
            // Not just "an id in the list" — a row the store hands back.
            store.get_evidence(*cited).expect("the row resolves");
        }
    }
    assert_eq!(summary.source, SummarySource::Counts);
    assert_eq!(summary.notice, WORKING_HYPOTHESIS_NOTICE);
}

#[test]
fn the_summary_says_nothing_a_medical_product_would_say() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);

    for point in &summary.points {
        assert_non_clinical(point.statement()).expect("a point");
    }
    let rendered = analysis::render(&summary).expect("the summary renders");
    assert_non_clinical(&rendered).expect("the whole rendering");
    assert!(
        rendered.contains(WORKING_HYPOTHESIS_NOTICE),
        "the reader is told what this is: {rendered}",
    );
    assert!(
        rendered.contains("依据"),
        "each line names how many rows are behind it: {rendered}",
    );
}

#[test]
fn a_point_with_no_evidence_is_not_a_value_that_can_exist() {
    let refused = SummaryPoint::new("你们最近联系得比以前多", SupportedBand::Weak, Vec::new())
        .expect_err("an unsupported point");
    assert_eq!(refused, DraftError::NoEvidence);
}

#[test]
fn a_point_that_reads_like_a_diagnosis_fails_to_build() {
    let refused = SummaryPoint::new(
        "对方在群里的发言看起来有社交焦虑症",
        SupportedBand::Weak,
        vec![id("101")],
    )
    .expect_err("the vocabulary check runs at construction, not at render time");
    assert!(matches!(refused, DraftError::Clinical(_)));
}

#[test]
fn an_edge_whose_evidence_does_not_resolve_produces_no_summary() {
    let store = store_with_one_partner();
    let graph = soul_graph::load(&store).expect("the graph loads");
    let edge = graph.edges_for(peer())[0].clone();
    let mut resolved = soul_graph::resolve_evidence(&store, &edge).expect("resolved");

    // The caller hands over one row fewer than the edge cites. A summary that
    // accepted this would claim support it cannot show.
    let dropped = resolved.pop().expect("more than one row");
    let refused = analysis::summarize_person(&graph, peer(), &resolved)
        .expect_err("a short evidence list is not a smaller summary");
    assert_eq!(
        refused,
        DraftError::UnresolvedEvidence {
            evidence_id: dropped.evidence_id,
        },
    );
}

#[test]
fn a_person_the_graph_has_never_seen_gets_no_summary() {
    let store = store_with_one_partner();
    let graph = soul_graph::load(&store).expect("the graph loads");
    let stranger = id("777");
    assert_eq!(
        analysis::summarize_person(&graph, stranger, &[]).expect_err("nothing to summarize"),
        DraftError::NoSuchTie {
            contact_id: stranger,
        },
    );
}

#[test]
fn the_counts_in_the_summary_are_the_counts_in_the_graph() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let graph = soul_graph::load(&store).expect("the graph loads");
    let strength = &graph.edges_for(peer())[0].tie_strength;

    assert_eq!(strength.interaction_count, 6);
    assert_eq!(strength.outgoing_count, 3);
    assert_eq!(strength.incoming_count, 3);
    assert_eq!(strength.active_day_count, 3);

    let rendered = analysis::render(&summary).expect("renders");
    assert!(rendered.contains("6 次往来"), "{rendered}");
    assert!(rendered.contains("3 个自然日"), "{rendered}");
    assert!(
        rendered.contains("两边说得差不多"),
        "three each is not a one-sided tie: {rendered}",
    );
}

#[test]
fn the_summary_names_nobody() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let rendered = analysis::render(&summary).expect("renders");

    assert!(
        rendered.contains("这个人"),
        "the subject is referred to, not named: {rendered}",
    );
    // A `PersonNode` has no field holding a name, so there is nothing here for
    // a summary to have copied. This checks the rendering did not go looking.
    let graph = soul_graph::load(&store).expect("loads");
    let node = graph.node(peer()).expect("the node");
    assert!(node.label_ref.is_none());
    assert!(!rendered.contains(&peer().to_string()));
}

// ----------------------------------------------- AC-16, the rephrasing ---

#[test]
fn a_rephrasing_changes_the_wording_and_not_the_evidence() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("你们最近往来比较稳定，多数是一对一说话。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("the rephrasing is read");

    assert_eq!(phrased.source, SummarySource::UserEndpoint);
    assert_eq!(
        phrased.points, summary.points,
        "the points and their evidence are not up for negotiation",
    );
    assert_eq!(phrased.evidence_ids(), summary.evidence_ids());
    assert_eq!(
        phrased.narrative.as_deref(),
        Some("你们最近往来比较稳定，多数是一对一说话。"),
    );
}

#[test]
fn a_rephrasing_that_reads_like_a_diagnosis_is_dropped_and_the_points_stand() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("从往来频率看，对方有明显的焦虑症倾向。");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("a bad narrative is not a failed summary");

    assert_eq!(phrased.narrative, None);
    assert_eq!(
        phrased.source,
        SummarySource::Counts,
        "the summary says where it actually came from",
    );
    assert_eq!(phrased.points, summary.points);
    assert_non_clinical(&analysis::render(&phrased).expect("renders")).expect("still clean");
}

#[test]
fn an_endpoint_that_answers_with_nonsense_leaves_the_summary_intact() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::raw("<html>502 bad gateway</html>");

    let phrased = analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("an unreadable answer is not a failed summary");
    assert_eq!(phrased.narrative, None);
    assert_eq!(phrased.points, summary.points);
}

#[test]
fn what_goes_out_for_rephrasing_is_soul_s_own_counts() {
    let store = store_with_one_partner();
    let (summary, _) = summarize(&store);
    let mut generator = Canned::saying("好的。");

    analysis::phrase_with(
        &summary,
        &Redactor::new(KnownIdentifiers::new()),
        &mut generator,
    )
    .expect("phrased");
    let sent = generator.last_body.expect("a body was built");

    for point in &summary.points {
        assert!(sent.contains(point.statement()), "the statements travel");
    }
    assert!(
        !sent.contains(&peer().to_string()),
        "no contact id goes out"
    );
    assert_non_clinical(&sent).expect("nothing forbidden leaves either");
}
