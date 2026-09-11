//! AC-32's session path: lock a tie, then rebuild by importing again.

use soul_testkit::fixtures;
use soulcore::commands::session::Session;

#[test]
fn locking_a_tie_through_the_session_survives_a_later_rebuild() {
    let dir = tempfile::tempdir().expect("temp dir");
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");
    let mut session = Session::open(dir.path());
    session
        .commit_soul_import_v1(&text)
        .expect("the corpus commits");

    let first = session.people().expect("graph");
    let id = first.ties[0].relationship_id.clone();
    let peer = first.ties[0].to_contact_id.clone();
    let machine = first.ties[0].band.clone();
    let before_count = first.ties[0].interaction_count;

    let locked = session.correct_tie(&id, "strong").expect("lock");
    let tie = locked
        .ties
        .iter()
        .find(|tie| tie.relationship_id == id)
        .expect("the same edge");
    assert_eq!(tie.band, "strong");
    assert!(tie.locked_by_user);
    assert_eq!(tie.user_band.as_deref(), Some("strong"));
    assert_eq!(tie.machine_band.as_deref(), Some(machine.as_str()));
    assert!(
        tie.venue_split_measured,
        "the graph view carries the venue split the band was decided on",
    );

    let unfrozen_filing = ["由你本人", "指定"].concat();
    let summary = session.person_summary(&peer).expect("A2");
    assert!(
        !summary.text.contains("按上面的计数"),
        "a locked edge must not file the band from the counts: {}",
        summary.text,
    );
    assert!(
        !summary.text.contains(&unfrozen_filing),
        "COPY_ZH has not frozen a user-set filing sentence: {}",
        summary.text,
    );
    assert!(
        summary.points.iter().all(|point| point.band == "strong"),
        "A2 consumes the effective band, not the machine's: {:?}",
        summary.points,
    );

    let chain = session.audit().expect("the chain");
    assert!(chain.verified);
    let correction = chain
        .entries
        .iter()
        .find(|entry| entry.action == "profile.correct")
        .expect("the lock is on the chain");
    assert_eq!(correction.decision, "allowed");
    let encoded = serde_json::to_string(&chain).expect("serialize");
    for needle in ["公司门口", "café", "好的没问题", unfrozen_filing.as_str()] {
        assert!(
            !encoded.contains(needle),
            "an audit entry must not carry a body ({needle}): {encoded}",
        );
    }

    session
        .commit_soul_import_v1(&text)
        .expect("a second import rebuilds");
    let after = session.people().expect("graph");
    let tie = after
        .ties
        .iter()
        .find(|tie| tie.relationship_id == id)
        .expect("the edge is still there");
    assert_eq!(
        tie.band, "strong",
        "a rebuild must not lift the user's lock"
    );
    assert!(tie.locked_by_user);
    assert!(
        tie.interaction_count > before_count,
        "the machine's counts keep moving under the lock",
    );
    assert!(
        tie.machine_band.is_some(),
        "the machine's reading stays beside the lock and is still a band",
    );
}
