//! Owned local work. No endpoint, token, transport, or persistent grant exists here.
use super::*;
use soul_store_api::assistance::AssistanceWorkClaim;
pub use soul_store_api::assistance::{AssistanceWorkMetadata, AssistanceWorkState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceRuntimeState {
    NotStarted,
    Running,
    Stopped,
    Failed,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssistanceWorkStatus {
    pub active_worker: bool,
    pub active_item_id: Option<Uuid>,
    pub pending_items: Option<u64>,
    pub claimed_items: Option<u64>,
    pub foreground_preparation_held: bool,
    pub coordination_unknown: bool,
    pub collector_paused_for_import: bool,
    pub can_begin: bool,
    pub runtime_state: AssistanceRuntimeState,
    pub last_runtime_error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssistanceWorkOutcome {
    pub item_id: Uuid,
    pub revision: Uuid,
    pub claim_id: Uuid,
    pub status: AssistanceWorkDisposition,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceWorkDisposition {
    Ready,
    SourceUnavailable,
    Failed,
    Superseded,
}
#[derive(Debug, Clone)]
pub(super) struct ActiveAssistance {
    pub(super) id: Uuid,
    pub(super) claim: AssistanceWorkClaim,
    pub(super) unknown: bool,
    attempted: Arc<AtomicBool>,
}

pub struct SessionAssistanceWork {
    id: Uuid,
    lease: DraftLease,
    store: Arc<Mutex<SqlCipherStore>>,
    claim: AssistanceWorkClaim,
    attempted: Arc<AtomicBool>,
}
pub struct SessionAssistanceCompletion {
    id: Uuid,
    _lease: DraftLease,
    result: Result<AssistanceWorkOutcome, SessionRefusal>,
}
impl std::fmt::Debug for SessionAssistanceWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionAssistanceWork")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for SessionAssistanceCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionAssistanceCompletion")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
fn unknown() -> SessionRefusal {
    SessionRefusal {
        reason_code: "ASSISTANCE_COORDINATION_UNKNOWN".into(),
        explanation: "本地工作提交状态尚未确认；保留原认领并重新核对，不自动重复执行。".into(),
    }
}
fn outcome(
    claim: &AssistanceWorkClaim,
    status: AssistanceWorkDisposition,
) -> AssistanceWorkOutcome {
    AssistanceWorkOutcome {
        item_id: claim.item_id,
        revision: claim.revision,
        claim_id: claim.claim_id,
        status,
    }
}

impl SessionAssistanceWork {
    pub fn execute(self) -> SessionAssistanceCompletion {
        self.attempted.store(true, Ordering::Release);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut store = hold(&self.store);
            let result = store.transact(|store| -> Result<AssistanceWorkOutcome, SessionRefusal> {
                let item = store.assistance_item(self.claim.item_id)?;
                let (status, code) = if item.revision != self.claim.revision || !item.is_open() {
                    (AssistanceWorkState::SourceUnavailable, None)
                } else if item.validity != AssistanceValidity::Current {
                    (AssistanceWorkState::SourceUnavailable, None)
                } else {
                    match assistance::validate(store, &item) {
                        Ok(_) => (AssistanceWorkState::Ready, None),
                        Err(error)
                            if matches!(
                                error.reason_code.as_str(),
                                "ASSISTANCE_CHANGED" | "LOCAL_INSIGHT_UNAVAILABLE"
                            ) =>
                        {
                            (AssistanceWorkState::SourceUnavailable, None)
                        }
                        Err(_) => (
                            AssistanceWorkState::Failed,
                            Some("ASSISTANCE_LOCAL_READ_FAILED"),
                        ),
                    }
                };
                // The same guard/transaction owns final source validation and
                // publication. A changed revision cannot receive this result.
                if status == AssistanceWorkState::Ready {
                    preparation::prepare_for_claim(store, &self.claim, &item)?;
                }
                let accepted = store.finish_assistance_work(&self.claim, status, code)?;
                let disposition = if !accepted {
                    AssistanceWorkDisposition::Superseded
                } else {
                    match status {
                        AssistanceWorkState::Ready => AssistanceWorkDisposition::Ready,
                        AssistanceWorkState::Failed => AssistanceWorkDisposition::Failed,
                        _ => AssistanceWorkDisposition::SourceUnavailable,
                    }
                };
                Ok(outcome(&self.claim, disposition))
            });
            match result {
                Ok(value) => Ok(value),
                Err(_)
                    if !store.transaction_in_progress()
                        && store
                            .assistance_claim_completed(&self.claim)
                            .unwrap_or(false) =>
                {
                    Ok(outcome(&self.claim, AssistanceWorkDisposition::Superseded))
                }
                Err(_) => Err(unknown()),
            }
        }))
        .unwrap_or_else(|_| {
            let mut store = hold(&self.store);
            if store.transaction_in_progress() {
                return Err(unknown());
            }
            if store
                .assistance_claim_completed(&self.claim)
                .unwrap_or(false)
            {
                return Ok(outcome(&self.claim, AssistanceWorkDisposition::Superseded));
            }
            store
                .transact(|store| {
                    let accepted = store.finish_assistance_work(
                        &self.claim,
                        AssistanceWorkState::Failed,
                        Some("ASSISTANCE_LOCAL_WORKER_FAILED"),
                    )?;
                    Ok::<_, SessionRefusal>(outcome(
                        &self.claim,
                        if accepted {
                            AssistanceWorkDisposition::Failed
                        } else {
                            AssistanceWorkDisposition::Superseded
                        },
                    ))
                })
                .map_err(|_| unknown())
        });
        SessionAssistanceCompletion {
            id: self.id,
            _lease: self.lease,
            result,
        }
    }
}

