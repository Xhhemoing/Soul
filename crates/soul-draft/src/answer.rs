//! What the endpoint said, read as data.
//!
//! An OpenAI-compatible answer is JSON that a model wrote, which puts it in
//! the same class as a pasted message and an imported line: it may be read and
//! it may be shown, and it may not be parsed into evidence, into an action, or
//! into anything the product then does. That is why this returns
//! [`UntrustedText`] and reaches for exactly one field. Nothing here looks at
//! `tool_calls`, and nothing anywhere else does either.

use serde_json::Value;

use soul_policy::injection::UntrustedText;

use crate::error::{DraftError, DraftResult};

/// The assistant text of the first choice.
///
/// An answer that does not have that shape is a readable failure rather than
/// an empty draft: "the endpoint said something this build cannot read" and
/// "the endpoint had nothing to say" are different facts and the user is
/// entitled to know which one happened.
pub fn answer_text(answer_body: &str) -> DraftResult<UntrustedText> {
    let value: Value =
        serde_json::from_str(answer_body).map_err(|_| DraftError::UnreadableAnswer)?;
    let text = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .ok_or(DraftError::UnreadableAnswer)?;
    Ok(UntrustedText::new(text))
}
