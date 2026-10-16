//! Bounded, local selection from a scope the user explicitly chose for one draft.
//!
//! The caller is the trusted use-case adapter: candidate ids and limits are
//! not grants supplied by imported prose. This module never lists the store,
//! mints persistent consent, approves egress, or writes an inferred fact.
//! A citation identifies a saved memory and its two readable sealed fields;
//! it does not prove that the words are true, current, or mutually consistent.

use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;
use uuid::Uuid;

use soul_schema::common::{field_aad, SealedSubject};
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_store_api::types::StoreError;
use soul_store_api::{BlobStore, MemoryStore};

use crate::{SUMMARY_FIELD, TITLE_FIELD};

pub const MEMORY_CONTEXT_NOTICE: &str = "仅在你本次允许的记忆范围内选择。这些是保存的记录，不代表事实已核实、仍然有效或不存在冲突；没有核实事实时间，也没有自动识别语义冲突。关联外部来源的记忆暂不支持。选择只用于本次本地起草参考，不授予研究许可或端点原文出网许可。";

/// An explicit request for a time-specific fact cannot be satisfied by the
/// current memory schema. A UUID timestamp is not a fact's validity time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MemoryTimeScope {
    #[default]
    Unspecified,
    AtTime,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct MemorySelectionRequest {
    pub candidate_ids: Vec<Uuid>,
    pub query: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub time_scope: MemoryTimeScope,
}

impl fmt::Debug for MemorySelectionRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MemorySelectionRequest")
            .field("candidate_ids", &self.candidate_ids)
            .field("has_query", &self.query.is_some())
            .field("memory_type", &self.memory_type)
            .field("time_scope", &self.time_scope)
            .finish()
    }
}

/// Resource budgets for this new use case, supplied only by trusted core.
/// These are not T4D/A0/A1/A2/A3 algorithm parameters or stored permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionLimits {
    pub max_candidates: usize,
    pub max_selected: usize,
    pub max_query_chars: usize,
    pub max_entry_bytes: usize,
    /// Exact UTF-8 JSON bytes of the selected preview array, including
    /// citations, punctuation, and JSON escaping. Not a token budget or the
    /// final HTTP budget: the use-case adapter must also check that payload.
    pub max_context_bytes: usize,
}

impl Default for SelectionLimits {
    fn default() -> Self {
        Self {
            max_candidates: 32,
            max_selected: 8,
            max_query_chars: 256,
            max_entry_bytes: 16 * 1024,
            max_context_bytes: 16 * 1024,
        }
    }
}

impl SelectionLimits {
    fn validate(self) -> Result<(), SelectionError> {
        if self.max_candidates == 0
            || self.max_selected == 0
            || self.max_selected > self.max_candidates
            || self.max_query_chars == 0
            || self.max_entry_bytes == 0
            || self.max_context_bytes < 2
            || self.max_entry_bytes > self.max_context_bytes
        {
            return Err(SelectionError::InvalidLimits);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPrivacyClass {
    /// Both sealed fields identify the owner and the record explicitly says
    /// no third-party content. This still does not grant any egress.
    Owner,
    ThirdParty,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemoryCitation {
    pub memory_id: Uuid,
    pub title_blob_id: Uuid,
    pub summary_blob_id: Uuid,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct SelectedMemoryPreview {
    pub memory_id: Uuid,
    pub memory_type: MemoryType,
    pub title: String,
    pub summary: String,
    pub privacy_class: MemoryPrivacyClass,
    pub citation: MemoryCitation,
}

impl fmt::Debug for SelectedMemoryPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectedMemoryPreview")
            .field("memory_id", &self.memory_id)
            .field("memory_type", &self.memory_type)
            .field("privacy_class", &self.privacy_class)
            .field("citation", &self.citation)
            .finish_non_exhaustive()
    }
}

/// A readable selection and its exact row snapshot. Deliberately not
/// serializable: only an explicit local preview may cross the IPC boundary.
#[derive(Clone, PartialEq, Eq)]
pub struct SelectedMemory {
    snapshot: SoulMemory,
    preview: SelectedMemoryPreview,
}

impl fmt::Debug for SelectedMemory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SelectedMemory")
            .field("memory_id", &self.memory_id())
            .field("citation", self.citation())
            .field("privacy_class", &self.privacy_class())
            .finish_non_exhaustive()
    }
}

impl SelectedMemory {
    pub fn memory_id(&self) -> Uuid {
        self.snapshot.memory_id
    }

