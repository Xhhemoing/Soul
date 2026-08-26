//! AD-9 and AD-13 where a user meets them: 「看这个人的摘要」.
//!
//! `soul-draft`'s `projection_sentences.rs` owns the arithmetic — the closed
//! intervals, the group case, the evidence one bullet may cite, the screening
//! every template goes through. All of it runs on a hand-built store, and none
//! of it can say whether the forecast survives the trip through an import, a
//! rebuild, the graph the 人脉图 reads and the summary command the shell binds
//! to. That trip is what is checked here, and only that.
//!
//! Three states of one edge, all of them reached the way a user reaches them:
//!
//! * a live Strong tie, which is told which day it comes down and which day it
//!   hits the floor;
//! * the same edge after the user has ruled on the band, which says nothing
//!   about the clock — the band is their verdict now, and the machine's clock
//!   does not move it (GC-9a's argument, AD-13's fourth silence);
//! * a Weak tie in the same store, which has no band below it to project.
//!
//! Then the control: release the lock and the sentences come back, so the
//! silence in the middle is the lock rather than something the session lost
//! along the way.
//!
//! The store is seeded with an inline `soul-import-v1` export rather than a
//! fixture, for the reason `session_crash.rs` gives: the counts *are* the
//! assertion here. Twelve one-to-one exchanges spread over six days is what
//! makes the first edge Strong under T4D, and a fixture that grew a line would
//! quietly change which band this file is about.

use std::path::PathBuf;

use soul_draft::projection::WORKING_HYPOTHESIS_CLOSER;
use soulcore::commands::draft::PersonSummaryView;
use soulcore::commands::graph::TieEdgeView;
use soulcore::commands::session::Session;

/// The two clauses `COPY_ZH.md` §6 gives the two steps of the clock. Checked
/// as clauses rather than as dates because the dates are the frozen crate's
/// arithmetic and are pinned there; what this file is asking is whether the
/// step and the floor reach the screen at all.
const STEP_CLAUSE: &str = "从那天起这一档会从「强」降到「中等」";
const FLOOR_CLAUSE: &str = "从那天起这一档会算「弱」";

/// Twelve one-to-one exchanges with `u-qing`, six out and six in, one pair on
/// each of six consecutive days — Strong on the counts, and not yet demoted.
/// Plus two with `u-tan`, which is under the Moderate bar and so Weak.
///
/// The newest row in the file is `u-qing`'s, which is what fixes the store-wide
/// `as_of` the rebuild scores against: the Strong edge is at zero silent days
/// and both steps of its clock are still ahead of it. Nothing here reads a wall
/// clock, so this file says the same thing next year.
const EXPORT: &str = concat!(
    r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#,
    "\n",
    r#"{"type":"message","id":"p-01","occurred_at":"2026-08-18T10:05:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"下周搬家的车我订好了"}"#,
    "\n",
    r#"{"type":"message","id":"p-02","occurred_at":"2026-08-18T10:12:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-qing","text":"几点的，我那天有空"}"#,
    "\n",
    r#"{"type":"message","id":"p-03","occurred_at":"2026-08-19T09:30:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"上午九点，先搬书"}"#,
    "\n",
    r#"{"type":"message","id":"p-04","occurred_at":"2026-08-19T09:41:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-qing","text":"书我来搬，你别抬重的"}"#,
    "\n",
    r#"{"type":"message","id":"p-05","occurred_at":"2026-08-20T12:15:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"纸箱不够，我再买十个"}"#,
    "\n",
    r#"{"type":"message","id":"p-06","occurred_at":"2026-08-20T12:26:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-qing","text":"我家里还有五个，给你带过去"}"#,
    "\n",
    r#"{"type":"message","id":"p-07","occurred_at":"2026-08-21T18:40:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"厨房那些锅先不打包"}"#,
    "\n",
    r#"{"type":"message","id":"p-08","occurred_at":"2026-08-21T18:52:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-qing","text":"行，最后一天再收"}"#,
    "\n",
    r#"{"type":"message","id":"p-09","occurred_at":"2026-08-22T08:20:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"新住处的钥匙拿到了"}"#,
    "\n",
    r#"{"type":"message","id":"p-10","occurred_at":"2026-08-22T08:33:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-qing","text":"那晚上过去看看"}"#,
    "\n",
    r#"{"type":"message","id":"p-11","occurred_at":"2026-08-23T21:02:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"都搬完了，谢谢你这几天"}"#,
    "\n",
    r#"{"type":"message","id":"p-12","occurred_at":"2026-08-23T21:14:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-qing","text":"客气什么，明天好好睡一觉"}"#,
    "\n",
    r#"{"type":"message","id":"q-01","occurred_at":"2026-08-23T09:00:00Z","sender_scope":"self","conversation_id":"c-02","sender_id":"u-self","text":"表格我填完发给你了"}"#,
    "\n",
    r#"{"type":"message","id":"q-02","occurred_at":"2026-08-23T09:07:00Z","sender_scope":"third_party","conversation_id":"c-02","sender_id":"u-tan","text":"收到"}"#,
    "\n",
);

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

