//! Explicit local goals and stable references to existing import observations.
use super::*;
use crate::commands::insights::{self, InsightReference, SelectedInsightContext};
use soul_schema::memory::{ForgetState, MemoryType};
pub use soul_store_api::assistance::{
    AssistanceHandling, AssistanceItem as AssistanceItemRow, AssistanceObservationSource,
    AssistanceSource, AssistanceValidity,
};
use soul_store_api::MemoryStore;

#[derive(Debug, Clone, Serialize)]
pub struct AssistanceItems {
    pub items: Vec<AssistanceItemRow>,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssistanceItemDetail {
    pub item: AssistanceItemRow,
    pub goal: Option<MemoryDetail>,
    pub observation: Option<InsightReference>,
    pub notice: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistanceItemRequest {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAssistanceGoal {
    pub title: String,
    pub summary: String,
    pub deadline_at: Option<String>,
    pub source_observation: Option<AssistanceItemRequest>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditAssistanceGoal {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub title: String,
    pub summary: String,
    pub deadline_at: Option<String>,
}
macro_rules! redacted_debug {
    ($name:ty) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
}
redacted_debug!(CreateAssistanceGoal);
redacted_debug!(EditAssistanceGoal);
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkAssistanceObservation {
    pub bundle_id: Uuid,
    pub contact_id: Uuid,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshAssistanceObservation {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub bundle_id: Uuid,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindAssistanceGoalSource {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub source_observation: Option<AssistanceItemRequest>,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceAction {
    Cancel,
    CompleteSelfReported,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistanceActionRequest {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub action: AssistanceAction,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssistanceContextView {
    Memory {
        item_id: Uuid,
        revision: Uuid,
        memory: MemoryContextView,
    },
    Insight {
        item_id: Uuid,
        revision: Uuid,
        insight: InsightContextView,
    },
}
#[derive(Debug, Clone)]
pub(super) struct AssistanceBinding {
    item: AssistanceItemRow,
}

fn changed() -> SessionRefusal {
    SessionRefusal {
        reason_code: "ASSISTANCE_CHANGED".into(),
        explanation:
            "事项、目标或固定来源已变化，请重新打开当前事项。已完成或取消的事项不会自动重开。"
                .into(),
    }
}
fn deadline(value: &Option<String>) -> Result<(), SessionRefusal> {
    if let Some(at) = value {
        // -00:00 means an unknown offset in RFC3339, not an explicit timezone.
        if at.ends_with("-00:00") || soul_graph::t4d_adapt::rfc3339_to_unix(at).is_err() {
            return Err(SessionRefusal {reason_code:"ASSISTANCE_DEADLINE_INVALID".into(),explanation:"截止时间须是带明确时区偏移的绝对 RFC3339 时间；不把自然语言或缺失时间补成午夜。".into()});
        }
    }
    Ok(())
}
fn expected(
    store: &SqlCipherStore,
    id: Uuid,
    revision: Uuid,
) -> Result<AssistanceItemRow, SessionRefusal> {
    let item = store.assistance_item(id)?;
    if item.revision != revision || !item.is_open() {
        return Err(changed());
    }
    Ok(item)
}
fn goal_memory(store: &SqlCipherStore, item: &AssistanceItemRow) -> Result<Uuid, SessionRefusal> {
    let AssistanceSource::ManualGoal { memory_id, .. } = item.source else {
        return Err(changed());
    };
    let memory = store.get_memory(memory_id)?;
    if memory.memory_type != MemoryType::Commitment || memory.forget_state != ForgetState::Active {
        return Err(changed());
    }
    Ok(memory_id)
}
fn observation(
    store: &mut SqlCipherStore,
    namespace: Uuid,
    contact: Uuid,
    bundle: Uuid,
) -> Result<SelectedInsightContext, SessionRefusal> {
    let selection = insights::select_context(store, bundle, contact)?;
    let reference = selection.reference();
    if reference.namespace_id != namespace
        || reference.contact_id != contact
        || reference.bundle_id != bundle
    {
        return Err(changed());
    }
    Ok(selection)
}
fn source_from(
    store: &mut SqlCipherStore,
    request: &AssistanceItemRequest,
) -> Result<AssistanceObservationSource, SessionRefusal> {
    let item = expected(store, request.item_id, request.expected_revision)?;
    if item.validity != AssistanceValidity::Current {
        return Err(changed());
    }
    let AssistanceSource::Observation {
        namespace_id,
        contact_id,
        bundle_id,
    } = item.source
    else {
        return Err(changed());
    };
    observation(store, namespace_id, contact_id, bundle_id)?;
    Ok(AssistanceObservationSource {
        item_id: item.item_id,
        revision: item.revision,
        namespace_id,
        contact_id,
        bundle_id,
    })
}
fn validate(
    store: &mut SqlCipherStore,
    item: &AssistanceItemRow,
) -> Result<Option<SelectedInsightContext>, SessionRefusal> {
    if !item.is_current() {
        return Err(changed());
    }
    match &item.source {
        AssistanceSource::Observation {
            namespace_id,
            contact_id,
            bundle_id,
        } => observation(store, *namespace_id, *contact_id, *bundle_id).map(Some),
        AssistanceSource::ManualGoal {
            observation: source,
            ..
        } => {
            let memory_id = goal_memory(store, item)?;
            // A sourced goal still depends on its own readable goal/CK. It
            // cannot borrow the observation's liveness in place of that goal.
            drop(memory_commands::read(store, memory_id)?);
            if let Some(source) = source {
                let actual = source_from(
                    store,
                    &AssistanceItemRequest {
                        item_id: source.item_id,
                        expected_revision: source.revision,
                    },
                )?;
                if actual != *source {
                    return Err(changed());
                }
                observation(
                    store,
                    source.namespace_id,
                    source.contact_id,
                    source.bundle_id,
                )
                .map(Some)
            } else {
                Ok(None)
            }
        }
    }
}
fn detail(
    store: &mut SqlCipherStore,
    item: AssistanceItemRow,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    // This is the explicit one-item read. A failed source never hides or destroys
    // an independent handwritten goal's own memory.
    let goal = match &item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => {
            match memory_commands::detail(store, *memory_id) {
                Ok(memory) => Some(memory),
                Err(soul_memory::MemoryError::Forgotten(_)) => None,
                Err(error) => return Err(error.into()),
            }
        }
        _ => None,
    };
    let reference = if item.is_current() {
        match validate(store, &item) {
            Ok(selection) => selection.map(|s| s.reference()),
            Err(error)
                if matches!(
                    error.reason_code.as_str(),
                    "ASSISTANCE_CHANGED" | "LOCAL_INSIGHT_UNAVAILABLE"
                ) =>
            {
                None
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let available = match &item.source {
        AssistanceSource::ManualGoal { observation, .. } => {
            goal.is_some() && (observation.is_none() || reference.is_some())
        }
        AssistanceSource::Observation { .. } => reference.is_some(),
    };
    // Read-time domain validation can include a masked operational error in
    // existing insight rebuilding. It must never persist a business transition.
    // Explicit source changes and forgetting own their atomic invalidations.
    let notice = if available {
        "本地事项；查看、复制或生成草稿均不表示现实任务已完成。"
    } else {
        "固定来源已失效或事项已结束；可明确重新绑定或清除来源，结束的事项不会自动重开。"
    }
    .to_owned();
    Ok(AssistanceItemDetail {
        item,
        goal,
        observation: reference,
        notice,
    })
}

impl Session {
    pub fn assistance_items(&self) -> Result<AssistanceItems, SessionRefusal> {
        let shared = self.opened_store()?;
        let items = hold(&shared).assistance_items()?;
        Ok(AssistanceItems { items })
    }
    pub fn assistance_item(&self, item_id: &str) -> Result<AssistanceItemDetail, SessionRefusal> {
        let id = parse_id(item_id, "事项")?;
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        store.transact(|store| {
            let item = store.assistance_item(id)?;
            detail(store, item)
        })
    }
    pub fn create_assistance_goal(
        &mut self,
        request: CreateAssistanceGoal,
    ) -> Result<AssistanceItemDetail, SessionRefusal> {
        deadline(&request.deadline_at)?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let at = now_unix_seconds();
        store.transact(|store| {
            let source = request
                .source_observation
                .as_ref()
                .map(|r| source_from(store, r))
                .transpose()?;
            let memory = memory_commands::write_new(
                store,
                &NewMemory {
                    memory_type: "commitment".into(),
                    title: request.title,
                    summary: request.summary,
                },
                at,
            )?;
            let item = AssistanceItemRow {
                item_id: Uuid::now_v7(),
                revision: Uuid::now_v7(),
                source: AssistanceSource::ManualGoal {
                    memory_id: parse_id(&memory.memory_id, "目标记忆")?,
                    observation: source,
                },
                validity: AssistanceValidity::Current,
                handling: AssistanceHandling::Open,
                deadline_at: request.deadline_at,
                created_at: at,
                updated_at: at,
            };
            store.insert_assistance_item(&item)?;
            detail(store, item)
        })
    }
    pub fn edit_assistance_goal(
        &mut self,
        request: EditAssistanceGoal,
    ) -> Result<AssistanceItemDetail, SessionRefusal> {
        deadline(&request.deadline_at)?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let at = now_unix_seconds();
        store.transact(|store| {
            let item = expected(store, request.item_id, request.expected_revision)?;
            let memory_id = goal_memory(store, &item)?;
            memory_commands::write_change(
                store,
                memory_id,
                &MemoryChange {
                    title: Some(request.title),
                    summary: Some(request.summary),
                    memory_type: None,
                },
                at,
            )?;
            // put_memory refreshed the exact item's revision in its savepoint.
            let mut item = store.assistance_item(item.item_id)?;
            let old = item.revision;
            item.revision = Uuid::now_v7();
            item.deadline_at = request.deadline_at;
            item.updated_at = at;
            store.replace_assistance_item(&item, old)?;
            detail(store, item)
        })
    }
    pub fn link_assistance_observation(
        &mut self,
        request: LinkAssistanceObservation,
    ) -> Result<AssistanceItemDetail, SessionRefusal> {
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let at = now_unix_seconds();
        store.transact(|store| {
            let selected = insights::select_context(store, request.bundle_id, request.contact_id)?;
            let reference = selected.reference();
            // Identity is namespace/contact, never a newly recomputed bundle.
            if let Some(item) =
                store.assistance_observation(reference.namespace_id, request.contact_id)?
            {
                return detail(store, item);
            }
            let item = AssistanceItemRow {
                item_id: Uuid::now_v7(),
                revision: Uuid::now_v7(),
                source: AssistanceSource::Observation {
                    namespace_id: reference.namespace_id,
                    contact_id: reference.contact_id,
                    bundle_id: reference.bundle_id,
                },
                validity: AssistanceValidity::Current,
                handling: AssistanceHandling::Open,
                deadline_at: None,
                created_at: at,
                updated_at: at,
            };
            store.insert_assistance_item(&item)?;
            detail(store, item)
        })
    }
    pub fn refresh_assistance_observation(
        &mut self,
        request: RefreshAssistanceObservation,
    ) -> Result<AssistanceItemDetail, SessionRefusal> {
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        store.transact(|store| {
            let mut item = expected(store, request.item_id, request.expected_revision)?;
            let AssistanceSource::Observation {
                namespace_id,
                contact_id,
                ..
            } = item.source
            else {
                return Err(changed());
            };
            observation(store, namespace_id, contact_id, request.bundle_id)?;
            item.source = AssistanceSource::Observation {
                namespace_id,
                contact_id,
                bundle_id: request.bundle_id,
            };
            item.validity = AssistanceValidity::Current;
            item.revision = Uuid::now_v7();
            item.updated_at = now_unix_seconds();
            store.replace_assistance_item(&item, request.expected_revision)?;
            detail(store, item)
        })
    }
    pub fn bind_assistance_goal_source(
        &mut self,
        request: BindAssistanceGoalSource,
    ) -> Result<AssistanceItemDetail, SessionRefusal> {
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        store.transact(|store| {
            let mut item = expected(store, request.item_id, request.expected_revision)?;
            let memory_id = goal_memory(store, &item)?;
            let source = request
                .source_observation
                .as_ref()
                .map(|r| source_from(store, r))
                .transpose()?;
            item.source = AssistanceSource::ManualGoal {
                memory_id,
                observation: source,
            };
            item.revision = Uuid::now_v7();
            item.validity = AssistanceValidity::Current;
            item.updated_at = now_unix_seconds();
            store.replace_assistance_item(&item, request.expected_revision)?;
            detail(store, item)
        })
    }
    pub fn act_assistance_item(
        &mut self,
        request: AssistanceActionRequest,
    ) -> Result<AssistanceItemDetail, SessionRefusal> {
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        store.transact(|store| {
            let mut item = expected(store, request.item_id, request.expected_revision)?;
            item.handling = match request.action {
                AssistanceAction::Cancel => AssistanceHandling::Cancelled,
                AssistanceAction::CompleteSelfReported => AssistanceHandling::ResolvedReported,
            };
            item.revision = Uuid::now_v7();
            item.updated_at = now_unix_seconds();
            store.replace_assistance_item(&item, request.expected_revision)?;
            detail(store, item)
        })
    }
    pub fn prepare_assistance_context(
        &mut self,
        request: AssistanceItemRequest,
    ) -> Result<AssistanceContextView, SessionRefusal> {
        self.require_store_idle()?;
        self.require_draft_slot()?;
        let item = {
            let shared = self.opened_store()?;
            let mut store = hold(&shared);
            let item = expected(&store, request.item_id, request.expected_revision)?;
            validate(&mut store, &item)?;
            item
        };
        let binding = AssistanceBinding { item: item.clone() };
        let (contact, bundle) = match &item.source {
            AssistanceSource::Observation {
                contact_id,
                bundle_id,
                ..
            } => (Some(*contact_id), Some(*bundle_id)),
            AssistanceSource::ManualGoal {
                observation: Some(s),
                ..
            } => (Some(s.contact_id), Some(s.bundle_id)),
            _ => (None, None),
        };
        if let (Some(contact_id), Some(bundle_id)) = (contact, bundle) {
            let insight = self.prepare_insight_context(InsightContextRequest {
                bundle_id,
                contact_id,
            })?;
            self.held_insight.as_mut().ok_or_else(changed)?.assistance = Some(binding.clone());
            self.revalidate_assistance(&binding)?;
            Ok(AssistanceContextView::Insight {
                item_id: item.item_id,
                revision: item.revision,
                insight,
            })
        } else {
            let AssistanceSource::ManualGoal { memory_id, .. } = item.source else {
                return Err(changed());
            };
            let memory = self.prepare_memory_context(MemoryContextRequest {
                candidate_ids: vec![memory_id],
                query: None,
                memory_type: Some(MemoryType::Commitment),
                time_specific: false,
            })?;
            if memory.context_id.is_none() {
                return Err(changed());
            }
            self.held_memory.as_mut().ok_or_else(changed)?.assistance = Some(binding.clone());
            self.revalidate_assistance(&binding)?;
            Ok(AssistanceContextView::Memory {
                item_id: item.item_id,
                revision: item.revision,
                memory,
            })
        }
    }
    pub(super) fn revalidate_assistance(
        &self,
        binding: &AssistanceBinding,
    ) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let item = expected(&store, binding.item.item_id, binding.item.revision)?;
        if item != binding.item {
            return Err(changed());
        }
        validate(&mut store, &item)?;
        Ok(())
    }
}
