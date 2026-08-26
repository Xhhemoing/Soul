//! What 看这个人的摘要 does after the user has forgotten that person.
//!
//! A forget is not a delete of everything that mentions somebody. It destroys
//! the content keys their words were sealed under, tombstones their contact
//! row and demotes the inferences that rested on their evidence — and it
//! deliberately leaves the evidence rows and the relationship row where they
//! are, because `soul_graph::resolve_evidence` fails rather than returning a
//! short list and `people_view` runs it over every edge. Deleting the rows
//! would take the whole graph page down with them.
//!
//! So everything the summary is built out of survives a forget: the edge, its
//! counts, and the ids of the rows behind them. The only record that the
//! person is gone is `PersonNode::forget_state`, and until this the summary
//! path never read it. A user who forgot somebody could press the button on
//! their tombstone and get the counts back, sentence by sentence, each one
//! citing evidence — and with an endpoint configured, those counts left the
//! machine on the way.
//!
//! Written against a running session for the same reason `session_summary.rs`
//! is: only the product path can show that nothing reached the endpoint, and
//! only the real encrypted store can be forgotten from. The forget itself goes
//! through `soulcore::commands::store::execute_forget` on the session's own
//! store handle, which is the call the product makes.

use std::path::PathBuf;

use uuid::Uuid;

use soul_store_api::forget::ForgetUnit;
use soul_testkit::fixtures;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::draft::PersonSummaryView;
use soulcore::commands::session::Session;
use soulcore::commands::store as store_commands;

/// An answer that would be shown if one were ever asked for. It never is.
const ON_TOPIC: &str = "你们最近往来比较稳定，多数时候是一对一说话。";

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

/// A session with the Telegram export committed, which also rebuilds the
/// graph.
fn session_with_people(directory: &PathBuf) -> Session {
    let mut session = Session::open(directory);
    session
        .commit_telegram(
            &fixtures::read_text("import/telegram/result_basic.json").expect("fixture"),
        )
        .expect("the export commits");
    session
}

/// Two people this graph has ties to, busiest first. The first one gets
/// forgotten; the second is the control nobody touches.
fn two_third_parties(session: &Session) -> (String, String) {
    let mut people: Vec<_> = session
        .people()
        .expect("the store opened")
        .people
        .into_iter()
        .filter(|person| !person.is_you && person.tie_count > 0)
        .collect();
    people.sort_by_key(|person| std::cmp::Reverse(person.interaction_count));
    assert!(
        people.len() >= 2,
        "the export has to name two people, or there is no control in this test",
    );
    (people[0].contact_id.clone(), people[1].contact_id.clone())
}

/// Every evidence row the graph cites on a tie touching this person.
fn evidence_behind(session: &Session, contact_id: &str) -> Vec<String> {
    session
        .people()
        .expect("the store opened")
        .ties
        .into_iter()
        .filter(|tie| tie.from_contact_id == contact_id || tie.to_contact_id == contact_id)
        .flat_map(|tie| tie.evidence.into_iter().map(|row| row.evidence_id))
        .collect()
}

/// Forget one contact, through the call the product makes, on the store the
/// session already has open.
fn forget_contact(session: &Session, contact_id: &str) {
    let handle = session.store().expect("the store opened");
    let mut store = handle.lock().expect("the store mutex");
    let receipt = store_commands::execute_forget(
        &mut store,
        ForgetUnit::Contact(contact_id.parse::<Uuid>().expect("a contact id")),
    )
    .expect("the forget runs");
    assert_eq!(
        receipt.impact.contacts_affected, 1,
        "the forget has to reach the contact row, or nothing below is under test",
    );
}

/// Every UUID-shaped token in a string.
///
/// Written by hand rather than with a pattern crate: what matters is that a
/// refusal shown to the user does not hand back the identifiers of the rows
/// that were behind the person they just forgot.
fn uuids_in(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_ascii_hexdigit() && character != '-')
        .filter(|token| token.parse::<Uuid>().is_ok())
        .map(str::to_owned)
        .collect()
}