/// A session with the export above committed, which is also the rebuild.
fn imported(directory: &PathBuf) -> Session {
    let mut session = Session::open(directory);
    session
        .commit_soul_import_v1(EXPORT)
        .expect("the export commits");
    session
}

/// The two people the export puts in the graph: the one with twelve exchanges
/// and the one with two, in that order.
fn the_busy_one_and_the_quiet_one(session: &Session) -> (String, String) {
    let mut peers: Vec<_> = session
        .people()
        .expect("the graph reads back")
        .people
        .into_iter()
        .filter(|person| !person.is_you && person.tie_count > 0)
        .collect();
    assert_eq!(peers.len(), 2, "the export has two peers in it: {peers:?}");
    peers.sort_by_key(|person| person.interaction_count);
    let quiet = peers.remove(0);
    let busy = peers.remove(0);
    assert_eq!(busy.interaction_count, 12);
    assert_eq!(quiet.interaction_count, 2);
    (busy.contact_id, quiet.contact_id)
}

/// The edge between the user and this person, with the sanity checks the rest
/// of the file rests on: the counts made it Strong, and nobody has ruled on it.
fn a_live_strong_tie(session: &Session, contact_id: &str) -> TieEdgeView {
    let tie = session
        .people()
        .expect("the graph reads back")
        .ties
        .into_iter()
        .find(|tie| tie.to_contact_id == contact_id || tie.from_contact_id == contact_id)
        .expect("the peer has an edge");
    assert_eq!(
        tie.band, "strong",
        "the fixture must be Strong before anything is projected off it",
    );
    assert!(!tie.locked_by_user, "nobody has corrected this edge yet");
    tie
}

/// The statements that carry the §6 closer, which is what a projected sentence
/// is and what an A2 count bullet is not.
fn projected(summary: &PersonSummaryView) -> Vec<&str> {
    summary
        .points
        .iter()
        .map(|point| point.statement.as_str())
        .filter(|statement| statement.ends_with(WORKING_HYPOTHESIS_CLOSER))
        .collect()
}

/// The point holding `clause`, or a failure naming everything that was there.
fn point_saying<'a>(summary: &'a PersonSummaryView, clause: &str) -> &'a str {
    summary
        .points
        .iter()
        .map(|point| point.statement.as_str())
        .find(|statement| statement.contains(clause))
        .unwrap_or_else(|| {
            panic!(
                "no point says `{clause}`: {:?}",
                summary
                    .points
                    .iter()
                    .map(|point| point.statement.as_str())
                    .collect::<Vec<_>>(),
            )
        })
}

/// AD-9 and AD-13 arriving on the screen: a Strong tie the rule has not
/// demoted is told which day it comes down and which day it hits the floor.
///
/// Both sentences carry the §6 closer, both rest on rows, and both are in the
/// rendered text — a point the renderer dropped would be a forecast the user
/// never reads.
#[test]
fn a_live_strong_tie_is_told_both_days_its_band_is_due_to_move() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);
    let (busy, _) = the_busy_one_and_the_quiet_one(&session);
    a_live_strong_tie(&session, &busy);

    let summary = session.person_summary(&busy).expect("a summary");
    assert_eq!(summary.source, "counts", "no endpoint is configured");

    for clause in [STEP_CLAUSE, FLOOR_CLAUSE] {
        let statement = point_saying(&summary, clause);
        assert!(
            statement.ends_with(WORKING_HYPOTHESIS_CLOSER),
            "COPY_ZH §0.6 asks every explanation to close with it: {statement}",
        );
        assert!(
            summary.text.contains(clause),
            "the forecast never reached the rendering: {}",
            summary.text,
        );
    }

    for statement in projected(&summary) {
        let point = summary
            .points
            .iter()
            .find(|point| point.statement == statement)
            .expect("the statement came from a point");
        assert!(
            !point.evidence_ids.is_empty(),
            "AC-16: a forecast with nothing behind it: {statement}",
        );
    }

    drop(keep);
}

