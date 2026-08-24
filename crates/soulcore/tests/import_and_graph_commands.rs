//! The WP05 and WP06 command surfaces, exercised the way WP09 will bind them.
//!
//! Everything they promise is already proved inside `soul-import` and
//! `soul-graph`. What is checked here is that the thin wrappers reach it, and
//! the one thing only they do: appending the audit entries those crates hand
//! back, so that a caller who goes through the command surface cannot end up
//! with an import the chain never heard about.

use uuid::Uuid;

use soul_import::questionnaire::AnswerShape;
use soul_profile::questionnaire::{Answer, QuestionnaireResponse};
use soul_schema::audit::AuditAction;
use soul_schema::event::EventSource;
use soul_schema::profile::AxisPosition;
use soul_store_api::types::EventFilter;
use soul_store_api::{AuditLog, EventStore, GraphStore};
use soul_testkit::fixtures;
use soulcore::commands::{
    graph as graph_commands, import as import_commands, profile as profile_commands,
};

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
///
/// The point of this test after WP13 is that there is no separate import-side
/// questionnaire to take. The fallback the import surface decides on is
/// answered through the profile surface, question for question, and what comes
/// out is a profile — not a second set of answers sitting beside one.
#[test]
fn the_fallback_questionnaire_is_the_profile_s_own_and_leaves_a_profile_behind() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = soulcore::commands::store::open_test_store(dir.path(), SEED).expect("open");
    let profile_id = Uuid::now_v7();

    assert!(import_commands::questionnaire_needed(None));

    // The two surfaces draw the same list; the profile one adds what each
    // question moves.
    let asked = import_commands::questions();
    let placed = profile_commands::questions();
    assert_eq!(asked.len(), placed.len());
    for (question, placeable) in asked.iter().zip(&placed) {
        assert_eq!(question.key, placeable.question_id);
    }

    // A wizard answers by handing back the option it drew, or what was typed.
    let given = |question: &soul_import::questionnaire::Question| match question.shape {
        AnswerShape::Choice(options) => options[options.len() - 1],
        AnswerShape::Prose => "晚上十一点以后基本不回。",
    };
    let answers = asked
        .iter()
        .map(|question| {
            Answer::for_question(question.key, given(question)).expect("the option that was drawn")
        })
        .collect();

    let outcome = profile_commands::intake(
        &mut store,
        profile_id,
        &QuestionnaireResponse::new("2026-08-24T09:30:00Z", answers),
        AT,
    )
    .expect("intake");

    assert_eq!(outcome.evidence_ids.len(), asked.len());
    assert_eq!(
        outcome
            .profile
            .trait_axes
            .iter()
            .filter(|axis| axis.position != AxisPosition::Unknown)
            .count(),
        5,
        "AC-03: answering leaves a profile that is not empty",
    );

    let events = store
        .list_events(&EventFilter::all())
        .expect("events")
        .into_iter()
        .filter(|event| event.source == EventSource::UiQuestionnaire)
        .count();
    assert_eq!(
        events,
        asked.len(),
        "the answers are events on the import side and a profile on the other, once each",
    );
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
