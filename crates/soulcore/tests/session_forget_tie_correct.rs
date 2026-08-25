//! What 改这一档 does after the user has forgotten the person on the other end.
//!
//! `session_forget_summary.rs` is the same bug on the summary button, and this
//! is the half that was left live. A forget destroys the content keys the
//! person's words were sealed under, tombstones their contact row and demotes
//! the inferences resting on their evidence to `orphaned` — and deliberately
//! leaves the relationship row and the evidence ids alone, because
//! `resolve_evidence` fails rather than returning a short list and
//! `people_view` runs it over every edge. A rebuild then skips the tombstoned
//! peer, so their stale edge stays exactly as the last live rebuild wrote it:
//! in the view, with a band on it and the three band words under it on 人脉图.
//!
//! Pressing one of them used to write through. `soul_graph::correct_tie` read
//! the edge and never the peer's forget state; `set_verdict` finds the tie
//! inference with `list_inferences`, which does not filter on state, and
//! `put_inference` files whatever it is handed as live. So a correction on a
//! tombstone's tie put the forgotten person's inference back to live and added
//! a `UserCorrection` evidence row about them — the forget quietly undone by a
//! button the interface was still offering.
//!
//! Written against a running session for the same reason the summary file is:
//! only the product path can show that the button the shell binds to refuses,
//! and only the real encrypted store can be forgotten from. The forget goes
//! through `soulcore::commands::store::execute_forget` on the session's own
//! store handle, which is the call the product makes.

use std::path::PathBuf;

use uuid::Uuid;

use soul_store_api::forget::ForgetUnit;
use soul_store_api::types::InferenceState;
use soul_store_api::ProfileStore;
use soul_testkit::fixtures;
use soulcore::commands::graph::TieEdgeView;
use soulcore::commands::session::Session;
use soulcore::commands::store as store_commands;

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

/// The tie this person is on, as the view hands it to the screen.
fn tie_touching(session: &Session, contact_id: &str) -> TieEdgeView {
    session
        .people()
        .expect("the store opened")
        .ties
        .into_iter()
        .find(|tie| tie.from_contact_id == contact_id || tie.to_contact_id == contact_id)
        .expect("the person has an edge, or there is no band to press")
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

/// The state of the tie inference about one edge, or `None` where the store
/// holds no inference about it at all.
fn inference_state(session: &Session, relationship_id: &str) -> Option<InferenceState> {
    let handle = session.store().expect("the store opened");
    let store = handle.lock().expect("the store mutex");
    let inference = store
        .list_inferences()
        .expect("inferences")
        .into_iter()
        .find(|inference| {
            inference
                .target
                .get("relationship_id")
                .and_then(serde_json::Value::as_str)
                == Some(relationship_id)
        })?;
    Some(
        store
            .inference_state(inference.inference_id)
            .expect("the state of an inference the store just listed"),
    )
}

/// Any band that is not the one this edge is already in, so a press is a
/// correction rather than a re-statement.
fn other_than(band: &str) -> &'static str {
    match band {
        "strong" => "weak",
        _ => "strong",
    }
}

