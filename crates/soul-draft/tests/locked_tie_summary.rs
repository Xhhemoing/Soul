//! GC-9a: a people summary about an edge the user corrected says nothing about
//! filing it.
//!
//! The filing sentence — 「按上面的计数，这条往来归在「…」一档。这是对记录的归档…」 — makes
//! two claims that hold on a derived band and fail on a corrected one: that the
//! band follows from the counts printed above it, and that it is the machine's
//! reading of the record. On an edge the user has ruled on, neither is true.
//!
//! There are three things one could do about that, and only one of them is
//! allowed here. Rendering the frozen sentence anyway says something false.
//! Writing a variant — 「这一档由你本人指定」 or anything like it — puts user-facing
//! copy in a crate, which is the one thing `docs/algorithms/COPY_ZH.md`'s
//! header rule forbids (D35, D48): the template changes in that file first,
//! with a record of the change, or it does not change. What is left is silence,
//! and silence costs nothing here because the band, the lock and the machine's
//! own reading all reach the interface through the graph view as tokens.
//!
//! # What is asserted here
//!
//! `soul_draft::analysis` drops the filing bullet on a locked edge, and
//! `soul_draft::a2_adapt::is_locked_by_user` is where it asks. The control — a
//! derived band is still filed — is what keeps that `if` from being one that is
//! never false, and the repository-wide negative assertion is the one that
//! would catch the tempting fix of writing the variant instead.

use uuid::Uuid;

use soul_draft::analysis;
use soul_graph::interaction::{conversation_ref, interaction_evidence, InteractionRef};
use soul_graph::{correct_tie, release_tie, Direction, Venue};
use soul_schema::common::{SchemaVersion, Subject, SupportedBand, Timestamp};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::memory::ForgetState;
use soul_store_api::{FakeStore, GraphStore, ProfileStore};

/// A fixed clock, so a replay of this file writes the same audit entries.
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
fn store_with_one_partner() -> FakeStore {
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
    store
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

/// AD-13 projection lines. They speak for the machine's demotion clock, so a
/// band the user locked must not keep forecasting it.
fn projects_the_clock(text: &str) -> bool {
    text.contains("如果你们一直没有新的往来")
}

/// The control: on an edge nobody has corrected, the filing sentence is exactly
/// where it has always been. Without this the suppression would pass just as
/// well against a summary that never files anything.
#[test]
fn an_uncorrected_edge_is_still_filed_under_a_band() {
    let store = store_with_one_partner();
    let text = rendered(&store);
    assert!(
        files_the_tie(&text),
        "the frozen filing sentence belongs on a derived band:\n{text}",
    );
}

/// The lock the summary asks about is the edge's, and no edge sets it by
/// accident.
///
/// The suppression in `analysis::points_for` is one `if` over
/// `a2_adapt::is_locked_by_user`, and an `if` that is never false is not a
/// rule. This pins the other side of it: a rebuilt edge is the machine's
/// reading, so it is not locked and the filing sentence stands.
#[test]
fn an_edge_the_rebuild_derived_is_not_a_band_the_user_set() {
    let store = store_with_one_partner();
    let graph = soul_graph::load(&store).expect("the graph loads");
    let edge = &graph.edges_for(peer())[0];

    assert!(!soul_draft::a2_adapt::is_locked_by_user(&edge.tie_strength));
    assert!(files_the_tie(&rendered(&store)));
}

/// GC-9a itself: once the user has ruled, the filing sentence is gone and
/// nothing has been written in its place.
#[test]
fn a_corrected_edge_is_not_filed_and_no_variant_replaces_the_sentence() {
    let mut store = store_with_one_partner();
    let relationship_id = tie(&store);
    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("the user rules");

    let text = rendered(&store);
    assert!(
        !files_the_tie(&text),
        "a band the user set does not follow from the counts above it:\n{text}",
    );
    assert!(
        !text.contains(UNFROZEN_VARIANT),
        "silence, not a sentence COPY_ZH has not frozen:\n{text}",
    );
}

/// A count is unaffected by who chose the word for it, so every count line of
/// the summary survives the correction character for character.
///
/// Two sentences do go away on purpose: the filing line (GC-9a) and the
/// demotion-clock projection (AD-13 / COPY_ZH §6), both of which speak for the
/// machine's reading of the band. Filtering them out of `before` is what keeps
/// this test about the counts rather than about those two suppressions.
#[test]
fn correcting_the_band_changes_no_other_line_of_the_summary() {
    let mut store = store_with_one_partner();
    let relationship_id = tie(&store);
    let before: Vec<String> = lines_of(&rendered(&store))
        .into_iter()
        .filter(|line| !files_the_tie(line) && !projects_the_clock(line))
        .collect();

    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("the user rules");

    let after = rendered(&store);
    assert!(
        !lines_of(&after).iter().any(|line| projects_the_clock(line)),
        "a locked edge does not forecast the machine demotion clock:\n{after}",
    );
    assert_eq!(before, lines_of(&after));
}

/// GC-3's half of the promise, seen from the summary: releasing hands the band
/// back to the counts, so the sentence that describes the counts comes back too.
#[test]
fn releasing_the_tie_hands_the_filing_sentence_back() {
    let mut store = store_with_one_partner();
    let relationship_id = tie(&store);
    correct_tie(&mut store, relationship_id, SupportedBand::Weak, NOW).expect("the user rules");
    assert!(!files_the_tie(&rendered(&store)));

    release_tie(&mut store, relationship_id, NOW).expect("the user hands it back");
    let text = rendered(&store);
    assert!(
        files_the_tie(&text),
        "the band is the machine's again, and it says how it got there:\n{text}",
    );
}

/// The edge the fixture's one partner is on.
fn tie(store: &FakeStore) -> Uuid {
    soul_graph::load(store)
        .expect("the graph loads")
        .edges_for(peer())[0]
        .relationship_id
}

/// The rendered summary as its individual lines.
fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

/// A summary is never left with nothing to say, and everything it does say
/// cites a row that resolves.
#[test]
fn the_summary_has_points_and_they_all_cite_evidence() {
    let store = store_with_one_partner();
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