    pub fn memory_type(&self) -> MemoryType {
        self.snapshot.memory_type
    }

    pub fn title(&self) -> &str {
        &self.preview.title
    }

    pub fn summary(&self) -> &str {
        &self.preview.summary
    }

    pub fn privacy_class(&self) -> MemoryPrivacyClass {
        self.preview.privacy_class
    }

    pub fn citation(&self) -> &MemoryCitation {
        &self.preview.citation
    }

    pub fn preview(&self) -> SelectedMemoryPreview {
        self.preview.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionOutcome {
    Context,
    Abstain,
    Clarify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    DuplicateId,
    NotFound,
    Inactive,
    UnsupportedLinkedSources,
    MissingSealedFields,
    InvalidSealedReference,
    Unreadable,
    NotUtf8,
    EmptyContent,
    EntryBudget,
    TypeMismatch,
    KeywordMismatch,
    SelectionLimit,
    ContextBudget,
    TemporalScopeUnavailable,
    FinalRequestBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectionExclusion {
    pub memory_id: Uuid,
    pub reason: ExclusionReason,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct MemorySelectionPreview {
    pub selected: Vec<SelectedMemoryPreview>,
    pub excluded: Vec<SelectionExclusion>,
    pub outcome: SelectionOutcome,
    pub notice: String,
}

impl fmt::Debug for MemorySelectionPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MemorySelectionPreview")
            .field("selected", &self.selected)
            .field("excluded", &self.excluded)
            .field("outcome", &self.outcome)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSelection {
    selected: Vec<SelectedMemory>,
    excluded: Vec<SelectionExclusion>,
    outcome: SelectionOutcome,
}

impl ContextSelection {
    pub fn selected(&self) -> &[SelectedMemory] {
        &self.selected
    }

    pub fn excluded(&self) -> &[SelectionExclusion] {
        &self.excluded
    }

    pub fn outcome(&self) -> SelectionOutcome {
        self.outcome
    }

    pub fn preview(&self) -> MemorySelectionPreview {
        MemorySelectionPreview {
            selected: self.selected.iter().map(SelectedMemory::preview).collect(),
            excluded: self.excluded.clone(),
            outcome: self.outcome,
            notice: MEMORY_CONTEXT_NOTICE.to_owned(),
        }
    }

    /// The actual serialized selected-context array, not a sum of estimated
    /// per-item character counts. This excludes local-only exclusion notices.
    pub fn serialized_context_bytes(&self) -> Result<usize, SelectionError> {
        context_bytes(&self.selected)
    }

    /// Synchronize the local snapshot/preview with the complete entries that
    /// fit the final serialized endpoint request. Only an existing selected
    /// subset is accepted; this cannot expand the user's original scope.
    pub fn retain_ids(&mut self, retained_ids: &[Uuid]) -> Result<(), SelectionError> {
        let retained: BTreeSet<_> = retained_ids.iter().copied().collect();
        if retained.len() != retained_ids.len()
            || retained
                .iter()
                .any(|id| !self.selected.iter().any(|entry| entry.memory_id() == *id))
        {
            return Err(SelectionError::InvalidRetainedIds);
        }
        self.selected.retain(|entry| {
            if retained.contains(&entry.memory_id()) {
                true
            } else {
                self.excluded.push(SelectionExclusion {
                    memory_id: entry.memory_id(),
                    reason: ExclusionReason::FinalRequestBudget,
                });
                false
            }
        });
        if self.outcome != SelectionOutcome::Clarify {
            self.outcome = if self.selected.is_empty() {
                SelectionOutcome::Abstain
            } else {
                SelectionOutcome::Context
            };
        }
        Ok(())
    }
}

/// Failure codes contain no query, title, summary, or backend diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SelectionError {
    #[error("invalid memory selection resource limits")]
    InvalidLimits,
    #[error("the explicitly permitted candidate scope exceeds the limit")]
    TooManyCandidates,
    #[error("the local memory query exceeds the limit")]
    QueryTooLong,
    #[error("the memory store could not be read")]
    StoreUnavailable,
    #[error("the selected memory changed or is no longer readable")]
    SourceChanged,
    #[error("the selected context could not be serialized")]
    Serialization,
    #[error("retained ids must be a unique subset of this selection")]
    InvalidRetainedIds,
}

/// Resolve only ids explicitly authorized for this one local draft. Manual
/// selection preserves order; keyword selection ranks title hits first and
/// then stable ids, without claiming semantic or counterevidence recall.
pub fn select_for_draft<S>(
    store: &S,
    request: &MemorySelectionRequest,
    limits: SelectionLimits,
) -> Result<ContextSelection, SelectionError>
where
    S: MemoryStore + BlobStore,
{
    limits.validate()?;
    if request.candidate_ids.len() > limits.max_candidates {
        return Err(SelectionError::TooManyCandidates);
    }
    if request
        .query
        .as_ref()
        .is_some_and(|query| query.chars().count() > limits.max_query_chars)
    {
        return Err(SelectionError::QueryTooLong);
    }

    let mut result = ContextSelection {
        selected: Vec::new(),
        excluded: Vec::new(),
        outcome: SelectionOutcome::Abstain,
    };
    let mut seen = BTreeSet::new();
    if request.time_scope == MemoryTimeScope::AtTime {
        for id in &request.candidate_ids {
            if seen.insert(*id) {
                exclude(&mut result, *id, ExclusionReason::TemporalScopeUnavailable);
            }
        }
        result.outcome = SelectionOutcome::Clarify;
        return Ok(result);
    }

    let tokens: Vec<String> = request
        .query
        .as_deref()
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    let mut candidates = Vec::new();
    for id in &request.candidate_ids {
        if !seen.insert(*id) {
            exclude(&mut result, *id, ExclusionReason::DuplicateId);
            continue;
        }
        let memory = match store.get_memory(*id) {
            Ok(memory) => memory,
            Err(StoreError::NotFound { .. }) => {
                exclude(&mut result, *id, ExclusionReason::NotFound);
                continue;
            }
            Err(_) => return Err(SelectionError::StoreUnavailable),
        };
        if memory.memory_id != *id {
            exclude(&mut result, *id, ExclusionReason::InvalidSealedReference);
            continue;
        }
        if request
            .memory_type
            .is_some_and(|kind| memory.memory_type != kind)
        {
            exclude(&mut result, *id, ExclusionReason::TypeMismatch);
            continue;
        }
        let entry = match resolve(store, memory, limits.max_entry_bytes) {
            Ok(entry) => entry,
            Err(ResolveFailure::Exclude(reason)) => {
                exclude(&mut result, *id, reason);
                continue;
            }
            Err(ResolveFailure::StoreUnavailable) => {
                return Err(SelectionError::StoreUnavailable);
            }
        };
        let title = entry.title().to_lowercase();
        let summary = entry.summary().to_lowercase();
        if !tokens
            .iter()
            .all(|token| title.contains(token.as_str()) || summary.contains(token.as_str()))
        {
            exclude(&mut result, *id, ExclusionReason::KeywordMismatch);
            continue;
        }
        let title_hits = tokens
            .iter()
            .filter(|token| title.contains(token.as_str()))
            .count();
        candidates.push((title_hits, entry));
    }
    if !tokens.is_empty() {
        candidates.sort_by(|(left_hits, left), (right_hits, right)| {
            right_hits
                .cmp(left_hits)
                .then_with(|| left.memory_id().cmp(&right.memory_id()))
        });
    }
    for (_, entry) in candidates {
        let id = entry.memory_id();
        if result.selected.len() >= limits.max_selected {
            exclude(&mut result, id, ExclusionReason::SelectionLimit);
            continue;
        }
        result.selected.push(entry);
        if result.serialized_context_bytes()? > limits.max_context_bytes {
            result.selected.pop();
            exclude(&mut result, id, ExclusionReason::ContextBudget);
        }
    }
    if !result.selected.is_empty() {
        result.outcome = SelectionOutcome::Context;
    }
    Ok(result)
}

/// Recheck the same selected rows while the use-case adapter holds its store
/// guard, immediately before preparation/dispatch. The Session's revision
/// still guards late results and all other source writes across async work.
pub fn revalidate_selection<S>(
    store: &S,
    selection: &ContextSelection,
) -> Result<(), SelectionError>
where
    S: MemoryStore + BlobStore,
{
    for selected in &selection.selected {
        let current = store
            .get_memory(selected.memory_id())
            .map_err(|_| SelectionError::SourceChanged)?;
        if current != selected.snapshot {
            return Err(SelectionError::SourceChanged);
        }
        let resolved = resolve(store, current, usize::MAX)
            .map_err(|_| SelectionError::SourceChanged)?;
        if resolved.preview != selected.preview {
            return Err(SelectionError::SourceChanged);
        }
    }
    Ok(())
}

fn exclude(result: &mut ContextSelection, memory_id: Uuid, reason: ExclusionReason) {
    result
        .excluded
        .push(SelectionExclusion { memory_id, reason });
}

fn context_bytes(entries: &[SelectedMemory]) -> Result<usize, SelectionError> {
    let preview: Vec<_> = entries.iter().map(|entry| &entry.preview).collect();
    serde_json::to_vec(&preview)
        .map(|bytes| bytes.len())
        .map_err(|_| SelectionError::Serialization)
}

enum ResolveFailure {
    Exclude(ExclusionReason),
    StoreUnavailable,
}

fn resolve<S: BlobStore>(
    store: &S,
    memory: SoulMemory,
    max_entry_bytes: usize,
) -> Result<SelectedMemory, ResolveFailure> {
    let reject = |reason| ResolveFailure::Exclude(reason);
    if memory.forget_state != ForgetState::Active {
        return Err(reject(ExclusionReason::Inactive));
    }
    if memory
        .source_event_ids
        .as_ref()
        .is_some_and(|ids| !ids.is_empty())
        || memory
            .evidence_ids
            .as_ref()
            .is_some_and(|ids| !ids.is_empty())
    {
        return Err(reject(ExclusionReason::UnsupportedLinkedSources));
    }
    let (Some(title_ref), Some(summary_ref)) = (&memory.title_ref, &memory.summary_ref) else {
        return Err(reject(ExclusionReason::MissingSealedFields));
    };
    if title_ref.content_key_id != memory.content_key_id
        || summary_ref.content_key_id != memory.content_key_id
        || title_ref.blob_id == summary_ref.blob_id
        || title_ref.aad.as_deref()
            != Some(field_aad(&memory.memory_id, TITLE_FIELD).as_str())
        || summary_ref.aad.as_deref()
            != Some(field_aad(&memory.memory_id, SUMMARY_FIELD).as_str())
    {
        return Err(reject(ExclusionReason::InvalidSealedReference));
    }
    // A character consumes at least one UTF-8 byte. This avoids opening a
    // correctly stored oversized field; the exact byte check follows open.
    let chars = title_ref.char_count.checked_add(summary_ref.char_count);
    if chars.is_none_or(|chars| u128::from(chars) > max_entry_bytes as u128) {
        return Err(reject(ExclusionReason::EntryBudget));
    }
    let title = open_utf8(store, title_ref)?;
    let summary = open_utf8(store, summary_ref)?;
    if title.trim().is_empty() || summary.trim().is_empty() {
        return Err(reject(ExclusionReason::EmptyContent));
    }
    if title
        .len()
        .checked_add(summary.len())
        .is_none_or(|bytes| bytes > max_entry_bytes)
    {
        return Err(reject(ExclusionReason::EntryBudget));
    }
    let privacy_class = match (title_ref.subject, summary_ref.subject) {
        (SealedSubject::Owner, SealedSubject::Owner)
            if memory.third_party_content_present == Some(false) =>
        {
            MemoryPrivacyClass::Owner
        }
        (SealedSubject::ThirdParty, SealedSubject::ThirdParty) => MemoryPrivacyClass::ThirdParty,
        (SealedSubject::Mixed, _) | (_, SealedSubject::Mixed) => MemoryPrivacyClass::Mixed,
        (SealedSubject::Owner, SealedSubject::ThirdParty)
        | (SealedSubject::ThirdParty, SealedSubject::Owner) => MemoryPrivacyClass::Mixed,
        _ => MemoryPrivacyClass::Unknown,
    };
    let preview = SelectedMemoryPreview {
        memory_id: memory.memory_id,
        memory_type: memory.memory_type,
        title,
        summary,
        privacy_class,
        citation: MemoryCitation {
            memory_id: memory.memory_id,
            title_blob_id: title_ref.blob_id,
            summary_blob_id: summary_ref.blob_id,
        },
    };
    Ok(SelectedMemory {
        snapshot: memory,
        preview,
    })
}

fn open_utf8<S: BlobStore>(
    store: &S,
    sealed: &soul_schema::common::SealedText,
) -> Result<String, ResolveFailure> {
    let bytes = store.open(sealed).map_err(|error| match error {
        StoreError::Backend(_) => ResolveFailure::StoreUnavailable,
        _ => ResolveFailure::Exclude(ExclusionReason::Unreadable),
    })?;
    String::from_utf8(bytes)
        .map_err(|_| ResolveFailure::Exclude(ExclusionReason::NotUtf8))
}
