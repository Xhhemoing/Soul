//! PRODUCT_LOCK: *出站消息 v0.1 只起草，不发送*.
//!
//! The rest of the suite checks what a draft says. This one checks what a
//! draft *is*, because the promise not to send is not a behaviour that could
//! be tested by observing one run — it is the absence of a capability, and
//! the only way to test an absence is to pin the surface it would have to
//! appear on.
//!
//! Two surfaces are pinned here. The serialized [`Draft`] the shell receives:
//! `DRAFT_FIELDS` lists every key, `deny_unknown_fields` makes the list
//! two-way, and this file asserts that no key reads like a recipient or an
//! action. And [`NeverSent`], which has one inhabitant and refuses to
//! deserialize from `true`, so a `delivery: true` arriving from anywhere is a
//! parse error rather than a state.
//!
//! When someone adds a field to `Draft`, one of the assertions below fails.
//! That is the point: the failure is a prompt to ask whether the new field is
//! the beginning of a send button.

use serde_json::Value;

use soul_draft::draft::{self, Draft, NeverSent, DRAFT_FIELDS, NOT_SENT_NOTICE};
use soul_draft::{DraftRequest, DraftSource, Drafter, ProfileBrief};
use soul_policy::redactor::{KnownIdentifiers, Redactor};

fn a_draft() -> Draft {
    Drafter::new(Redactor::new(KnownIdentifiers::new()), "local-model")
        .draft_offline(&DraftRequest::from_paste(
            ProfileBrief::neutral(),
            "周五的安排你看行吗",
        ))
        .expect("a draft")
}

/// Words that would mean this value can leave the machine on the user's
/// behalf, or that something acted on it.
const FORBIDDEN_IN_A_FIELD_NAME: &[&str] = &[
    "send",
    "sent_to",
    "deliver",
    "recipient",
    "to",
    "address",
    "channel",
    "chat_id",
    "phone",
    "email",
    "account",
    "execute",
    "confirm",
    "post",
    "publish",
    "reply_to",
    "webhook",
    "endpoint",
    "url",
];

#[test]
fn no_field_on_a_draft_could_name_somewhere_to_send_it() {
    let value = draft::to_value(&a_draft());
    let object = value.as_object().expect("a draft is an object");

    // `serde_json`'s map is ordered, so compare as sets and let the message
    // name whichever side is missing something.
    let mut present: Vec<&str> = object.keys().map(String::as_str).collect();
    let mut declared: Vec<&str> = DRAFT_FIELDS.to_vec();
    present.sort_unstable();
    declared.sort_unstable();
    assert_eq!(
        present, declared,
        "DRAFT_FIELDS is supposed to be the whole surface",
    );

    for key in object.keys() {
        for forbidden in FORBIDDEN_IN_A_FIELD_NAME {
            let is_hit = key == forbidden
                || key.starts_with(&format!("{forbidden}_"))
                || key.ends_with(&format!("_{forbidden}"));
            assert!(
                !is_hit,
                "`{key}` reads like a way to send a draft; \
                 if it is not, rename it, and if it is, this is the wrong crate",
            );
        }
    }
}

#[test]
fn delivery_is_false_and_there_is_no_other_value_for_it() {
    let value = draft::to_value(&a_draft());
    assert_eq!(value["delivery"], Value::Bool(false));

    // The type has one inhabitant, so this is not a check that could be
    // spelled wrong somewhere else in the codebase.
    assert_eq!(
        serde_json::to_value(NeverSent).expect("serializes"),
        Value::Bool(false),
    );
    serde_json::from_value::<NeverSent>(Value::Bool(false)).expect("false round-trips");
    serde_json::from_value::<NeverSent>(Value::Bool(true))
        .expect_err("a draft claiming it was delivered must not parse");
}

#[test]
fn a_draft_arriving_with_an_extra_field_does_not_parse() {
    let mut value = draft::to_value(&a_draft());
    value
        .as_object_mut()
        .expect("an object")
        .insert("sent_to".into(), Value::String("+8613800138000".into()));

    serde_json::from_value::<Draft>(value)
        .expect_err("deny_unknown_fields is the other half of DRAFT_FIELDS");
}

#[test]
fn every_draft_tells_the_user_it_was_not_sent() {
    let draft = a_draft();
    assert_eq!(draft.not_sent_notice, NOT_SENT_NOTICE);
    assert!(
        NOT_SENT_NOTICE.contains("不会替你发送"),
        "the notice has to say the thing, not just exist",
    );

    // Both paths carry it, so the sentence is not a property of the offline
    // branch that the endpoint branch forgot.
    let from_endpoint = Drafter::new(Redactor::new(KnownIdentifiers::new()), "local-model")
        .draft_with(
            &DraftRequest::from_paste(ProfileBrief::neutral(), "周五的安排你看行吗"),
            None,
            &mut Canned(
                serde_json::json!({
                    "choices": [{ "message": { "role": "assistant", "content": "周五我可以，几点都行。" } }]
                })
                .to_string(),
            ),
        )
        .expect("a draft");
    assert_eq!(from_endpoint.source, DraftSource::UserEndpoint);
    assert_eq!(from_endpoint.not_sent_notice, NOT_SENT_NOTICE);
}

#[derive(Debug)]
struct Canned(String);

impl soul_draft::ReplyGenerator for Canned {
    fn generate(
        &mut self,
        _body: soul_policy::RedactedBody,
    ) -> Result<String, soul_draft::GenerationRefused> {
        Ok(self.0.clone())
    }
}