/// The band the user set is not on the machine's clock.
///
/// `moderate` rather than the band the counts gave, so the correction is a
/// correction: an unlocked Moderate edge at this silence would project its own
/// step down, which means the silence that follows is the lock and not the
/// band. Nothing is re-imported in between — the rebuild that derived the edge
/// ran once, and what changed is the user's verdict on it.
#[test]
fn a_band_the_user_ruled_on_stops_being_forecast() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);
    let (busy, _) = the_busy_one_and_the_quiet_one(&session);
    let tie = a_live_strong_tie(&session, &busy);

    assert!(
        !projected(&session.person_summary(&busy).expect("a summary")).is_empty(),
        "the control half of this test measured nothing",
    );

    session
        .correct_tie(&tie.relationship_id, "moderate")
        .expect("the user read the tie and said the band is wrong");

    let summary = session.person_summary(&busy).expect("a summary either way");
    assert!(
        projected(&summary).is_empty(),
        "the machine went on forecasting a band the user set: {:?}",
        summary
            .points
            .iter()
            .map(|point| point.statement.as_str())
            .collect::<Vec<_>>(),
    );
    assert!(
        !summary.points.is_empty(),
        "dropping the forecast took the counts with it",
    );
    for point in &summary.points {
        assert!(!point.evidence_ids.is_empty(), "`{}`", point.statement);
    }

    drop(keep);
}

/// The control for the silence above: hand the band back to the counts and the
/// forecast returns, on the same store and the same edge.
#[test]
fn releasing_the_lock_puts_the_forecast_back() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);
    let (busy, _) = the_busy_one_and_the_quiet_one(&session);
    let tie = a_live_strong_tie(&session, &busy);

    session
        .correct_tie(&tie.relationship_id, "moderate")
        .expect("correct");
    assert!(projected(&session.person_summary(&busy).expect("a summary")).is_empty());

    session
        .release_tie(&tie.relationship_id)
        .expect("the user asked for the counts to speak again");

    let summary = session.person_summary(&busy).expect("a summary");
    assert!(
        summary.text.contains(STEP_CLAUSE) && summary.text.contains(FLOOR_CLAUSE),
        "the counts speak again but the clock does not: {}",
        summary.text,
    );
    for statement in projected(&summary) {
        assert!(
            statement.ends_with(WORKING_HYPOTHESIS_CLOSER),
            "{statement}"
        );
    }

    drop(keep);
}

/// A Weak tie has no band below it, so there is no next step to name.
///
/// The same store and the same rebuild as the Strong edge above, which is what
/// makes this a statement about the band rather than about the session: one
/// summary carries the clock and the other does not.
#[test]
fn a_weak_tie_in_the_same_store_is_told_nothing_about_a_next_step() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);
    let (_, quiet) = the_busy_one_and_the_quiet_one(&session);

    let tie = session
        .people()
        .expect("the graph reads back")
        .ties
        .into_iter()
        .find(|tie| tie.to_contact_id == quiet || tie.from_contact_id == quiet)
        .expect("the quiet peer has an edge");
    assert_eq!(tie.band, "weak");
    assert!(
        !tie.locked_by_user,
        "the silence must be the band, not a lock"
    );

    let summary = session.person_summary(&quiet).expect("a summary");
    assert!(
        !summary.points.is_empty(),
        "a Weak tie still has counts to show",
    );
    assert!(
        projected(&summary).is_empty(),
        "a Weak tie was promised a band below it: {:?}",
        summary
            .points
            .iter()
            .map(|point| point.statement.as_str())
            .collect::<Vec<_>>(),
    );

    drop(keep);
}
