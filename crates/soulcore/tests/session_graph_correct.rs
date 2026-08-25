//! Correcting a tie through the session the desktop shell actually holds.
//!
//! `soul-graph` proves what a correction does to a store and
//! `graph_correction_commands.rs` proves what the command layer writes to the
//! chain. Neither of them is the layer the 人脉图 binds to: until the session
//! had these two methods, the band on an edge was something only a Rust test
//! could move — an installed Soul showed the user a working hypothesis with no
//! way to overrule it, which is the one thing non-negotiable constraint 10
//! rules out.
//!
//! So what is checked here is the product path: the band the screen is handed
//! after a correction, the machine's own reading kept beside it, the lock
//! surviving the rebuild a second import runs, the way back out to the counts,
//! and the two words that are refused before anything reaches the store.

use std::path::PathBuf;

use soul_testkit::fixtures;
use soulcore::commands::graph::TieEdgeView;
use soulcore::commands::session::Session;

/// Four people and three ties, which is enough to have a tie to correct and
/// two that must not move while it is corrected.
const IMPORT: &str = "import/soul-import-v1/three_partners.jsonl";

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

/// A session with the fixture imported, and the graph that import derived.
fn imported(directory: &PathBuf) -> Session {
    let mut session = Session::open(directory);
    let text = fixtures::read_text(IMPORT).expect("the fixture is on disk");
    session
        .commit_soul_import_v1(&text)
        .expect("a valid export commits");
    session
}

fn tie(session: &Session, relationship_id: &str) -> TieEdgeView {
    session
        .people()
        .expect("the graph reads back")
        .ties
        .into_iter()
        .find(|tie| tie.relationship_id == relationship_id)
        .expect("the tie is still in the view")
}

/// The first tie the view lists, and the band it arrived with.
fn a_tie(session: &Session) -> TieEdgeView {
    session
        .people()
        .expect("the graph reads back")
        .ties
        .into_iter()
        .next()
        .expect("the fixture has ties in it")
}

/// Any band that is not the one this edge is already in, so a correction is a
/// correction rather than a re-statement.
fn other_than(band: &str) -> &'static str {
    match band {
        "strong" => "weak",
        _ => "strong",
    }
}

/// AC-07 on the graph side, through the session: the band the user chose is
/// the band the screen is handed, the lock is on it, and what the counts say
/// is kept beside it rather than dropped.
#[test]
fn a_correction_pins_the_band_and_keeps_the_machines_reading_beside_it() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);

    let before = a_tie(&session);
    assert!(!before.locked_by_user, "nobody has corrected this edge yet");
    assert_eq!(before.user_band, None);
    let chosen = other_than(&before.band);

    let graph = session
        .correct_tie(&before.relationship_id, chosen)
        .expect("the user read the tie and said the band is wrong");

    let after = graph
        .ties
        .iter()
        .find(|tie| tie.relationship_id == before.relationship_id)
        .expect("the corrected tie is in the answer");
    assert_eq!(after.band, chosen, "the effective band is the user's");
    assert!(after.locked_by_user);
    assert_eq!(after.user_band.as_deref(), Some(chosen));
    assert_eq!(
        after.machine_band.as_deref(),
        Some(before.band.as_str()),
        "what the counts said is kept so the screen can draw the disagreement",
    );

    // What a correction fixes is the one summary word. The observations the
    // user could check by counting messages are exactly as they were.
    assert_eq!(after.interaction_count, before.interaction_count);
    assert_eq!(after.outgoing_count, before.outgoing_count);
    assert_eq!(after.incoming_count, before.incoming_count);
    assert_eq!(after.active_day_count, before.active_day_count);

    // The row that changed the band is evidence like any other, and the view
    // resolves it with the rest — an edge citing a band nothing supports is
    // the case AC-06 exists to prevent.
    assert!(
        after
            .evidence
            .iter()
            .any(|row| row.kind == "user_correction"),
        "the correction is not among the rows this edge cites: {:?}",
        after.evidence,
    );

    // Nobody else's edge moved.
    for other in graph
        .ties
        .iter()
        .filter(|tie| tie.relationship_id != before.relationship_id)
    {
        assert!(!other.locked_by_user, "a second edge was locked as well");
        assert_eq!(other.user_band, None);
    }

    // AC-23: the chain names the edge and the row, and carries no band word —
    // a vocabulary token in the audit log would be a fact about the person on
    // the other end of that edge.
    let chain = session.audit().expect("the chain reads back");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    let recorded = chain
        .entries
        .iter()
        .find(|entry| entry.action == "profile.correct")
        .unwrap_or_else(|| {
            panic!(
                "the correction never reached the chain: {:?}",
                chain.entries
            )
        });
    assert_eq!(recorded.decision, "allowed");
    assert!(
        recorded.subject_refs.contains(&before.relationship_id),
        "the entry does not name the edge it is about: {recorded:?}",
    );
    assert!(recorded.follows_previous);
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    assert!(!played.contains(chosen), "the chain carries a band word");

    drop(keep);
}

