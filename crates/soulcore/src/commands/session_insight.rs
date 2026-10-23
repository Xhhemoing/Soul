//! A single-use reference to one contact's trusted, exact A2 bundle snapshot.
use super::*;
use crate::commands::insights::{self, InsightReference, SelectedInsightContext};
use soul_memory::{ContextSelection, MemorySelectionPreview};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsightContextRequest {
    pub bundle_id: Uuid,
    pub contact_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct InsightContextView {
    pub context_id: Uuid,
    pub reference: InsightReference,
}

#[derive(Debug, Clone, Serialize)]
pub struct InsightDraftValue {
    pub draft: DraftValue,
    pub reference: InsightReference,
}

#[derive(Debug, Clone, Serialize)]
pub struct InsightDraftPlan {
    pub prepared: draft::InsightPrepared,
    pub reference: InsightReference,
    pub goal_preview: Option<MemorySelectionPreview>,
}

#[derive(Debug)]
pub(super) struct HeldInsight {
    id: Uuid,
    revision: Uuid,
    selection: SelectedInsightContext,
    pub(super) assistance: Option<assistance::AssistanceBinding>,
    pub(super) goal: Option<ContextSelection>,
}

#[derive(Debug)]
pub(super) struct PendingInsight {
    preparation_id: String,
    selection: SelectedInsightContext,
    pub(super) assistance: Option<assistance::AssistanceBinding>,
    goal: Option<ContextSelection>,
}

pub(super) struct DispatchedInsightContext {
    pub selection: SelectedInsightContext,
    pub goal: Option<ContextSelection>,
}

fn changed() -> SessionRefusal {
    SessionRefusal {
        reason_code: "INSIGHT_CONTEXT_CHANGED".to_owned(),
        explanation: "本次统计参考已变化、被消费或已失效，请重新预览统计材料及其出处。".to_owned(),
    }
}

impl Session {
    pub fn prepare_insight_context(
        &mut self,
        request: InsightContextRequest,
    ) -> Result<InsightContextView, SessionRefusal> {
        self.require_store_idle()?;
        self.require_draft_slot()?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let selection =
            insights::select_context(&mut hold(&shared), request.bundle_id, request.contact_id)?;
        let id = Uuid::now_v7();
        let reference = selection.reference();
        self.held_insight = Some(HeldInsight {
            id,
            revision: self.draft_revision,
            selection,
            assistance: None,
            goal: None,
        });
        Ok(InsightContextView {
            context_id: id,
            reference,
        })
    }

    fn selected_insight(&self, id: &str) -> Result<SelectedInsightContext, SessionRefusal> {
        let id = parse_id(id, "本次统计参考")?;
        let held = self.held_insight.as_ref().ok_or_else(changed)?;
        if held.id != id || held.revision != self.draft_revision {
            return Err(changed());
        }
        if let Some(binding) = &held.assistance {
            self.revalidate_assistance(binding)?;
        }
        self.revalidate_insight(&held.selection)?;
        if let Some(goal) = &held.goal {
            self.revalidate_memory(goal)?;
        }
        Ok(held.selection.clone())
    }

    pub(super) fn revalidate_insight(
        &self,
        selection: &SelectedInsightContext,
    ) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        insights::revalidate_context(&mut store, selection).map_err(|_| changed())
    }

    pub(super) fn insight_for_dispatch(
        &self,
        approval: &Approval,
    ) -> Result<Option<DispatchedInsightContext>, SessionRefusal> {
        if let Some(pending) = &self.pending_insight {
            if pending.preparation_id != approval.preparation_id {
                return Err(changed());
            }
            if let Some(binding) = &pending.assistance {
                self.revalidate_assistance(binding)?;
            }
            self.revalidate_insight(&pending.selection)?;
            if let Some(goal) = &pending.goal {
                self.revalidate_memory(goal)?;
            }
            return Ok(Some(DispatchedInsightContext {
                selection: pending.selection.clone(),
                goal: pending.goal.clone(),
            }));
        }
        Ok(None)
    }

    pub fn local_draft_with_insight(
        &mut self,
        pasted: &str,
        context_id: &str,
    ) -> Result<InsightDraftValue, SessionRefusal> {
        self.clear_scoped_draft();
        self.require_store_idle()?;
        let selection = self.selected_insight(context_id)?;
        let brief = self.owner_brief();
        // The deterministic template stays unchanged. The reference belongs
        // to the selected statistical material, not to each draft sentence.
        let drafted = draft::draft_pasted(&self.draft, &mut self.policy, brief, pasted)?;
        self.held_insight = None;
        self.pending_insight = None;
        self.held_memory = None;
        self.pending_memory = None;
        self.draft.discard();
        self.append_audit(&drafted.audit)?;
        Ok(InsightDraftValue {
            draft: drafted.draft,
            reference: selection.reference(),
        })
    }

    pub fn prepare_draft_with_insight(
        &mut self,
        pasted: &str,
        context_id: &str,
        include_counts: bool,
        include_original: bool,
    ) -> Result<InsightDraftPlan, SessionRefusal> {
        self.prepare_draft_with_insight_and_goal(
            pasted,
            context_id,
            include_counts,
            include_original,
            false,
        )
    }

    pub fn prepare_draft_with_insight_and_goal(
        &mut self,
        pasted: &str,
        context_id: &str,
        include_counts: bool,
        include_original: bool,
        include_goal: bool,
    ) -> Result<InsightDraftPlan, SessionRefusal> {
        self.clear_scoped_draft();
        self.require_store_idle()?;
        self.require_draft_slot()?;
        // Every replacement attempt revokes the old approval, including a
        // missing/non-owner goal or a stale context rejected before rendering.
        self.draft.discard();
        self.pending_memory = None;
        self.pending_insight = None;
        let selection = self.selected_insight(context_id)?;
        let assistance = self
            .held_insight
            .as_ref()
            .and_then(|held| held.assistance.clone());
        let goal = if include_goal {
            let held = self.held_insight.as_ref().ok_or_else(changed)?;
            if held.assistance.is_none() {
                return Err(changed());
            }
            Some(held.goal.clone().ok_or_else(changed)?)
        } else {
            None
        };
        let goal_material = goal
            .as_ref()
            .map(|goal| {
                let ids: Vec<_> = goal
                    .selected()
                    .iter()
                    .map(|entry| entry.memory_id())
                    .collect();
                if ids.len() != 1 {
                    return Err(changed());
                }
                memory_context::material(goal, &ids)
            })
            .transpose()?;
        self.held_memory = None;
        self.pending_memory = None;
        self.pending_insight = None;
        self.sync_identifiers();
        let brief = self.owner_brief();
        let prepared = draft::prepare_pasted_with_insight(
            &mut self.draft,
            &mut self.policy,
            brief,
            pasted,
            include_original,
            draft::InsightMaterial {
                selected: &selection,
                include_counts,
                goal: goal_material.as_ref(),
            },
        )
        .map_err(|refusal| self.refuse_draft(refusal))?;
        if let Err(refusal) = goal_material
            .as_ref()
            .map_or(Ok(()), |material| {
                self.note_injection(material.injection_signals().len() as u64)
            })
            .and_then(|_| {
                self.note_injection(injection::scan(&UntrustedText::new(pasted)).len() as u64)
            })
        {
            self.draft.discard();
            return Err(refusal);
        }
        self.pending_insight = Some(PendingInsight {
            preparation_id: prepared.plan.preparation_id.clone(),
            selection: selection.clone(),
            assistance,
            goal: goal.clone(),
        });
        Ok(InsightDraftPlan {
            prepared,
            reference: selection.reference(),
            goal_preview: goal.as_ref().map(ContextSelection::preview),
        })
    }
}
