//! A single-use reference to one contact's trusted, exact A2 bundle snapshot.
use super::*;
use crate::commands::insights::{self, InsightReference, SelectedInsightContext};

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
}

#[derive(Debug)]
pub(super) struct HeldInsight {
    id: Uuid,
    revision: Uuid,
    selection: SelectedInsightContext,
    pub(super) assistance: Option<assistance::AssistanceBinding>,
}

#[derive(Debug)]
pub(super) struct PendingInsight {
    preparation_id: String,
    selection: SelectedInsightContext,
    pub(super) assistance: Option<assistance::AssistanceBinding>,
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
    ) -> Result<Option<SelectedInsightContext>, SessionRefusal> {
        if let Some(pending) = &self.pending_insight {
            if pending.preparation_id != approval.preparation_id {
                return Err(changed());
            }
            if let Some(binding) = &pending.assistance {
                self.revalidate_assistance(binding)?;
            }
            self.revalidate_insight(&pending.selection)?;
            return Ok(Some(pending.selection.clone()));
        }
        Ok(None)
    }

    pub fn local_draft_with_insight(
        &mut self,
        pasted: &str,
        context_id: &str,
    ) -> Result<InsightDraftValue, SessionRefusal> {
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
        self.require_store_idle()?;
        self.require_draft_slot()?;
        let selection = self.selected_insight(context_id)?;
        let assistance = self
            .held_insight
            .as_ref()
            .and_then(|held| held.assistance.clone());
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
            &selection,
            include_counts,
        )
        .map_err(|refusal| self.refuse_draft(refusal))?;
        if let Err(refusal) =
            self.note_injection(injection::scan(&UntrustedText::new(pasted)).len() as u64)
        {
            self.draft.discard();
            return Err(refusal);
        }
        self.pending_insight = Some(PendingInsight {
            preparation_id: prepared.plan.preparation_id.clone(),
            selection: selection.clone(),
            assistance,
        });
        Ok(InsightDraftPlan {
            prepared,
            reference: selection.reference(),
        })
    }
}
