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
    let machine = first.ties[0].band.clone();

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

    session
        .commit_soul_import_v1(&text)
        .expect("a second import rebuilds");
    let after = session.people().expect("graph");
    let tie = after
        .ties
        .iter()
        .find(|tie| tie.relationship_id == id)
        .expect("the edge is still there");
    assert_eq!(tie.band, "strong", "a rebuild must not lift the user's lock");
    assert!(tie.locked_by_user);
}
