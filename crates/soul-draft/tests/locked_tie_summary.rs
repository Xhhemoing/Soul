//! GC-9a: a people summary about an edge the user corrected says nothing about
//! filing it.
//!
//! The filing sentence — 「按上面的计数，这段往来归在「…」一档；这是一个工作假设…」 — makes
//! two claims that hold on a derived band and fail on a corrected one: that the
//! band follows from the counts printed above it, and that it is a working
//! hypothesis. On an edge the user has ruled on, neither is true.
//!
//! There are three things one could do about that, and only one of them is
//! allowed here. Rendering the frozen sentence anyway says something false.
//! Writing a variant — 「这一档由你本人指定」 or anything like it — puts user-facing
//! copy in a crate, which is the one thing `docs/algorithms/COPY_ZH.md`'s
//! header rule forbids: the template changes in that file first, with a record
//! of the change, or it does not change. What is left is silence, and silence
//! costs nothing here because the band, the lock and the machine's own reading
//! all reach the interface through the graph view as tokens.
//!
//! So this file asserts the suppression *and* the absence of a homemade
//! replacement. The second assertion is the one that would catch the tempting
//! fix.

use uuid::Uuid;

use soul_draft::analysis;
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{correct_tie, release_tie, Direction, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};

const NOW: i64 = 1_787_529_600;

/// The frozen filing template, reduced to the two fragments that make it what
/// it is. Matching on fragments rather than the whole sentence means a reworded
/// filing point still trips this test.
const FILING_FRAGMENTS: [&str; 2] = ["按上面的计数", "一档"];

/// The variant Round 2 drafted, Round 3 accepted in principle, and COPY_ZH did
/// not freeze. Until it is in the frozen file it must not be anywhere in this
/// repository.
const UNFROZEN_VARIANT: &str = "由你本人指定";

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

fn observe(store: &mut FakeStore, evidence_id: Uuid, direction: Direction, at: &str) {
    let observation = InteractionRef::new(
        id("900"),
        owner(),
        peer(),
        conversation_ref("test", "chat-1"),
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
        .expect("the evidence row is written");
}

/// One partner, six exchanges over three days, and the edge derived from them.
fn store_with_one_partner() -> (FakeStore, Uuid) {
    let mut store = FakeStore::new();
    store
        .put_contact(contact(owner(), ContactClass::Owner))
        .expect("the owner");
    store
        .put_contact(contact(peer(), ContactClass::ThirdParty))
        .expect("the other person");

    let mut next = 100u32;
    for (index, day) in ["2026-03-02", "2026-03-05", "2026-03-09"]
        .iter()
        .enumerate()
    {
        for direction in [Direction::Outgoing, Direction::Incoming] {
            next += 1;
            observe(
                &mut store,
                id(&next.to_string()),
                direction,
                &format!("{day}T0{index}:15:00Z"),
            );
        }
    }
    soul_graph::rebuild(&mut store).expect("the graph derives");

    let graph = soul_graph::load(&store).expect("the graph loads");
    let relationship_id = graph.edges_for(peer())[0].relationship_id;
    (store, relationship_id)
}

/// The rendered summary for the one partner in the fixture.
fn rendered(store: &FakeStore) -> String {
    let graph = soul_graph::load(store).expect("the graph loads");
    let edge = graph.edges_for(peer())[0].clone();
    let resolved = soul_graph::resolve_evidence(store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");
    analysis::render(&summary).expect("the summary renders")
}

fn files_the_tie(text: &str) -> bool {
    FILING_FRAGMENTS
        .iter()
        .all(|fragment| text.contains(fragment))
}

/// The control: on an edge nobody has corrected, the filing sentence is exactly
/// where it has always been. Without this the suppression test would pass just
/// as well against a summary that never files anything.
#[test]
fn an_uncorrected_edge_is_still_filed_under_a_band() {
    let (store, _) = store_with_one_partner();
    let text = rendered(&store);
    assert!(
        files_the_tie(&text),
        "the frozen filing sentence belongs on a derived band:\n{text}",
    );
}

/// GC-9a: the correction takes the filing sentence out, and nothing takes its
/// place.
#[test]
fn a_corrected_edge_is_not_filed_under_anything() {
    let (mut store, relationship_id) = store_with_one_partner();
    let before = rendered(&store);

    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("correct");
    let after = rendered(&store);

    assert!(
        !files_the_tie(&after),
        "a band the user set does not follow from the counts above it:\n{after}",
    );
    assert!(
        !after.contains(UNFROZEN_VARIANT),
        "a replacement sentence is copy, and copy is frozen elsewhere:\n{after}",
    );

    // Everything else the summary said is a count, and a count is unaffected by
    // who chose the word for it.
    for line in before.lines().filter(|line| !files_the_tie(line)) {
        assert!(
            after.contains(line),
            "the correction removed more than the filing sentence: `{line}`",
        );
    }
}

/// Releasing the tie hands the sentence back, because the band is once again
/// what the counts say it is.
#[test]
fn releasing_the_tie_brings_the_filing_sentence_back() {
    let (mut store, relationship_id) = store_with_one_partner();
    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("correct");
    release_tie(&mut store, relationship_id, NOW).expect("release");

    let text = rendered(&store);
    assert!(
        files_the_tie(&text),
        "after a release the band is the machine's again:\n{text}",
    );
}

/// A summary is never left with nothing to say. The points that go missing are
/// the ones that stopped being true, not the ones the user came to read.
#[test]
fn a_corrected_edge_still_has_points_and_they_still_cite_evidence() {
    let (mut store, relationship_id) = store_with_one_partner();
    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("correct");

    let graph = soul_graph::load(&store).expect("the graph loads");
    let edge = graph.edges_for(peer())[0].clone();
    let resolved = soul_graph::resolve_evidence(&store, &edge).expect("the evidence resolves");
    let summary = analysis::summarize_person(&graph, peer(), &resolved).expect("a summary");

    assert!(!summary.points.is_empty());
    for point in &summary.points {
        assert!(!point.evidence_ids().is_empty(), "{}", point.statement());
        for cited in point.evidence_ids() {
            store.get_evidence(*cited).expect("the row resolves");
        }
    }
}

/// The negative assertion that outlives this file: no unfrozen filing variant
/// anywhere in the tree, fixtures included. Copy reaches COPY_ZH before it
/// reaches code, and this is what makes that checkable rather than remembered.
#[test]
fn no_unfrozen_filing_variant_is_anywhere_in_the_repository() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/soul-draft sits two levels below the repository root")
        .to_path_buf();

    let this_file = std::path::Path::new(file!())
        .file_name()
        .expect("this file has a name")
        .to_owned();

    let mut pending = vec![
        root.join("crates"),
        root.join("apps"),
        root.join("fixtures"),
    ];
    let mut scanned = 0usize;
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            for entry in std::fs::read_dir(&path).expect("read a directory") {
                pending.push(entry.expect("an entry").path());
            }
            continue;
        }
        if path.file_name().is_some_and(|name| name == this_file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        scanned += 1;
        assert!(
            !text.contains(UNFROZEN_VARIANT),
            "{} carries a filing variant COPY_ZH has not frozen",
            path.display(),
        );
    }
    assert!(scanned > 100, "the scan found almost nothing to read");
}