/// The bug, end to end: the band on a tombstone's stale tie will not move, and
/// the forget stays in force.
///
/// Both writes are pressed, because a release is a write on the same edge and
/// refusing only the correction would leave 按计数重新算 as the way in. What
/// each of them must not have done is checked on the store rather than on the
/// answer: the inference is still `orphaned` and the edge cites no new row.
#[test]
fn a_forgotten_persons_band_cannot_be_corrected_or_released() {
    let (keep, directory) = scratch();
    let mut session = session_with_people(&directory);
    let (them, somebody_else) = two_third_parties(&session);

    let theirs = tie_touching(&session, &them);
    let their_tie = theirs.relationship_id.clone();
    let evidence_before = theirs.evidence.len();
    assert!(evidence_before > 0, "their tie has to rest on something");
    assert_eq!(
        inference_state(&session, &their_tie),
        Some(InferenceState::Live),
        "the fixture has to start live, or the assertions below prove nothing",
    );

    forget_contact(&session, &them);
    assert_eq!(
        inference_state(&session, &their_tie),
        Some(InferenceState::Orphaned),
        "the forget itself has to demote it, or the correction below is not what is under test",
    );

    let corrected = session
        .correct_tie(&their_tie, other_than(&theirs.band))
        .expect_err("a forgotten person's band is not the user's to move any more");
    assert!(!corrected.explanation.is_empty());
    assert!(!corrected.reason_code.is_empty());

    let released = session
        .release_tie(&their_tie)
        .expect_err("a release is a write on the same edge");
    assert!(!released.explanation.is_empty());
    assert!(!released.reason_code.is_empty());

    assert_eq!(
        inference_state(&session, &their_tie),
        Some(InferenceState::Orphaned),
        "the refused press filed the forgotten person's tie as live again, which undoes the forget",
    );

    let after = tie_touching(&session, &them);
    assert_eq!(
        after.evidence.len(),
        evidence_before,
        "a refused correction wrote a UserCorrection row about somebody who was forgotten",
    );
    assert!(
        !after
            .evidence
            .iter()
            .any(|row| row.kind == "user_correction"),
        "the edge cites a correction row: {:?}",
        after.evidence,
    );
    assert!(
        !after.locked_by_user,
        "the band was locked by a refused call"
    );
    assert_eq!(after.band, theirs.band, "the band moved under a refusal");
    assert_eq!(after.user_band, None);

    // The control. Without it, a session that had stopped writing at all would
    // satisfy every assertion above.
    let mine = tie_touching(&session, &somebody_else);
    let chosen = other_than(&mine.band);
    session
        .correct_tie(&mine.relationship_id, chosen)
        .expect("somebody nobody forgot still has a band the user may overrule");
    let corrected = tie_touching(&session, &somebody_else);
    assert_eq!(corrected.band, chosen);
    assert!(corrected.locked_by_user);
    session
        .release_tie(&mine.relationship_id)
        .expect("and a way back out to the counts");

    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    drop(keep);
}

/// The refusal is a value the screen can render, and it names nothing about
/// the person who was forgotten.
///
/// The only identifier in it is the relationship the user pressed on, which is
/// what they handed in. An evidence id or a contact id would be this refusal
/// handing back a pointer into rows the user asked Soul to drop.
#[test]
fn the_refusal_reads_as_a_sentence_and_names_only_what_the_user_pressed() {
    let (keep, directory) = scratch();
    let mut session = session_with_people(&directory);
    let (them, _) = two_third_parties(&session);
    let theirs = tie_touching(&session, &them);
    let cited: Vec<String> = theirs
        .evidence
        .iter()
        .map(|row| row.evidence_id.clone())
        .collect();

    forget_contact(&session, &them);

    let refusal = session
        .correct_tie(&theirs.relationship_id, other_than(&theirs.band))
        .expect_err("a tombstone's band does not move");

    assert!(
        refusal.explanation.contains("已经被遗忘"),
        "the sentence has to say why, in words: {}",
        refusal.explanation,
    );
    assert!(
        refusal.explanation.contains("档位"),
        "and what it is refusing: {}",
        refusal.explanation,
    );
    assert_eq!(
        refusal.reason_code, "ROUTINE",
        "a forgotten peer is an ordinary refusal, not a policy event",
    );
    for evidence_id in &cited {
        assert!(
            !refusal.explanation.contains(evidence_id),
            "the refusal hands back an evidence id from the person who was forgotten: {}",
            refusal.explanation,
        );
    }
    assert!(
        !refusal.explanation.contains(&them),
        "the refusal names the forgotten contact: {}",
        refusal.explanation,
    );
    drop(keep);
}

/// The stale edge is still there, and the graph page still loads.
///
/// This is the constraint the refusal must not be bought with. Deleting the
/// forgotten person's relationship row would be the tidy-looking fix and is
/// the one thing this must not do: `people_view` resolves every id on every
/// edge and fails if one does not lead anywhere, so the buttons would be gone
/// because there is no page — which is not the same thing as a refusal.
#[test]
fn refusing_the_correction_leaves_the_stale_tie_where_the_forget_left_it() {
    let (keep, directory) = scratch();
    let mut session = session_with_people(&directory);
    let (them, _) = two_third_parties(&session);

    forget_contact(&session, &them);
    let before = tie_touching(&session, &them);
    let _ = session.correct_tie(&before.relationship_id, other_than(&before.band));
    let _ = session.release_tie(&before.relationship_id);

    let people = session
        .people()
        .expect("the graph still resolves every row its edges cite");
    let after = people
        .ties
        .iter()
        .find(|tie| tie.relationship_id == before.relationship_id)
        .expect("the stale edge stays; nothing here deletes a derived row");
    assert_eq!(after, &before, "the refused writes moved the edge");
    assert!(
        people
            .people
            .iter()
            .any(|person| person.contact_id == them && person.forgotten),
        "the tombstone has to still be on screen as one",
    );
    drop(keep);
}
