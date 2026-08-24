//! The WP03 and WP04 command surfaces, exercised end to end on a Linux host.
//!
//! Everything asserted here is already asserted inside `soul-profile` and
//! `soul-memory`. What this test adds is the wiring: that the commands reach a
//! real SQLCipher store rather than the `FakeStore` those crates unit-test
//! against, that the two surfaces share one store and one audit chain without
//! interfering, and that the chain still verifies after both have written to it.
//!
//! This is the shape WP09 will bind the desktop shell to, so it runs headless:
//! no platform key store, no UI, and the clock passed in rather than read.

use soul_profile::questionnaire::{Answer, QuestionnaireResponse};
use soul_profile::{AxisProposal, AxisUpdate, VoiceRegister, VoiceSetting, CURIOSITY, ORDERLINESS};
use soul_schema::common::{SealedSubject, SupportedBand};
use soul_schema::memory::MemoryType;
use soul_schema::profile::AxisPosition;
use soul_store_api::types::InferenceState;
use soul_store_api::{AuditLog, ProfileStore, SoulStore};
use soulcore::commands::store as store_commands;
use soulcore::commands::{memory as memory_commands, profile as profile_commands};
use uuid::Uuid;

const SEED: &str = "soulcore profile and memory commands";
/// 2026-08-24T00:00:00Z.
const NOW: i64 = 1_787_529_600;

const TITLE: &str = "第一次一个人搬家";
const SUMMARY: &str = "东西比想的多，最后一趟是抱着一盆快死的绿萝走回去的。";

#[test]
fn the_questionnaire_a_correction_and_a_memory_share_one_store_and_one_chain() {
    let dir = tempfile::tempdir().expect("temp dir");
    let profile_id = Uuid::now_v7();

    let memory_id = {
        let mut store = store_commands::open_test_store(dir.path(), SEED).expect("open");

        // --- WP03: intake, then a correction that has to survive inference ---
        let questions = profile_commands::questions();
        assert!(
            questions.len() >= 5,
            "one question per axis at the very least",
        );

        let outcome =
            profile_commands::intake(&mut store, profile_id, &two_answers(), NOW).expect("intake");
        assert_eq!(outcome.evidence_ids.len(), 2);
        assert_eq!(
            outcome.profile.trait_axes.len(),
            5,
            "a profile carries all five axes even when only two were answered",
        );

        profile_commands::correct_axis(
            &mut store,
            profile_id,
            CURIOSITY.axis_id,
            AxisPosition::Mixed,
            NOW,
        )
        .expect("correct");

        // An inference about the corrected axis is stored and refused; one
        // about an axis the user has not touched lands.
        let refused = profile_commands::record_inference(
            &mut store,
            profile_id,
            AxisProposal::new(
                CURIOSITY.axis_id,
                AxisPosition::LeansHigh,
                SupportedBand::Strong,
                outcome.evidence_ids.clone(),
            ),
            NOW,
        )
        .expect("record the inference about the locked axis");
        assert_eq!(refused.update, AxisUpdate::RefusedAxisLocked);

        let applied = profile_commands::record_inference(
            &mut store,
            profile_id,
            AxisProposal::new(
                ORDERLINESS.axis_id,
                AxisPosition::LeansLow,
                SupportedBand::Weak,
                outcome.evidence_ids.clone(),
            ),
            NOW,
        )
        .expect("record the inference about the open axis");
        assert_eq!(applied.update, AxisUpdate::Applied);

        // --- WP04: one memory, citing the questionnaire evidence ---
        let evidence_id = outcome.evidence_ids[0];
        let memory = memory_commands::create(
            &mut store,
            &soul_memory::MemoryDraft::own(MemoryType::Episodic, TITLE, SUMMARY)
                .about(SealedSubject::Owner)
                .citing(&[evidence_id]),
            NOW,
        )
        .expect("create the memory");

        let content = memory_commands::read(&store, memory.memory_id).expect("read it back");
        assert_eq!(content.title, TITLE);
        assert_eq!(content.summary, SUMMARY);

        store.flush().expect("flush");
        store.close().expect("close, as if the application exited");
        memory.memory_id
    };

    // Everything below runs against the file, reopened from scratch.
    let store = store_commands::open_test_store(dir.path(), SEED).expect("reopen");

    let view = profile_commands::view(&store, profile_id).expect("the view resolves all evidence");
    let curiosity = view
        .axes
        .iter()
        .find(|axis| axis.axis_id == CURIOSITY.axis_id)
        .expect("curiosity is in the view");
    assert!(
        curiosity.locked_by_user,
        "the correction has to survive the restart, or the user will be \
         overruled by the next inference after every relaunch",
    );
    assert_eq!(curiosity.position, AxisPosition::Mixed);
    assert_eq!(
        curiosity.inferences.len(),
        1,
        "the refused inference is kept: the user is entitled to see that the \
         machine still thinks otherwise",
    );
    for inference in &curiosity.inferences {
        assert!(
            !inference.evidence.is_empty(),
            "AC-06: every inference in the view has its evidence resolved",
        );
    }

    let rendered = profile_commands::render(&store, profile_id).expect("render");
    assert!(
        rendered.contains(soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE),
        "anything the user reads about themselves says it is a working hypothesis",
    );

    let content = memory_commands::read(&store, memory_id).expect("the memory survives a restart");
    assert_eq!(content.title, TITLE);

    // --- The forget, and what it reaches across both work packages ---
    let mut store = store;
    let impact = memory_commands::preview_forget(&store, memory_id).expect("preview");
    assert_eq!(impact.memories_affected, 1);
    assert_eq!(impact.sealed_blobs_destroyed, 2);
    assert_eq!(
        impact.inferences_orphaned, 2,
        "both inferences cite the questionnaire evidence this memory rests on",
    );

    let receipt = memory_commands::forget(&mut store, memory_id, NOW).expect("forget");
    assert_eq!(
        receipt.impact, impact,
        "the receipt charges what was quoted"
    );

    assert!(
        matches!(
            memory_commands::read(&store, memory_id),
            Err(soul_memory::MemoryError::Forgotten(_)),
        ),
        "the prose is gone; the fact that it existed is not",
    );
    for inference in store.list_inferences().expect("inferences") {
        assert_eq!(
            store
                .inference_state(inference.inference_id)
                .expect("state"),
            InferenceState::Orphaned,
            "an inference resting on forgotten evidence must be demoted",
        );
    }

    // The audit chain carried both work packages' writes and none of their words.
    let audit = store.list_audit().expect("audit");
    store
        .verify_audit_chain()
        .expect("the chain verifies after a restart and a forget");
    let encoded = serde_json::to_string(&audit).expect("serialize");
    for prose in [TITLE, SUMMARY] {
        assert!(!encoded.contains(prose), "the audit chain holds no prose");
    }
    for id in [profile_id, memory_id] {
        assert!(
            encoded.contains(&id.to_string()),
            "ids are what the chain is for, and they outlive the words -- \
             including the id of a memory whose words are gone",
        );
    }
}

