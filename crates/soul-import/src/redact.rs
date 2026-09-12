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
//! There are two bars, because there are two kinds of string.
//!
//! A **fragment** is something lifted out of the file — a property name the
//! contract does not define, say. Four scalars of content in one is an echo,
//! and the fragment is dropped. That is a shorter run than the leakage checker
//! in `soul-testkit` uses, because a fragment has no business repeating
//! anything at all.
//!
//! A **sentence** is one this crate composed out of its own vocabulary, with
//! any fragments already guarded. Holding it to four scalars turns out to
//! punish the reader rather than the attacker: a file whose text happens to
//! mention `RFC 3339` would silence the very sentence that explains what is
//! wrong with it. A sentence is refused when it repeats [`MIN_QUOTED_RUN`]
//! scalars, which fixed vocabulary does not reach by accident and a quotation
//! does immediately.

use std::collections::BTreeSet;

use serde_json::Value;

/// Shortest run of a content field that counts as an echo in a fragment.
pub const MIN_ECHOED_RUN: usize = 4;

/// Shortest run that counts as a quotation in a composed sentence.
pub const MIN_QUOTED_RUN: usize = 12;

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

    /// Does `message` repeat `run` or more scalars of any content field?
    pub fn repeats_run(&self, message: &str, run: usize) -> bool {
        self.secrets.iter().any(|secret| {
            secret.len() >= run
                && secret
                    .windows(run)
                    .any(|window| message.contains(&window.iter().collect::<String>()))
        })
    }

    /// Does `message` echo a content field closely enough for a fragment?
    pub fn echoes_content(&self, message: &str) -> bool {
        self.repeats_run(message, MIN_ECHOED_RUN)
    }

    /// Does `message` quote a content field?
    pub fn quotes_content(&self, message: &str) -> bool {
        self.repeats_run(message, MIN_QUOTED_RUN)
    }

    /// A string taken out of the file, or nothing if it echoes content.
    pub fn guard_fragment(&self, fragment: &str) -> Option<String> {
        match self.echoes_content(fragment) {
            true => None,
            false => Some(fragment.to_owned()),
        }
    }

    /// A composed sentence, or [`GENERIC_REASON`] if it quotes content.
    pub fn guard(&self, message: impl Into<String>) -> String {
        let message = message.into();
        match self.quotes_content(&message) {
            true => GENERIC_REASON.to_owned(),
            false => message,
        }
    }
}

/// Field names, truncated, capped and guarded, for a message about fields the
/// contract does not define.
///
/// A key is not prose, but a hostile file can put prose in a key, so the
/// rendering is bounded and then each name is guarded as the fragment it is.
/// A name that echoed content is dropped, which is why the count at the end
/// counts the names rather than what is shown.
pub fn summarize_names(names: &[String], guard: &ContentGuard) -> String {
    const MAX_NAMES: usize = 3;
    const MAX_LEN: usize = 40;
    let shown: BTreeSet<String> = names
        .iter()
        .take(MAX_NAMES)
        .map(|name| match name.chars().count() > MAX_LEN {
            true => format!("{}…", name.chars().take(MAX_LEN).collect::<String>()),
            false => name.clone(),
        })
        .filter_map(|name| guard.guard_fragment(&name))
        .collect();
    if shown.is_empty() {
        return format!("{} field(s) it does not define", names.len());
    }
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
