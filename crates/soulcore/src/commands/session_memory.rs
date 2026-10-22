//! The one-use, locally selected memory context and its actual draft dependencies.
use super::*;
use soul_draft::memory_context::{
    EndpointMemoryUse, MemoryContext, MemoryContextEntry, MemoryContextPrivacy,
};
use soul_memory::{
    ContextSelection, MemoryPrivacyClass, MemorySelectionPreview, MemorySelectionRequest,
    MemoryTimeScope, SelectionLimits, SelectionOutcome,
};
use soul_schema::memory::MemoryType;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryContextRequest {
    pub candidate_ids: Vec<Uuid>,
    pub query: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub time_specific: bool,
}
impl std::fmt::Debug for MemoryContextRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryContextRequest")
            .field("candidate_ids", &self.candidate_ids)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryContextView {
    pub context_id: Option<Uuid>,
    pub preview: MemorySelectionPreview,
}
#[derive(Debug, Clone, Serialize)]
pub struct MemoryDraftValue {
    pub draft: DraftValue,
    pub context: MemorySelectionPreview,
}
#[derive(Debug, Clone, Serialize)]
pub struct MemoryDraftPlan {
    pub prepared: draft::MemoryPrepared,
    pub context: MemorySelectionPreview,
}

#[derive(Debug)]
pub(super) struct HeldMemory {
    pub(super) id: Uuid,
    pub(super) revision: Uuid,
    pub(super) selection: ContextSelection,
    pub(super) assistance: Option<assistance::AssistanceBinding>,
}
#[derive(Debug)]
pub(super) struct PendingMemory {
    preparation_id: String,
    selection: ContextSelection,
    pub(super) assistance: Option<assistance::AssistanceBinding>,
}

pub(super) fn changed() -> SessionRefusal {
    SessionRefusal {
        reason_code: "MEMORY_CONTEXT_CHANGED".to_owned(),
        explanation:
            "本次记忆参考已变化、被消费或已失效，请重新预览。澄清和无材料结果不能用于记忆起草。"
                .to_owned(),
    }
}