/// The bug, end to end: a tombstone has no summary, and nothing goes out
/// asking for one.
///
/// The endpoint is configured before the refused call and asked for a summary
/// of somebody else afterwards, so `request_count` says something: zero is
/// what the refusal cost, and the one that follows shows the endpoint was
/// reachable the whole time.
#[test]
fn a_forgotten_person_gets_no_summary_and_nothing_is_asked_of_the_endpoint() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    endpoint.set_reply(ON_TOPIC);
    let mut session = session_with_people(&directory);
    let (them, somebody_else) = two_third_parties(&session);
    let cited = evidence_behind(&session, &them);
    assert!(!cited.is_empty(), "their tie has to rest on something");

    forget_contact(&session, &them);
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");

    let refusal = session
        .person_summary(&them)
        .expect_err("a forgotten person has nothing citable left");
    assert_eq!(
        endpoint.request_count(),
        0,
        "the counts behind a forgotten person were sent to the endpoint",
    );

    assert!(!refusal.explanation.is_empty());
    assert!(!refusal.reason_code.is_empty());
    for evidence_id in &cited {
        assert!(
            !refusal.explanation.contains(evidence_id),
            "the refusal hands back an evidence id from the person who was forgotten: {}",
            refusal.explanation,
        );
    }
    assert!(
        uuids_in(&refusal.explanation)
            .iter()
            .all(|found| *found == them),
        "the refusal names an identifier that is not the contact asked about: {}",
        refusal.explanation,
    );

    // The control: the same endpoint, the same session, somebody still active.
    // Without this, a broken endpoint would satisfy the assertion above.
    session
        .person_summary(&somebody_else)
        .expect("somebody nobody forgot still has a summary");
    assert_eq!(
        endpoint.request_count(),
        1,
        "the endpoint was reachable all along, so the zero above was the refusal",
    );

    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    drop(keep);
}

/// Forgetting one person changes nothing about anybody else's summary.
///
/// No endpoint here on purpose: the counts path is the one every installation
/// without a key is on, and the comparison is field for field, so a refusal
/// that over-reached — or a rebuild that quietly recounted somebody — shows up
/// as a difference rather than as a missing sentence nobody notices.
#[test]
fn forgetting_one_person_leaves_everybody_elses_summary_exactly_where_it_was() {
    let (keep, directory) = scratch();
    let mut session = session_with_people(&directory);
    let (them, somebody_else) = two_third_parties(&session);

    let before: PersonSummaryView = session
        .person_summary(&somebody_else)
        .expect("the counts, with nothing configured");
    assert_eq!(before.source, "counts");
    assert!(!before.points.is_empty());

    forget_contact(&session, &them);

    let after = session
        .person_summary(&somebody_else)
        .expect("forgetting one person does not touch another's summary");
    assert_eq!(
        after, before,
        "somebody else's summary moved under a forget"
    );
    drop(keep);
}

/// The graph page still renders after a forget, and says who is a tombstone.
///
/// This is the constraint the refusal must not be bought with. `people_view`
/// resolves every id on every edge and fails if one does not lead anywhere, so
/// a fix that deleted the forgotten person's evidence or their relationship
/// row would leave the whole page unable to load — the button would be gone
/// because there is no page, which is not the same thing as a refusal.
#[test]
fn the_graph_still_loads_after_a_forget_and_marks_the_tombstone() {
    let (keep, directory) = scratch();
    let session = session_with_people(&directory);
    let (them, _) = two_third_parties(&session);
    let ties_before = session.people().expect("the store opened").ties.len();

    forget_contact(&session, &them);

    let people = session
        .people()
        .expect("the graph still resolves every row its edges cite");
    assert_eq!(
        people.ties.len(),
        ties_before,
        "the stale edge stays; nothing here deletes a derived row",
    );
    for tie in &people.ties {
        assert!(
            !tie.evidence.is_empty(),
            "an edge the page draws has to still resolve what it cites",
        );
    }
    assert!(
        people
            .people
            .iter()
            .any(|person| person.contact_id == them && person.forgotten),
        "the tombstone has to be on screen as one",
    );
    drop(keep);
}
