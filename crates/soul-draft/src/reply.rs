//! Reading what the endpoint sent back — as data, and only as data.
//!
//! DECISIONS D25 says external content is never instruction, and a model's
//! response is external content: the user's endpoint is theirs, but the bytes
//! coming out of it were shaped by whatever went in, injected material
//! included. So this module parses, checks, and returns a string. It has no
//! notion of a tool call, no branch that acts on one, and nothing downstream
//! takes an action because of what a reply says.
//!
//! [`ModelReply::signals`] exists for the audit trail alone. It records what
//! the returned text *tried* to do so `injection.blocked` can say something
//! true, and nothing reads it back. A scanner that were load-bearing would be
//! a filter, and a filter can be evaded.

use serde_json::Value;

use soul_policy::clinical::{assert_non_clinical, NonClinicalViolation};
use soul_policy::injection::{self, InjectionSignal, UntrustedText};

/// Model text that has been read as data and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReply {
    pub text: String,
    /// What the reply attempted. Recorded for the audit entry, never obeyed.
    pub signals: Vec<InjectionSignal>,
}

/// Why a reply cannot be shown to the user.
///
/// None of these carry the endpoint's own words. A hostile endpoint controls
/// what it says, and a defect that quoted it would carry that text into a log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplyDefect {
    #[error("the endpoint did not answer with JSON")]
    NotJson,
    #[error("the answer has no assistant message in it")]
    NoMessage,
    #[error("the assistant message is empty")]
    Empty,
    #[error("the answer carries vocabulary this product must not use: {0}")]
    Clinical(#[from] NonClinicalViolation),
}

/// Pull the assistant's text out of an OpenAI-compatible response.
///
/// Both spellings are accepted because "OpenAI-compatible" is a family rather
/// than a specification: `choices[].message.content` is the chat shape and
/// `choices[].text` is the completion shape, and a user pointing Soul at their
/// own server should not have to care which one it speaks.
pub fn read(raw: &str) -> Result<ModelReply, ReplyDefect> {
    let value: Value = serde_json::from_str(raw).map_err(|_| ReplyDefect::NotJson)?;
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(ReplyDefect::NoMessage)?;

    let text = choice
        .pointer("/message/content")
        .or_else(|| choice.get("text"))
        .and_then(Value::as_str)
        .ok_or(ReplyDefect::NoMessage)?
        .trim()
        .to_owned();

    if text.is_empty() {
        return Err(ReplyDefect::Empty);
    }
    assert_non_clinical(&text)?;

    let signals = injection::scan(&UntrustedText::new(&text));
    Ok(ModelReply { text, signals })
}
