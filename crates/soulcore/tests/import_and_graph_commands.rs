//! The WP05 and WP06 command surfaces, exercised the way WP09 will bind them.
//!
//! Everything they promise is already proved inside `soul-import` and
//! `soul-graph`. What is checked here is that the thin wrappers reach it, and
//! the one thing only they do: appending the audit entries those crates hand
//! back, so that a caller who goes through the command surface cannot end up
//! with an import the chain never heard about.

use soul_import::questionnaire::{Answer, CollectingSink};
use soul_schema::audit::AuditAction;
use soul_schema::common::Timestamp;
use soul_store_api::{AuditLog, GraphStore};
use soul_testkit::fixtures;
use soulcore::commands::{graph as graph_commands, import as import_commands};

const SEED: &str = "soulcore import and graph commands";
const AT: i64 = 1_787_500_000;

#[test]
fn a_file_read_and_committed_through_the_commands_leaves_a_graph_and_a_chain() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = soulcore::commands::store::open_test_store(dir.path(), SEED).expect("open");

    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");
    let staged = import_commands::read_soul_import_v1(&text).expect("the fixture is valid");
    assert!(!import_commands::questionnaire_needed(Some(&staged)));

    let receipt = import_commands::commit(&mut store, &staged, AT).expect("commit");
    assert_eq!(receipt.events_written.len(), 16);

    let build = graph_commands::rebuild(&mut store, AT).expect("rebuild");
    assert_eq!(build.edges_written.len(), 4);

    let graph = graph_commands::load(&store).expect("load");
    assert_eq!(graph.third_party_nodes().len(), 4);
    for edge in &graph.edges {
        let evidence = graph_commands::edge_evidence(&store, edge).expect("resolve");
        assert_eq!(evidence.len(), edge.evidence_ids.len());
        assert_eq!(
            graph_commands::evidence_for(&store, edge.relationship_id)
                .expect("resolve by id")
                .len(),
            evidence.len(),
        );
    }

    let actions: Vec<AuditAction> = store
        .list_audit()
        .expect("chain")
        .into_iter()
        .map(|entry| entry.action)
        .collect();
    assert!(actions.contains(&AuditAction::ImportCommit));
    assert!(
        actions.contains(&AuditAction::InferenceWrite),
        "a rebuild writes inferences and the chain has to say so: {actions:?}",
    );
}

/// Reading is separate from committing so a caller can show the user what is
/// wrong with a file before anything lands.
#[test]
fn reading_a_broken_file_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = soulcore::commands::store::open_test_store(dir.path(), SEED).expect("open");

    let text =
        fixtures::read_text("import/soul-import-v1/invalid_missing_field.jsonl").expect("fixture");
    let failure = import_commands::read_soul_import_v1(&text).expect_err("broken");
    assert!(!failure.defects.is_empty());

    assert!(store.list_contacts().expect("contacts").is_empty());
    assert!(store.list_audit().expect("chain").is_empty());
}

/// The AC-03 path through the command surface: no file, so the questionnaire.
#[test]
fn the_questionnaire_path_records_answers_and_audits_them() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = soulcore::commands::store::open_test_store(dir.path(), SEED).expect("open");

    assert!(import_commands::questionnaire_needed(None));
    let asked = import_commands::questions();
    assert!(!asked.is_empty());

    let answers: Vec<Answer> = asked
        .iter()
        .take(3)
        .map(|question| Answer::new(question.key, "先推进，细节边做边补。"))
        .collect();
    let mut sink = CollectingSink::default();
    let receipt = import_commands::record_questionnaire(
        &mut store,
        &answers,
        &Timestamp::new("2026-08-24T09:30:00Z"),
        &mut sink,
        AT,
    )
    .expect("record");

    assert_eq!(receipt.answers.len(), 3);
    assert_eq!(sink.accepted.len(), 3);
    assert_eq!(store.list_audit().expect("chain").len(), 1);
}

#[test]
fn a_telegram_export_reaches_the_store_through_the_command_surface() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = soulcore::commands::store::open_test_store(dir.path(), SEED).expect("open");

    let document: serde_json::Value =
        fixtures::read_json("import/telegram/result_basic.json").expect("fixture");
    let staged = import_commands::read_telegram(&document).expect("valid");
    let receipt = import_commands::commit(&mut store, &staged, AT).expect("commit");

    assert_eq!(receipt.events_written.len(), 6);
    graph_commands::rebuild(&mut store, AT).expect("rebuild");
    assert_eq!(
        graph_commands::load(&store)
            .expect("load")
            .third_party_nodes()
            .len(),
        2,
    );
}