/// The promise a lock makes: a later rebuild recounts and leaves the band
/// where the user put it.
///
/// The rebuild here is the real one — `Session::commit_import` runs it after
/// every import, so importing the same export again is how a user reaches it
/// without a Rust test calling `soul_graph::rebuild` by hand. The counts move,
/// because the events are written a second time; the band does not.
#[test]
fn a_rebuild_recounts_the_edge_and_leaves_the_corrected_band_alone() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);

    let before = a_tie(&session);
    let chosen = other_than(&before.band);
    session
        .correct_tie(&before.relationship_id, chosen)
        .expect("correct");

    let text = fixtures::read_text(IMPORT).expect("the fixture is on disk");
    session
        .commit_soul_import_v1(&text)
        .expect("the same export imports again, and rebuilds the graph");

    let after = tie(&session, &before.relationship_id);
    assert!(
        after.interaction_count > before.interaction_count,
        "the second import wrote no events, so this proves nothing about a rebuild",
    );
    assert_eq!(after.band, chosen, "the rebuild moved the user's band");
    assert!(after.locked_by_user);
    assert_eq!(after.user_band.as_deref(), Some(chosen));
    assert!(
        after.machine_band.is_some(),
        "the rebuild dropped what the counts say, so the screen can no longer show it",
    );

    drop(keep);
}

/// The way back out. Releasing hands the band to the counts, takes the lock
/// off, and is recorded like the correction was.
#[test]
fn releasing_hands_the_band_back_to_the_counts() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);

    let before = a_tie(&session);
    session
        .correct_tie(&before.relationship_id, other_than(&before.band))
        .expect("correct");
    let corrections = session
        .audit()
        .expect("chain")
        .entries
        .iter()
        .filter(|entry| entry.action == "profile.correct")
        .count();

    let graph = session
        .release_tie(&before.relationship_id)
        .expect("the user asked for the counts to speak again");

    let after = graph
        .ties
        .iter()
        .find(|tie| tie.relationship_id == before.relationship_id)
        .expect("the released tie is in the answer");
    assert!(!after.locked_by_user);
    assert_eq!(after.user_band, None);
    assert_eq!(after.machine_band, None);
    assert_eq!(
        after.band, before.band,
        "the counts have not changed, so the band the machine gives back is the first one",
    );

    let chain = session.audit().expect("chain");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert_eq!(
        chain
            .entries
            .iter()
            .filter(|entry| entry.action == "profile.correct")
            .count(),
        corrections + 1,
        "a release is an action the user took, and the chain hears about it",
    );

    drop(keep);
}

/// Three words in, and nothing else reaches the store.
///
/// The vocabulary check is the session's rather than the graph's on purpose:
/// what the WebView sends is a string, and a band nobody offers has to come
/// back as a refusal with a code on it rather than as a panic or as a fourth
/// band written to an edge.
#[test]
fn a_word_that_is_not_a_band_is_refused_and_writes_nothing() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);

    let before = a_tie(&session);
    let entries = session.audit().expect("chain").entries.len();

    for refused in ["", "Strong", "very_strong", "强", "unknown"] {
        let refusal = session
            .correct_tie(&before.relationship_id, refused)
            .expect_err("that is not a band this product has");
        assert_eq!(refusal.reason_code, "ROUTINE", "{refused}");
        assert!(
            refusal.explanation.contains('弱')
                && refusal.explanation.contains("中等")
                && refusal.explanation.contains('强'),
            "the refusal does not say which three words there are: {refusal}",
        );
    }

    let after = tie(&session, &before.relationship_id);
    assert_eq!(after.band, before.band);
    assert!(!after.locked_by_user);
    assert_eq!(
        session.audit().expect("chain").entries.len(),
        entries,
        "a refused correction wrote to the chain",
    );

    drop(keep);
}

/// An edge nobody has is refused rather than invented.
///
/// v0.1 has no way to add a tie: an edge nothing was observed for is the one
/// thing the evidence rule forbids. Both halves of "which edge" are checked —
/// a string that is not an identifier at all, and one that is a perfectly good
/// identifier for an edge this store does not hold.
#[test]
fn an_edge_this_store_does_not_hold_is_refused_rather_than_created() {
    let (keep, directory) = scratch();
    let mut session = imported(&directory);

    let ties = session.people().expect("graph").ties.len();
    let absent = uuid::Uuid::now_v7().to_string();

    for (relationship_id, what) in [
        (absent.as_str(), "an edge nobody has"),
        ("not-a-relationship", "not an identifier"),
    ] {
        let refusal = session
            .correct_tie(relationship_id, "strong")
            .expect_err(what);
        assert!(!refusal.reason_code.is_empty(), "{what}");
        assert!(!refusal.explanation.is_empty(), "{what}");

        let refusal = session.release_tie(relationship_id).expect_err(what);
        assert!(!refusal.reason_code.is_empty(), "{what}");
    }

    assert_eq!(
        session.people().expect("graph").ties.len(),
        ties,
        "a correction to nothing grew the graph an edge",
    );

    drop(keep);
}

/// A machine whose store did not open says so, rather than answering with a
/// graph that has no ties in it. Same shape as every other read on this
/// session: a code the screen can branch on and a sentence it can render.
#[test]
fn a_store_that_will_not_open_refuses_the_correction() {
    let (keep, directory) = scratch();
    let occupied = directory.join("not-a-directory");
    std::fs::write(&occupied, b"a file where a directory would have to be").expect("write");
    let mut session = Session::open(occupied.join("data"));

    assert!(!session.status().store_opened, "the directory opened");
    let refusal = session
        .correct_tie(&uuid::Uuid::now_v7().to_string(), "strong")
        .expect_err("there is no store to correct anything in");
    assert_eq!(refusal.reason_code, "ROUTINE");
    assert!(refusal
        .explanation
        .contains(soulcore::commands::session::STORE_UNAVAILABLE_NOTICE));

    drop(keep);
}
