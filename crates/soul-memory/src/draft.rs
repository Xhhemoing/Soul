//! What a caller hands in, and what it gets back.
//!
//! A draft carries plaintext because that is what the user typed; it exists
//! only for the duration of the call that seals it. Everything that persists
//! is a [`soul_schema::common::SealedText`] pointer, which is what makes
//! forgetting a matter of destroying one key rather than hunting for copies.

use soul_schema::common::SealedSubject;
use soul_schema::memory::{ForgetState, MemoryType};
use uuid::Uuid;

use crate::error::{MemoryError, MemoryResult};

/// Column names for the two sealed fields.
///
/// They are bound into the AEAD tag alongside the memory id, so a sealed title
/// cannot be replayed into the summary column. The spelling has to match
/// `memory.schema.json`, which is why these are constants rather than literals
/// at the call site.
pub const TITLE_FIELD: &str = "title_ref";
pub const SUMMARY_FIELD: &str = "summary_ref";

/// What the UI and any E1 request body show in place of third-party prose.
pub const DEFAULT_PLACEHOLDER: &str = "[第三人内容已占位]";

/// A memory the user is about to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDraft {
    pub memory_type: MemoryType,
    pub title: String,
    pub summary: String,
    /// Who the prose is about. Anything other than `self` is third-party
    /// content and travels with a placeholder.
    pub subject: SealedSubject,
    pub source_event_ids: Vec<Uuid>,
    pub evidence_ids: Vec<Uuid>,
    /// Overrides [`DEFAULT_PLACEHOLDER`] for a draft that wants its own.
    pub placeholder: Option<String>,
}

impl MemoryDraft {
    /// A memory about the user, with nobody else's words in it.
    pub fn own(
        memory_type: MemoryType,
        title: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        MemoryDraft {
            memory_type,
            title: title.into(),
            summary: summary.into(),
            subject: SealedSubject::Owner,
            source_event_ids: Vec::new(),
            evidence_ids: Vec::new(),
            placeholder: None,
        }
    }

    /// Mark the prose as carrying someone else's words.
    pub fn about(mut self, subject: SealedSubject) -> Self {
        self.subject = subject;
        self
    }

    pub fn from_events(mut self, event_ids: &[Uuid]) -> Self {
        self.source_event_ids = event_ids.to_vec();
        self
    }

    pub fn citing(mut self, evidence_ids: &[Uuid]) -> Self {
        self.evidence_ids = evidence_ids.to_vec();
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// `true` unless the prose is the user's own.
    pub fn third_party_content_present(&self) -> bool {
        !matches!(self.subject, SealedSubject::Owner)
    }

    /// The placeholder this draft's sealed fields carry, if any.
    ///
    /// Own prose gets none: a placeholder there would suggest the redactor has
    /// something to hide behind, and it does not.
    pub fn placeholder(&self) -> Option<String> {
        match self.third_party_content_present() {
            true => Some(
                self.placeholder
                    .clone()
                    .unwrap_or_else(|| DEFAULT_PLACEHOLDER.to_owned()),
            ),
            false => None,
        }
    }

    pub fn validate(&self) -> MemoryResult<()> {
        if self.title.trim().is_empty() {
            return Err(MemoryError::Empty("title"));
        }
        if self.summary.trim().is_empty() {
            return Err(MemoryError::Empty("summary"));
        }
        Ok(())
    }
}

/// A change to a stored memory. `None` leaves the field as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryEdit {
    pub memory_type: Option<MemoryType>,
    pub title: Option<String>,
    pub summary: Option<String>,
}

impl MemoryEdit {
    pub fn title(title: impl Into<String>) -> Self {
        MemoryEdit {
            title: Some(title.into()),
            ..MemoryEdit::default()
        }
    }

    pub fn summary(summary: impl Into<String>) -> Self {
        MemoryEdit {
            summary: Some(summary.into()),
            ..MemoryEdit::default()
        }
    }

    pub fn and_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn and_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn as_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.memory_type.is_none() && self.title.is_none() && self.summary.is_none()
    }

    pub fn validate(&self) -> MemoryResult<()> {
        if self.title.as_deref().is_some_and(|t| t.trim().is_empty()) {
            return Err(MemoryError::Empty("title"));
        }
        if self.summary.as_deref().is_some_and(|s| s.trim().is_empty()) {
            return Err(MemoryError::Empty("summary"));
        }
        Ok(())
    }
}

/// A memory with its prose opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryContent {
    pub memory: soul_schema::memory::SoulMemory,
    pub title: String,
    pub summary: String,
}

/// A memory without its prose: what a list view is allowed to show before the
/// user asks to open one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDigest {
    pub memory_id: Uuid,
    pub memory_type: MemoryType,
    pub forget_state: ForgetState,
    pub third_party_content_present: bool,
    /// Scalar counts, taken from the sealed pointers. Metadata, not text.
    pub title_chars: u64,
    pub summary_chars: u64,
}