#[test]
fn a_voice_the_user_set_is_what_the_draft_layer_reads_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let profile_id = Uuid::now_v7();
    let mut store = store_commands::open_test_store(dir.path(), SEED).expect("open");

    profile_commands::set_voice(
        &mut store,
        profile_id,
        VoiceSetting::Register(VoiceRegister::Formal),
        NOW,
    )
    .expect("set the voice");

    let displaced = profile_commands::suggest_voice(
        &mut store,
        profile_id,
        VoiceSetting::Register(VoiceRegister::Casual),
        NOW,
    )
    .expect("suggest");
    assert!(
        !displaced,
        "an inferred voice must not quietly displace one the user chose; WP10 \
         drafts in this voice and the user will be the one sending it",
    );

    assert_eq!(
        profile_commands::voice(&store, profile_id)
            .expect("read the voice")
            .register,
        VoiceRegister::Formal,
    );
}

/// Two axis answers, which is enough to prove the surface is wired without
/// restating `soul-profile`'s own fixture-driven intake test.
fn two_answers() -> QuestionnaireResponse {
    let questions = soul_profile::questionnaire();
    let answers = [
        (CURIOSITY.question_id, AxisPosition::LeansHigh),
        (ORDERLINESS.question_id, AxisPosition::LeansLow),
    ]
    .into_iter()
    .map(|(question_id, position)| {
        assert!(
            questions.iter().any(|q| q.question_id == question_id),
            "{question_id} is a question the questionnaire actually asks",
        );
        Answer::Axis {
            question_id: question_id.to_owned(),
            position,
        }
    })
    .collect();

    QuestionnaireResponse::new("2026-08-24T00:00:00Z", answers)
}
