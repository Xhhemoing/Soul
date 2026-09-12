//! AC-06 on the profile side, plus the two red lines the axes carry.
//!
//! * every stored inference resolves to real evidence, and one with no evidence
//!   never reaches the store at all;
//! * the axis ids are fixed UUIDv7 constants, not something generated per
//!   install, and the readable half is the `label`;
//! * nothing about an axis is numeric, and nothing it says is clinical.

use serde_json::{json, Value};
use uuid::Uuid;

use soul_policy::clinical::{assert_non_clinical, denied_terms_in, term_count};
use soul_profile::numeric::reject_numeric_rating_value;
use soul_profile::{
    axes, intake, profile_view, reject_numeric_rating, render, AxisProposal, ProfileError,
};
use soul_schema::common::{Privacy, Purpose, SchemaVersion, Subject, SupportedBand};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::profile::AxisPosition;
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;
use soul_store_api::types::StoreError;
use soul_store_api::{FakeStore, ProfileStore};
use soul_testkit::fixtures;

const NOW: i64 = 1_787_529_600;

/// `_defs.schema.json#/$defs/uuid7`, checked by hand so this test does not
/// depend on the validator it is trying to corroborate.
fn looks_like_uuid7(id: Uuid) -> bool {
    let text = id.to_string();
    let bytes: Vec<char> = text.chars().collect();
    text.len() == 36
        && bytes[14] == '7'
        && matches!(bytes[19], '8' | '9' | 'a' | 'b')
        && text
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase() || c == '-')
}

fn evidence(store: &mut FakeStore) -> Uuid {
    let evidence_id = Uuid::now_v7();
    store
        .put_evidence(SoulEvidence {
            schema_version: SchemaVersion,
            evidence_id,
            kind: EvidenceKind::UserStatement,
            subject: Subject::Owner,
            source_refs: vec![json!({ "origin": "wp03-test" })],
            strength: SupportedBand::Moderate,
            method: Some(EvidenceMethod::Manual),
            exportable_to_research: Some(false),
            privacy: Some(Privacy::local_only(
                Subject::Owner,
                vec![Purpose::SoulProfile],
            )),
        })
        .expect("evidence");
    evidence_id
}

#[test]
fn the_five_axis_ids_are_fixed_uuid7_constants_with_readable_labels() {
    assert_eq!(axes::DEFAULT_AXES.len(), 5);

    let mut seen: Vec<Uuid> = Vec::new();
    for axis in axes::DEFAULT_AXES {
        assert!(
            looks_like_uuid7(axis.axis_id),
            "{} is not shaped like a uuid7",
            axis.axis_id,
        );
        assert!(!seen.contains(&axis.axis_id), "duplicate axis id");
        seen.push(axis.axis_id);

        assert!(!axis.label.is_empty(), "the readable name lives in `label`");
        assert_eq!(axes::axis_by_id(axis.axis_id), Some(axis));
        assert_eq!(axes::axis_by_key(axis.key), Some(axis));
    }

    // The same constants, spelled out in a fixture. Changing one in the source
    // without changing the other forks every profile ever written.
    let profile: Value = fixtures::read_json("profile/default_axes_profile.json").expect("load");
    let recorded: Vec<&str> = profile["trait_axes"]
        .as_array()
        .expect("axes")
        .iter()
        .map(|axis| axis["axis_id"].as_str().expect("id"))
        .collect();
    let expected: Vec<String> = axes::DEFAULT_AXES
        .iter()
        .map(|axis| axis.axis_id.to_string())
        .collect();
    assert_eq!(recorded, expected);

    SchemaSet::load()
        .expect("contracts compile")
        .validate(SchemaId::Profile, &profile)
        .expect("the blank profile validates against the frozen contract");
}

