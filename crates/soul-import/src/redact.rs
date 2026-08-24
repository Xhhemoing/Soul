//! Keeping what someone wrote out of the error message that rejects it.
//!
//! An import failure has to be readable — the user has a file that will not
//! load and needs to know which line and which field — but it must not quote
//! the file back. Two things push in that direction: PRODUCT_LOCK's rule that
//! third-party prose stays put, and the plainer one that an error message ends
//! up in logs and bug reports.
//!
//! The usual sources of an echo are library error strings. `jsonschema`
//! renders the offending instance into its `Display`, and `serde` names an
//! unexpected enum value. Rather than trusting each call site to remember,
//! every message this crate emits passes through a [`ContentGuard`] that knows
//! the document's content fields and replaces anything that repeats a run of
//! them.
//!
//! The run length is short on purpose. The leakage checker in `soul-testkit`
//! uses eight scalars because it is looking at prose leaving the machine; here
//! the bar is four, because an error message has no business repeating even a
//! fragment.

use std::collections::BTreeSet;

use serde_json::Value;

/// Shortest run of a content field that counts as an echo.
pub const MIN_ECHOED_RUN: usize = 4;

/// What replaces a message that echoed content.
pub const GENERIC_REASON: &str = "the value does not satisfy the contract";

/// Remembers the content fields of one document and refuses messages that
/// repeat them.
#[derive(Debug, Clone, Default)]
pub struct ContentGuard {
    secrets: Vec<Vec<char>>,
}

impl ContentGuard {
    pub fn new() -> ContentGuard {
        ContentGuard::default()
    }

    /// Collect every string found under one of `content_keys`, at any depth.
    ///
    /// The key list is per-format and deliberately explicit: it names the
    /// fields that hold prose, names and account handles, and nothing else, so
    /// that ids, timestamps and enum values stay available to error messages.
    pub fn from_document(document: &Value, content_keys: &[&str]) -> ContentGuard {
        let mut guard = ContentGuard::new();
        guard.absorb(document, content_keys, false);
        guard
    }

    fn absorb(&mut self, value: &Value, content_keys: &[&str], inside_content: bool) {
        match value {
            Value::String(text) if inside_content => self.learn(text),
            Value::Object(map) => {
                for (key, nested) in map {
                    let is_content = inside_content || content_keys.contains(&key.as_str());
                    self.absorb(nested, content_keys, is_content);
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.absorb(item, content_keys, inside_content);
                }
            }
            _ => {}
        }
    }

    /// Treat one string as content that must never be repeated back.
    pub fn learn(&mut self, text: &str) {
        let scalars: Vec<char> = text.chars().collect();
        if scalars.len() >= MIN_ECHOED_RUN {
            self.secrets.push(scalars);
        }
    }

    pub fn secret_count(&self) -> usize {
        self.secrets.len()
    }

    /// Does `message` repeat [`MIN_ECHOED_RUN`] or more scalars of any content
    /// field?
    pub fn echoes_content(&self, message: &str) -> bool {
        if self.secrets.is_empty() {
            return false;
        }
        let haystack: String = message.chars().collect();
        self.secrets.iter().any(|secret| {
            secret
                .windows(MIN_ECHOED_RUN)
                .any(|window| haystack.contains(&window.iter().collect::<String>()))
        })
    }

    /// `message`, or [`GENERIC_REASON`] if it echoed content.
    pub fn guard(&self, message: impl Into<String>) -> String {
        let message = message.into();
        match self.echoes_content(&message) {
            true => GENERIC_REASON.to_owned(),
            false => message,
        }
    }
}

/// Field names, truncated and capped, for an `additionalProperties` message.
///
/// A key is not prose, but a hostile file can put prose in a key, so the
/// rendering is bounded before the guard ever sees it.
pub fn summarize_names(names: &[String]) -> String {
    const MAX_NAMES: usize = 3;
    const MAX_LEN: usize = 40;
    let shown: BTreeSet<String> = names
        .iter()
        .take(MAX_NAMES)
        .map(|name| match name.chars().count() > MAX_LEN {
            true => format!("{}…", name.chars().take(MAX_LEN).collect::<String>()),
            false => name.clone(),
        })
        .collect();
    let joined = shown
        .into_iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    match names.len() > MAX_NAMES {
        true => format!("{joined} and {} more", names.len() - MAX_NAMES),
        false => joined,
    }
}
