//! Minimal Session/SQLCipher bridge, without duplicating q2_continuity.
use super::{
    briefs::build_brief,
    endpoint::{EndpointInput, EndpointPair},
    endpoint_tests::{assert_unrated, evaluation, manual, material},
    records::{TaskType, Variant},
};
use soul_draft::ProfileBrief;
use soul_policy::redactor::{NAME_PLACEHOLDER, THIRD_PARTY_PLACEHOLDER};
use soul_profile::profile_view;
use soul_schema::{contact::ContactClass, memory::ForgetState};
use soul_store_api::{BlobStore, GraphStore};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::{
    draft::known_identifiers,
    session::{Session, OWNER_PROFILE_ID},
};

#[test]
fn endpoint_session_sqlcipher_b_matches_wire_and_correction_survives_restart_without_prose_audit() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    let reply = "合成回复：已收到安排，后续再核对。";
    let pasted = "李 雷和李雷说周五的会场已确认，请再核对时间。";
    endpoint.set_reply(reply);
    let mut session = Session::open(directory.path());
    let import = fixtures::read_text("import/telegram/result_basic.json").unwrap();
    session.commit_telegram(&import).unwrap();
    session.set_voice("warmth", "warm").unwrap();
    session.set_voice("register", "formal").unwrap();
    let screen = session.profile().unwrap();
    let store = session.store().unwrap();
    let (view, identifiers) = {
        let store = store.lock().unwrap();
        let labels: Vec<_> = store
            .list_contacts()
            .unwrap()
            .into_iter()
            .filter(|c| {
                c.contact_class != ContactClass::Owner && c.forget_state == ForgetState::Active
            })
            .filter_map(|c| c.display_label_ref)
            .map(|sealed| String::from_utf8(store.open(&sealed).unwrap()).unwrap())
            .collect();
        assert!(labels.iter().any(|label| label == "李 雷"));
        (
            profile_view(&*store, OWNER_PROFILE_ID).unwrap(),
            known_identifiers(&store).unwrap(),
        )
    };
    let explicit_b = ProfileBrief::from_view(&view).unwrap();
    assert_eq!(
        build_brief(Variant::B, &view, manual()).unwrap(),
        explicit_b
    );
    assert_ne!(explicit_b, ProfileBrief::neutral());
    let rendered = explicit_b.render().unwrap();
    let (mut data, slot) = evaluation(TaskType::T3);
    let mut pair = EndpointPair::start(
        &mut data,
        &slot,
        11,
        EndpointInput {
            snapshot: &view,
            manual: manual(),
            identifiers,
            pasted,
            endpoint: Some(&endpoint.base_url()),
        },
    )
    .unwrap();
    let plan = pair.prepare(&mut data, 1, 12).unwrap();
    pair.generate(&mut data, 1, &plan.approval(), 13).unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let plan = session.prepare_draft(pasted, Some(true)).unwrap();
    let draft = session.generate_draft(&plan.approval()).unwrap();
    assert_eq!(draft.text, reply);
    assert_eq!(endpoint.request_count(), 2);
    for request in endpoint.requests() {
        let text = material(&request.body);
        assert!(
            text.contains(&rendered),
            "explicit B and Session wire share stored brief"
        );
        assert!(text.contains("周五的会场已确认"));
        assert!(text.contains(NAME_PLACEHOLDER));
        assert!(!text.contains("李 雷"));
        assert!(!text.contains("李雷"));
        assert!(!text.contains(THIRD_PARTY_PLACEHOLDER));
    }
    assert_unrated(&data);
    let audit = session.audit().unwrap();
    assert!(audit.verified);
    let encoded = serde_json::to_string(&audit).unwrap();
    for prose in [pasted, reply, "周五的会场已确认", "李 雷", "李雷"] {
        assert!(!encoded.contains(prose));
    }
    drop(store);
    drop(session);
    let reopened = Session::open(directory.path());
    assert_eq!(reopened.profile().unwrap(), screen);
    let store = reopened.store().unwrap();
    let reopened_b =
        ProfileBrief::from_view(&profile_view(&*store.lock().unwrap(), OWNER_PROFILE_ID).unwrap())
            .unwrap();
    assert_eq!(reopened_b, explicit_b);
    assert_eq!(reopened.audit().unwrap().entries, audit.entries);
    assert!(!reopened.snapshot().llm_endpoint_configured);
}