#[test]
fn nothing_an_axis_says_is_clinical_and_nothing_it_carries_is_numeric() {
    assert!(
        term_count() > 60,
        "the denylist has to be loaded for this test to mean anything",
    );

    for axis in axes::DEFAULT_AXES {
        for text in [axis.label, axis.leans_low, axis.leans_high] {
            assert_non_clinical(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        }
        for position in [
            AxisPosition::LeansLow,
            AxisPosition::Mixed,
            AxisPosition::LeansHigh,
            AxisPosition::Unknown,
        ] {
            let statement_key = axis.statement_key(position);
            assert_non_clinical(&statement_key).expect("statement key");
            assert!(
                !statement_key.chars().any(|c| c.is_ascii_digit()),
                "{statement_key} carries a digit",
            );
            assert_non_clinical(&axis.describe(position)).expect("reading");
        }
    }

    // The wording of the questions lives in the one canonical list now, so
    // that is where it gets checked. `no_question_reaches_for_diagnostic_
    // vocabulary` in `soul-import` covers the same ground from the other side.
    for question in soul_profile::questionnaire() {
        assert_non_clinical(question.prompt)
            .unwrap_or_else(|e| panic!("{}: {e}", question.question_id));
    }

    reject_numeric_rating(&axes::blank_axes()).expect("a blank profile is number-free");

    // The walker has to be armed. `TraitAxis` has no field a number could go
    // in today, so the typed check alone could be a comforting no-op.
    let doctored = json!([{
        "axis_id": axes::CURIOSITY.axis_id,
        "label": axes::CURIOSITY.label,
        "position": "leans_high",
        "evidence_band": "strong",
        "clinical_claim": false,
        "confidence": 0.82
    }]);
    let refused = reject_numeric_rating_value(&doctored).expect_err("a number must be refused");
    assert!(
        refused.path.ends_with("confidence"),
        "the refusal names the offending field: {refused}",
    );

    // Same argument for the vocabulary check: it says the axes are clean, so
    // show it is capable of saying otherwise.
    assert!(denied_terms_in(axes::EMOTIONAL_STEADINESS.label).is_empty());
    assert!(
        !denied_terms_in("百分位得分").is_empty(),
        "the denylist check has to be able to fail",
    );
}

/// Every voice a profile can hold, rendered and checked.
///
/// Eighty-one combinations is cheap, and one of them used to read 少量表情,
/// which contains the scale word 量表. A test that only rendered the default
/// voice would have shipped it.
#[test]
fn no_voice_a_profile_can_hold_renders_into_forbidden_vocabulary() {
    use soul_profile::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth};

    let mut checked = 0usize;
    for register in [
        VoiceRegister::Casual,
        VoiceRegister::Plain,
        VoiceRegister::Formal,
    ] {
        for directness in [
            VoiceDirectness::Reserved,
            VoiceDirectness::Balanced,
            VoiceDirectness::Direct,
        ] {
            for warmth in [VoiceWarmth::Cool, VoiceWarmth::Even, VoiceWarmth::Warm] {
                for emoji_use in [EmojiUse::Never, EmojiUse::Sparing, EmojiUse::Frequent] {
                    let voice = VoiceProfile {
                        register,
                        directness,
                        warmth,
                        emoji_use,
                        user_set: Default::default(),
                    };
                    let rendered = soul_profile::render_voice(&voice);
                    assert_non_clinical(&rendered)
                        .unwrap_or_else(|e| panic!("`{rendered}` is not sayable: {e}"));
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 81);
}

#[test]
fn every_stored_inference_resolves_to_real_evidence() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();
    let response =
        soul_profile::questionnaire::every_axis(AxisPosition::Mixed, "2026-08-24T09:00:00Z");
    intake(&mut store, profile_id, &response, NOW).expect("intake");

    let first = evidence(&mut store);
    let second = evidence(&mut store);
    soul_profile::record_axis_inference(
        &mut store,
        profile_id,
        AxisProposal::new(
            axes::SOCIAL_ENERGY.axis_id,
            AxisPosition::LeansLow,
            SupportedBand::Moderate,
            vec![first, second],
        )
        .falsified_by("连续两周主动约人见面"),
        NOW,
    )
    .expect("inference");

    let view = profile_view(&store, profile_id).expect("view");
    let mut inference_count = 0usize;
    for axis in &view.axes {
        for inference in &axis.inferences {
            inference_count += 1;
            assert!(
                !inference.evidence.is_empty(),
                "AC-06: an inference with no dereferenceable evidence is not allowed to exist",
            );
            assert_eq!(
                inference.evidence.len(),
                inference.inference.evidence_ids.len(),
                "every cited id came back as a row",
            );
            for (resolved, cited) in inference
                .evidence
                .iter()
                .zip(&inference.inference.evidence_ids)
            {
                assert_eq!(resolved.evidence_id, *cited);
            }
        }
        for cited in &axis.evidence {
            assert_eq!(
                store
                    .get_evidence(cited.evidence_id)
                    .expect("resolves")
                    .evidence_id,
                cited.evidence_id,
            );
        }
    }
    assert_eq!(inference_count, 1);

    let rendered = render(&view).expect("render");
    assert!(rendered.contains(view.notice), "the notice travels with it");
    assert_non_clinical(&rendered).expect("the rendered profile is not a clinical statement");
}

#[test]
fn an_inference_with_no_evidence_never_reaches_the_store() {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();
    let response =
        soul_profile::questionnaire::every_axis(AxisPosition::Mixed, "2026-08-24T09:00:00Z");
    intake(&mut store, profile_id, &response, NOW).expect("intake");
    let before = store.list_inferences().expect("inferences").len();

    let refused = soul_profile::record_axis_inference(
        &mut store,
        profile_id,
        AxisProposal::new(
            axes::ACCOMMODATION.axis_id,
            AxisPosition::LeansHigh,
            SupportedBand::Weak,
            Vec::new(),
        ),
        NOW,
    )
    .expect_err("an unsupported claim must be refused");
    assert!(matches!(refused, ProfileError::NoEvidence), "{refused}");

    let dangling = soul_profile::record_axis_inference(
        &mut store,
        profile_id,
        AxisProposal::new(
            axes::ACCOMMODATION.axis_id,
            AxisPosition::LeansHigh,
            SupportedBand::Weak,
            vec![Uuid::now_v7()],
        ),
        NOW,
    )
    .expect_err("evidence that does not resolve is no evidence");
    assert!(
        matches!(
            dangling,
            ProfileError::Store(StoreError::ContractViolation(_))
        ),
        "{dangling}",
    );

    assert_eq!(
        store.list_inferences().expect("inferences").len(),
        before,
        "neither attempt left anything behind",
    );
}
