use soul_policy::e1::{chat_json_body, encoded_chat_body, E1Purpose, DRAFTING_INSTRUCTION};
use soul_policy::redactor::{
    combine_redacted, ExemptionRequest, KnownIdentifiers, RedactedBodyCombinationError, Redactor,
    Turn,
};
use soul_schema::common::SealedSubject;
use uuid::Uuid;

#[test]
fn composition_matches_single_pass_redaction_and_preserves_one_original_grant() {
    let redactor = Redactor::new(KnownIdentifiers::new().with_name("李雷"));
    let paste = Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, "李雷说周末见。");
    let memory = Turn::new(Uuid::now_v7(), SealedSubject::Owner, "我想去图书馆。");
    let third = Turn::new(Uuid::now_v7(), SealedSubject::Mixed, "不能外发的合成秘密。");
    let base = redactor.redact_for_e1_with_exemption(
        &[paste.clone()],
        ExemptionRequest::for_turn(paste.turn_id)
            .confirm(true)
            .unwrap(),
    );
    let owner = redactor.redact_for_e1(&[memory.clone()]);
    let private = redactor.redact_for_e1(&[third.clone()]);
    let combined = combine_redacted(&[&base, &owner, &private]).unwrap();
    let expected = redactor.redact_for_e1_with_exemption(
        &[paste.clone(), memory, third],
        ExemptionRequest::for_turn(paste.turn_id)
            .confirm(true)
            .unwrap(),
    );
    assert_eq!(combined, expected);
    assert_eq!(combined.third_party_turns(), 2);
    assert_eq!(combined.placeheld_turns(), 1);
    assert_eq!(combined.exempted_turn(), Some(paste.turn_id));
    assert!(!combined.as_str().contains("合成秘密"));
    assert!(!combined.as_str().contains("李雷"));
}

#[test]
fn combining_two_exemption_bodies_is_refused_even_for_the_same_turn_id() {
    let redactor = Redactor::default();
    let turn = Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, "合成正文");
    let base = redactor.redact_for_e1_with_exemption(
        &[turn.clone()],
        ExemptionRequest::for_turn(turn.turn_id)
            .confirm(true)
            .unwrap(),
    );
    assert_eq!(
        combine_redacted(&[&base, &base]),
        Err(RedactedBodyCombinationError::MultipleExemptions)
    );
}

#[test]
fn shared_serializer_measures_real_utf8_json_with_fences_model_and_escaping() {
    let redactor = Redactor::new(KnownIdentifiers::new().with_name("李"));
    let body = redactor.redact_for_e1(&[Turn::new(
        Uuid::now_v7(),
        SealedSubject::Owner,
        "李\n\"中文\\资料",
    )]);
    let encoded = encoded_chat_body(E1Purpose::Draft, "模型\"\\", &body);
    let parsed: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(parsed, chat_json_body(E1Purpose::Draft, "模型\"\\", &body));
    assert_eq!(parsed["messages"][0]["content"], DRAFTING_INSTRUCTION);
    assert!(encoded.len() > body.as_str().len());
    assert!(body.as_str().contains("[姓名已占位]"));
    assert!(encoded.contains("\\n"));
    assert!(encoded.contains("\\\""));
}