impl Session {
    pub fn record_assistance_runtime_state(
        &mut self,
        state: AssistanceRuntimeState,
        code: Option<&str>,
    ) {
        self.assistance_runtime_state = state;
        self.last_assistance_runtime_error = code.map(|code| match code {
            "ASSISTANCE_RUNTIME_START_FAILED"
            | "ASSISTANCE_RUNTIME_PANICKED"
            | "ASSISTANCE_RUNTIME_COORDINATION_FAILED"
            | "ASSISTANCE_RUNTIME_FAILED" => code.to_owned(),
            _ => "ASSISTANCE_RUNTIME_FAILED".into(),
        });
    }
    pub(super) fn assistance_foreground_held(&self) -> bool {
        self.held_import.is_some()
            || self.held_memory.is_some()
            || self.held_insight.is_some()
            || self.pending_memory.is_some()
            || self.pending_insight.is_some()
            || self.draft.pending_plan_hash().is_some()
    }
    pub fn assistance_work_status(&self) -> Result<AssistanceWorkStatus, SessionRefusal> {
        let active_worker = self
            .active_draft
            .as_ref()
            .is_some_and(|active| !active.finished.load(Ordering::Acquire));
        let foreground_preparation_held = self.assistance_foreground_held();
        let coordination_unknown = self.active_assistance.as_ref().is_some_and(|a| a.unknown);
        let counts = if self.store_work_busy() {
            None
        } else {
            let shared = self.opened_store()?;
            let counts = hold(&shared).assistance_work_counts()?;
            Some(counts)
        };
        Ok(AssistanceWorkStatus {
            active_worker,
            active_item_id: self.active_assistance.as_ref().map(|a| a.claim.item_id),
            pending_items: counts.map(|v| v.0),
            claimed_items: counts.map(|v| v.1),
            foreground_preparation_held,
            coordination_unknown,
            collector_paused_for_import: self.collector_paused_for_import,
            can_begin: !active_worker
                && !foreground_preparation_held
                && !coordination_unknown
                && counts.is_some_and(|v| v.0 > 0 && v.1 == 0),
            runtime_state: self.assistance_runtime_state,
            last_runtime_error: self.last_assistance_runtime_error.clone(),
        })
    }
    pub fn assistance_item_work(
        &self,
        item_id: &str,
    ) -> Result<AssistanceWorkMetadata, SessionRefusal> {
        let id = parse_id(item_id, "事项")?;
        let shared = self.opened_store()?;
        let mut metadata = hold(&shared).assistance_item_work(id)?;
        if self
            .active_assistance
            .as_ref()
            .is_some_and(|a| a.claim.item_id == id && a.unknown)
        {
            metadata.state = AssistanceWorkState::Unknown;
        }
        Ok(metadata)
    }
    pub fn retry_assistance_work(
        &mut self,
        request: AssistanceItemRequest,
    ) -> Result<AssistanceWorkMetadata, SessionRefusal> {
        if self
            .active_assistance
            .as_ref()
            .is_some_and(|a| a.claim.item_id == request.item_id && a.unknown)
        {
            return Err(unknown());
        }
        let shared = self.opened_store()?;
        let result =
            hold(&shared).retry_assistance_work(request.item_id, request.expected_revision)?;
        Ok(result)
    }
    pub fn begin_assistance_work(
        &mut self,
    ) -> Result<Option<SessionAssistanceWork>, SessionRefusal> {
        self.reconcile_assistance_reminder_check()?;
        self.reconcile_assistance_work()?;
        if self.active_draft.is_some() || self.assistance_foreground_held() {
            return Ok(None);
        }
        let shared = self.opened_store()?;
        let claim = hold(&shared).claim_assistance_work()?;
        let Some(claim) = claim else { return Ok(None) };
        let id = Uuid::now_v7();
        let finished = Arc::new(AtomicBool::new(false));
        self.active_draft = Some(ActiveDraft {
            id,
            kind: E1WorkerKind::Assistance,
            finished: finished.clone(),
        });
        let attempted = Arc::new(AtomicBool::new(false));
        self.active_assistance = Some(ActiveAssistance {
            id,
            claim: claim.clone(),
            unknown: false,
            attempted: attempted.clone(),
        });
        Ok(Some(SessionAssistanceWork {
            id,
            lease: DraftLease(finished),
            store: shared,
            claim,
            attempted,
        }))
    }
    pub fn complete_assistance_work(
        &mut self,
        completed: SessionAssistanceCompletion,
    ) -> Result<AssistanceWorkOutcome, SessionRefusal> {
        if !self
            .active_draft
            .as_ref()
            .is_some_and(|a| a.id == completed.id && a.kind == E1WorkerKind::Assistance)
        {
            return Err(unknown());
        }
        match completed.result {
            Ok(result) => {
                self.active_draft = None;
                self.active_assistance = None;
                let _ = self.flush_deferred_revoke();
                Ok(result)
            }
            Err(error) => {
                if let Some(active) = &mut self.active_assistance {
                    active.unknown = true;
                }
                Err(error)
            }
        }
    }
    pub fn reconcile_assistance_work(&mut self) -> Result<AssistanceWorkStatus, SessionRefusal> {
        if let Some(active) = self.active_assistance.clone() {
            if self
                .active_draft
                .as_ref()
                .is_some_and(|a| a.id == active.id && a.finished.load(Ordering::Acquire))
            {
                let result = (|| -> Result<(), SessionRefusal> {
                    let shared = self.opened_store()?;
                    let mut store = hold(&shared);
                    if store.transaction_in_progress() {
                        return Err(unknown());
                    }
                    if !store.assistance_claim_completed(&active.claim)? {
                        if active.attempted.load(Ordering::Acquire) || active.unknown {
                            // Resolve the original attempt with SQL metadata;
                            // never run its computation under a fresh claim.
                            store.transact(|store| {
                                store.finish_assistance_work(
                                    &active.claim,
                                    AssistanceWorkState::Failed,
                                    Some("ASSISTANCE_LOCAL_COMMIT_FAILED"),
                                )
                            })?;
                        } else {
                            store.abandon_assistance_claim(&active.claim)?;
                        }
                    }
                    Ok(())
                })();
                if result.is_err() {
                    if let Some(a) = &mut self.active_assistance {
                        a.unknown = true;
                    }
                    return Err(unknown());
                }
                self.active_assistance = None;
                self.active_draft = None;
                let _ = self.flush_deferred_revoke();
            }
        }
        self.assistance_work_status()
    }
}
