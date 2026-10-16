//! Separately selected memory material. It does not alter the user's profile
//! or the deterministic template, and local selection is not an E1 grant.

use std::collections::BTreeSet;
use std::fmt;

use uuid::Uuid;

use soul_policy::injection::{self, InjectionSignal, UntrustedText};
use soul_policy::redactor::{combine_redacted, RedactedBody, Redactor, Turn};
use soul_schema::common::SealedSubject;

use crate::draft::{DraftRequest, BRIEF_TURN_ID};

pub const LOCAL_MEMORY_PLACEHOLDER: &str = "[本次记忆仅供本地参考，正文未向端点提供]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointMemoryUse {
    LocalReferenceOnly,
    /// Only trusted core may derive this from a separate, explicit choice
    /// covering this exact selected owner snapshot. Final E1 approval remains
    /// mandatory and is issued outside this crate.
    OwnerExcerpt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryContextPrivacy {
    Owner,
    ThirdParty,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MemoryContextError {
    #[error("only explicitly classified owner memory can be selected for endpoint prose")]
    EndpointOwnerRequired,
    #[error("memory materials must have distinct local and turn identities")]
    DuplicateIdentity,
    #[error("memory turn identities must not collide with the brief or conversation")]
    TurnIdentityCollision,
    #[error("the original-turn exemption must name a conversation turn, never a memory")]
    ExemptionOutsideConversation,
    #[error("the redacted memory material could not be combined")]
    Composition,
}

#[derive(Clone, PartialEq, Eq)]
pub struct MemoryContextEntry {
    memory_id: Uuid,
    turn_id: Uuid,
    privacy: MemoryContextPrivacy,
    title: String,
    summary: String,
    endpoint_use: EndpointMemoryUse,
}

impl fmt::Debug for MemoryContextEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MemoryContextEntry")
            .field("memory_id", &self.memory_id)
            .field("turn_id", &self.turn_id)
            .field("privacy", &self.privacy)
            .field("endpoint_use", &self.endpoint_use)
            .finish_non_exhaustive()
    }
}

impl MemoryContextEntry {
    pub fn new(
        memory_id: Uuid,
        turn_id: Uuid,
        privacy: MemoryContextPrivacy,
        title: impl Into<String>,
        summary: impl Into<String>,
        endpoint_use: EndpointMemoryUse,
    ) -> Result<Self, MemoryContextError> {
        if endpoint_use == EndpointMemoryUse::OwnerExcerpt && privacy != MemoryContextPrivacy::Owner
        {
            return Err(MemoryContextError::EndpointOwnerRequired);
        }
        Ok(Self {
            memory_id,
            turn_id,
            privacy,
            title: title.into(),
            summary: summary.into(),
            endpoint_use,
        })
    }

    pub fn memory_id(&self) -> Uuid {
        self.memory_id
    }

    pub fn endpoint_use(&self) -> EndpointMemoryUse {
        self.endpoint_use
    }

    fn turn(&self) -> Turn {
        let (subject, material) = match self.endpoint_use {
            EndpointMemoryUse::OwnerExcerpt => (
                SealedSubject::Owner,
                format!(
                    "本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：\n{}\n{}",
                    self.title, self.summary
                ),
            ),
            EndpointMemoryUse::LocalReferenceOnly => {
                // The original subject is retained when known. For unknown
                // data no original words enter the turn: this is a fixed
                // system marker, not a reclassification of the record.
                let subject = match self.privacy {
                    MemoryContextPrivacy::ThirdParty => SealedSubject::ThirdParty,
                    MemoryContextPrivacy::Mixed => SealedSubject::Mixed,
                    MemoryContextPrivacy::Owner | MemoryContextPrivacy::Unknown => {
                        SealedSubject::Owner
                    }
                };
                (subject, LOCAL_MEMORY_PLACEHOLDER.to_owned())
            }
        };
        Turn::new(self.turn_id, subject, material)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryContext {
    entries: Vec<MemoryContextEntry>,
}

impl MemoryContext {
    pub fn new(entries: Vec<MemoryContextEntry>) -> Result<Self, MemoryContextError> {
        let mut memories = BTreeSet::new();
        let mut turns = BTreeSet::new();
        if entries.iter().any(|entry| {
            !memories.insert(entry.memory_id)
                || !turns.insert(entry.turn_id)
                || entry.turn_id == BRIEF_TURN_ID
        }) {
            return Err(MemoryContextError::DuplicateIdentity);
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[MemoryContextEntry] {
        &self.entries
    }

    pub fn injection_signals(&self) -> Vec<InjectionSignal> {
        let mut signals: Vec<_> = self
            .entries
            .iter()
            .flat_map(|entry| {
                injection::scan(&UntrustedText::new(format!(
                    "{}\n{}",
                    entry.title, entry.summary
                )))
            })
            .collect();
        signals.sort();
        signals.dedup();
        signals
    }

    pub fn validate_for(&self, request: &DraftRequest) -> Result<(), MemoryContextError> {
        if self.entries.iter().any(|entry| {
            request
                .turns()
                .iter()
                .any(|turn| turn.turn_id == entry.turn_id)
        }) {
            return Err(MemoryContextError::TurnIdentityCollision);
        }
        Ok(())
    }

    pub(crate) fn redact_entries(&self, redactor: &Redactor) -> Vec<RedactedMemoryEntry> {
        self.entries
            .iter()
            .map(|entry| RedactedMemoryEntry {
                memory_id: entry.memory_id,
                endpoint_use: entry.endpoint_use,
                // No exemption argument exists on this path.
                body: redactor.redact_for_e1(&[entry.turn()]),
            })
            .collect()
    }
}

pub(crate) struct RedactedMemoryEntry {
    memory_id: Uuid,
    endpoint_use: EndpointMemoryUse,
    body: RedactedBody,
}

impl fmt::Debug for RedactedMemoryEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedactedMemoryEntry")
            .field("memory_id", &self.memory_id)
            .field("endpoint_use", &self.endpoint_use)
            .finish_non_exhaustive()
    }
}

/// A single already-redacted base plus independently redacted memory items.
/// Trimming never recreates a one-shot original-turn exemption.
pub struct RedactedMemoryContext {
    base: RedactedBody,
    entries: Vec<RedactedMemoryEntry>,
}

impl fmt::Debug for RedactedMemoryContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedactedMemoryContext")
            .field("retained_memory_ids", &self.retained_memory_ids())
            .field("endpoint_owner_ids", &self.endpoint_owner_ids())
            .finish_non_exhaustive()
    }
}

impl RedactedMemoryContext {
    pub(crate) fn new(base: RedactedBody, entries: Vec<RedactedMemoryEntry>) -> Self {
        Self { base, entries }
    }

    pub fn body(&self) -> Result<RedactedBody, MemoryContextError> {
        let mut parts = Vec::with_capacity(self.entries.len() + 1);
        parts.push(&self.base);
        parts.extend(self.entries.iter().map(|entry| &entry.body));
        combine_redacted(&parts).map_err(|_| MemoryContextError::Composition)
    }

    pub fn trim_last(&mut self) -> bool {
        self.entries.pop().is_some()
    }

    pub fn retained_memory_ids(&self) -> Vec<Uuid> {
        self.entries.iter().map(|entry| entry.memory_id).collect()
    }

    pub fn endpoint_owner_ids(&self) -> Vec<Uuid> {
        self.entries
            .iter()
            .filter(|entry| entry.endpoint_use == EndpointMemoryUse::OwnerExcerpt)
            .map(|entry| entry.memory_id)
            .collect()
    }
}