pub(super) fn material(
    selection: &ContextSelection,
    owner_ids: &[Uuid],
) -> Result<MemoryContext, SessionRefusal> {
    let ids: std::collections::BTreeSet<_> = owner_ids.iter().copied().collect();
    if ids.len() != owner_ids.len()
        || ids.iter().any(|id| {
            !selection.selected().iter().any(|entry| {
                entry.memory_id() == *id && entry.privacy_class() == MemoryPrivacyClass::Owner
            })
        })
    {
        return Err(changed());
    }
    MemoryContext::new(
        selection
            .selected()
            .iter()
            .map(|entry| {
                let privacy = match entry.privacy_class() {
                    MemoryPrivacyClass::Owner => MemoryContextPrivacy::Owner,
                    MemoryPrivacyClass::ThirdParty => MemoryContextPrivacy::ThirdParty,
                    MemoryPrivacyClass::Mixed => MemoryContextPrivacy::Mixed,
                    MemoryPrivacyClass::Unknown => MemoryContextPrivacy::Unknown,
                };
                MemoryContextEntry::new(
                    entry.memory_id(),
                    Uuid::now_v7(),
                    privacy,
                    entry.title(),
                    entry.summary(),
                    if ids.contains(&entry.memory_id()) {
                        EndpointMemoryUse::OwnerExcerpt
                    } else {
                        EndpointMemoryUse::LocalReferenceOnly
                    },
                )
                .map_err(|_| changed())
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(|_| changed())
}

impl Session {
    pub fn prepare_memory_context(
        &mut self,
        request: MemoryContextRequest,
    ) -> Result<MemoryContextView, SessionRefusal> {
        self.require_draft_slot()?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let selection = soul_memory::select_for_draft(
            &*hold(&shared),
            &MemorySelectionRequest {
                candidate_ids: request.candidate_ids,
                query: request.query,
                memory_type: request.memory_type,
                time_scope: if request.time_specific {
                    MemoryTimeScope::AtTime
                } else {
                    MemoryTimeScope::Unspecified
                },
            },
            SelectionLimits::default(),
        )
        .map_err(|_| changed())?;
        let id = (selection.outcome() == SelectionOutcome::Context).then(Uuid::now_v7);
        let preview = selection.preview();
        self.held_memory = id.map(|id| HeldMemory {
            id,
            revision: self.draft_revision,
            selection,
            assistance: None,
        });
        Ok(MemoryContextView {
            context_id: id,
            preview,
        })
    }

    fn selected_memory(&self, id: &str) -> Result<ContextSelection, SessionRefusal> {
        let id = parse_id(id, "本次记忆参考")?;
        let held = self.held_memory.as_ref().ok_or_else(changed)?;
        if held.id != id
            || held.revision != self.draft_revision
            || held.selection.outcome() != SelectionOutcome::Context
        {
            return Err(changed());
        }
        if let Some(binding) = &held.assistance {
            self.revalidate_assistance(binding)?;
        }
        self.revalidate_memory(&held.selection)?;
        Ok(held.selection.clone())
    }

    pub(super) fn revalidate_memory(
        &self,
        selection: &ContextSelection,
    ) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        let store = hold(&shared);
        soul_memory::revalidate_selection(&*store, selection).map_err(|_| changed())
    }

    pub(super) fn memory_for_dispatch(
        &mut self,
        approval: &Approval,
    ) -> Result<Option<ContextSelection>, SessionRefusal> {
        if let Some(pending) = &self.pending_memory {
            if pending.preparation_id != approval.preparation_id {
                return Err(changed());
            }
            if let Some(binding) = &pending.assistance {
                self.revalidate_assistance(binding)?;
            }
            self.revalidate_memory(&pending.selection)?;
            // Dispatch failures must not consume a different preparation, but
            // the selected context cannot become a fresh local grant again.
            return Ok(Some(pending.selection.clone()));
        }
        Ok(None)
    }

    pub fn draft_pasted_with_memory(
        &mut self,
        pasted: &str,
        context_id: &str,
    ) -> Result<MemoryDraftValue, SessionRefusal> {
        self.require_store_idle()?;
        let selection = self.selected_memory(context_id)?;
        self.held_insight = None;
        self.pending_insight = None;
        let memory = material(&selection, &[])?;
        let brief = self.owner_brief();
        let drafted =
            draft::draft_pasted_with_memory(&self.draft, &mut self.policy, brief, pasted, &memory)?;
        self.held_memory = None;
        self.pending_memory = None;
        self.draft.discard();
        self.append_audit(&drafted.audit)?;
        Ok(MemoryDraftValue {
            draft: drafted.draft,
            context: selection.preview(),
        })
    }

    pub fn prepare_draft_with_memory(
        &mut self,
        pasted: &str,
        context_id: &str,
        owner_ids: &[Uuid],
        include_original: bool,
    ) -> Result<MemoryDraftPlan, SessionRefusal> {
        self.require_draft_slot()?;
        let mut selection = self.selected_memory(context_id)?;
        let assistance = self
            .held_memory
            .as_ref()
            .and_then(|held| held.assistance.clone());
        self.held_insight = None;
        self.pending_insight = None;
        let memory = material(&selection, owner_ids)?;
        self.sync_identifiers();
        let brief = self.owner_brief();
        let prepared = draft::prepare_pasted_with_memory(
            &mut self.draft,
            &mut self.policy,
            brief,
            pasted,
            include_original,
            &memory,
        )
        .map_err(|refusal| self.refuse_draft(refusal))?;
        if selection.retain_ids(&prepared.retained_memory_ids).is_err() {
            // The real HTTP serializer may remove complete entries. A task
            // diagnosis must not survive losing its required supports, and a
            // rejected replacement must leave no dispatchable preparation.
            self.draft.discard();
            self.held_memory = None;
            self.pending_memory = None;
            return Err(changed());
        }
        if let Err(refusal) = self
            .note_injection(memory.injection_signals().len() as u64)
            .and_then(|_| {
                self.note_injection(injection::scan(&UntrustedText::new(pasted)).len() as u64)
            })
        {
            self.draft.discard();
            self.pending_memory = None;
            return Err(refusal);
        }
        self.pending_memory = Some(PendingMemory {
            preparation_id: prepared.plan.preparation_id.clone(),
            selection: selection.clone(),
            assistance,
        });
        Ok(MemoryDraftPlan {
            prepared,
            context: selection.preview(),
        })
    }
}
