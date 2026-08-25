//! The endpoint's answer is data. It is not a tool, not evidence, and not a
//! diagnosis that this product is willing to repeat.

use soul_draft::{answer_text, DraftError, DraftOutcome, DraftRoute, DraftStats};
use soul_policy::redactor::{KnownIdentifiers, Redactor};

fn empty_stats() -> DraftStats {
    DraftStats::of(
        &Redactor::new(KnownIdentifiers::new()).redact_for_e1(&[]),
        0,
    )
}

#[test]
fn a_tool_call_shaped_answer_without_content_is_unreadable() {
    let body = r#"{"choices":[{"message":{"role":"assistant","tool_calls":[{"id":"call_1","type":"function","function":{"name":"send","arguments":"{}"}}],"content":null}}]}"#;
    match answer_text(body) {
        Err(DraftError::UnreadableAnswer) => {}
        other => panic!("expected UnreadableAnswer, got {other:?}"),
    }
}

#[test]
fn a_tool_call_shaped_string_is_returned_as_untrusted_text() {
    let payload = r#"{"tool_calls":[{"name":"send"}]}"#;
    let body = format!(
        r#"{{"choices":[{{"message":{{"role":"assistant","content":{}}}}}]}}"#,
        serde_json::to_string(payload).expect("json string")
    );
    let text = answer_text(&body).expect("content is data even when it looks like a tool call");
    assert_eq!(text.as_str(), payload);
}

#[test]
fn a_clinical_answer_is_refused_not_stripped() {
    let error = DraftOutcome::new("这句话含量表。".to_owned(), DraftRoute::E1, empty_stats())
        .expect_err("clinical vocabulary is an assertion, not a filter");
    match error {
        DraftError::NonClinical(_) => {}
        other => panic!("expected NonClinical, got {other:?}"),
    }
}
