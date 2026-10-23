//! Transient ownership of the existing core slots and body-free invalidation events.
//! Lock order is Session -> control. Event emission never holds a Session guard.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};

use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_types::*;
use soulcore::commands::memory::{
    ForgetConfirmation, ForgetPreview, ForgetReceiptView, MemoryChange, MemoryDetail,
};
use soulcore::commands::session::{
    ImportOperationView, ImportPrepareRequest, PreferenceOccasion, PreparedImportView,
    RuntimePendingSlots, ScopedDraftView, ScopedRegisterProposal, ScopedRegisterReceipt,
    ScopedRegisterStatus, Session, SessionRefusal, VoiceRegister,
};
use uuid::Uuid;

use super::Runtime;

/// Constructed by trusted transports, never deserialized from a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller(RuntimeCaller);

impl Caller {
    pub fn desktop(label: impl Into<String>) -> Self {
        Self(RuntimeCaller {
            transport: CallerTransport::Desktop,
            webview_label: Some(label.into()),
        })
    }
    pub fn http() -> Self {
        Self(RuntimeCaller {
            transport: CallerTransport::Http,
            webview_label: None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
    caller: Caller,
    correlation_id: Option<Uuid>,
}

impl RuntimeContext {
    pub fn new(caller: Caller, correlation_id: Option<Uuid>) -> Self {
        Self {
            caller,
            correlation_id,
        }
    }
}

#[derive(Debug)]
struct OwnedSlot {
    id: Uuid,
    caller: Caller,
}

#[derive(Debug, Default)]
struct Slots {
    result: Option<OwnedSlot>,
    proposal: Option<OwnedSlot>,
    forget: Option<OwnedSlot>,
    // Committed IDs remain bound for same-owner idempotence in this runtime.
    // A restart has no approvals; explicit operation readback still works.
    imports: HashMap<Uuid, Caller>,
    committed: HashSet<Uuid>,
    import_active: Option<(Uuid, Caller)>,
}

impl Slots {
    fn reconcile(&mut self, actual: RuntimePendingSlots) {
        for (slot, id) in [
            (&mut self.result, actual.scoped_result),
            (&mut self.proposal, actual.scoped_proposal),
            (&mut self.forget, actual.forget_preview),
        ] {
            if slot.as_ref().is_some_and(|held| Some(held.id) != id) {
                *slot = None;
            }
        }
        let active = self.import_active.is_some();
        self.imports.retain(|id, _| {
            active || actual.import_operation == Some(*id) || self.committed.contains(id)
        });
    }
}

#[derive(Debug)]
pub(super) struct Control {
    epoch: String,
    sequence: u64,
    active: BTreeMap<String, RuntimeOperation>,
    subscribers: Vec<mpsc::SyncSender<RuntimeEvent>>,
    slots: Slots,
}

impl Control {
    pub(super) fn new() -> Self {
        Self {
            epoch: Uuid::now_v7().to_string(),
            sequence: 0,
            active: BTreeMap::new(),
            subscribers: Vec::new(),
            slots: Slots::default(),
        }
    }

    fn emit(
        &mut self,
        operation: &RuntimeOperation,
        phase: OperationPhase,
        outcome: Option<OperationOutcome>,
    ) {
        self.sequence = self.sequence.saturating_add(1);
        let event = RuntimeEvent {
            epoch: self.epoch.clone(),
            sequence: self.sequence,
            operation_id: operation.operation_id.clone(),
            caller: operation.caller.clone(),
            correlation_id: operation.correlation_id.clone(),
            kind: operation.kind,
            phase,
            outcome,
        };
        self.subscribers.retain(|sender| {
            !matches!(
                sender.try_send(event.clone()),
                Err(mpsc::TrySendError::Disconnected(_))
            )
        });
    }
}

fn lock(control: &Mutex<Control>) -> MutexGuard<'_, Control> {
    control
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EventGuard {
    control: Arc<Mutex<Control>>,
    operation: RuntimeOperation,
    outcome: OperationOutcome,
}
impl Drop for EventGuard {
    fn drop(&mut self) {
        let mut control = lock(&self.control);
        control.active.remove(&self.operation.operation_id);
        control.emit(&self.operation, OperationPhase::Settled, Some(self.outcome));
    }
}

struct ImportLease {
    control: Arc<Mutex<Control>>,
    id: Uuid,
}
impl Drop for ImportLease {
    fn drop(&mut self) {
        let mut control = lock(&self.control);
        if control
            .slots
            .import_active
            .as_ref()
            .is_some_and(|(id, _)| *id == self.id)
        {
            control.slots.import_active = None;
        }
    }
}

fn refusal(code: &str, explanation: &str) -> SessionRefusal {
    SessionRefusal {
        reason_code: code.into(),
        explanation: explanation.into(),
    }
}
fn mismatch(session: &Session, operation_uuid: Uuid) -> SessionRefusal {
    match session.record_runtime_caller_denial(operation_uuid) {
        Ok(()) => refusal(
            "RUNTIME_CALLER_MISMATCH",
            "This preparation belongs to another caller.",
        ),
        Err(error) => error,
    }
}
fn unavailable() -> SessionRefusal {
    refusal(
        "RUNTIME_CONFIRMATION_UNAVAILABLE",
        "This confirmation is no longer available; prepare again.",
    )
}
fn busy() -> SessionRefusal {
    refusal("IMPORT_BUSY", "An import is still using this Session.")
}
fn check(
    session: &Session,
    slot: &Option<OwnedSlot>,
    actual: Option<Uuid>,
    ctx: &RuntimeContext,
    operation_uuid: Uuid,
) -> Result<(), SessionRefusal> {
    if let Some(id) = actual {
        if !slot
            .as_ref()
            .is_some_and(|slot| slot.id == id && slot.caller == ctx.caller)
        {
            return Err(mismatch(session, operation_uuid));
        }
    }
    Ok(())
}

impl Runtime {
    pub fn subscribe_events(&self, capacity: usize) -> mpsc::Receiver<RuntimeEvent> {
        let (sender, receiver) = mpsc::sync_channel(capacity.clamp(1, 256));
        let mut control = lock(&self.control);
        if control.subscribers.len() < 64 {
            control.subscribers.push(sender);
        }
        receiver
    }

    pub fn event_snapshot(&self) -> RuntimeEventSnapshot {
        let control = lock(&self.control);
        RuntimeEventSnapshot {
            epoch: control.epoch.clone(),
            sequence: control.sequence,
            active_operations: control.active.values().cloned().collect(),
        }
    }

    fn tracked<T>(
        &self,
        ctx: &RuntimeContext,
        kind: OperationKind,
        body: impl FnOnce(Uuid) -> Result<T, SessionRefusal>,
    ) -> Result<T, SessionRefusal> {
        let operation_uuid = Uuid::now_v7();
        let operation = RuntimeOperation {
            operation_id: operation_uuid.to_string(),
            caller: ctx.caller.0.clone(),
            correlation_id: ctx.correlation_id.map(|id| id.to_string()),
            kind,
        };
        {
            let mut control = lock(&self.control);
            control
                .active
                .insert(operation.operation_id.clone(), operation.clone());
            control.emit(&operation, OperationPhase::Started, None);
        }
        let mut guard = EventGuard {
            control: Arc::clone(&self.control),
            operation,
            outcome: OperationOutcome::Refused,
        };
        let result = body(operation_uuid);
        if result.is_ok() {
            guard.outcome = OperationOutcome::Success;
        }
        // body has returned and all its Session/store guards have been dropped.
        drop(guard);
        result
    }

    fn in_session<T>(
        &self,
        body: impl FnOnce(&mut Session, &mut Slots) -> Result<T, SessionRefusal>,
    ) -> Result<T, SessionRefusal> {
        let mut session = self.held_session();
        let mut control = lock(&self.control);
        control.slots.reconcile(session.runtime_pending_slots());
        let result = body(&mut session, &mut control.slots);
        control.slots.reconcile(session.runtime_pending_slots());
        result
    }

    /// Blocking owned-work entry: transports run this whole function on their
    /// owned blocking worker, so dropping an HTTP/IPC waiter never skips complete.
    pub fn prepare_import(
        &self,
        ctx: &RuntimeContext,
        request: ImportPrepareRequest,
    ) -> Result<PreparedImportView, SessionRefusal> {
        self.prepare_import_inner(ctx, request, false)
    }

    pub fn prepare_synthetic_import(
        &self,
        ctx: &RuntimeContext,
        request: SyntheticImportRequest,
    ) -> Result<PreparedImportView, SessionRefusal> {
        let text = match request.fixture_id {
            BuiltinFixtureId::Basic => {
                include_str!("../../../fixtures/import/soul-import-v1/valid_basic.jsonl")
            }
        };
        self.prepare_import_inner(
            ctx,
            ImportPrepareRequest {
                text: text.to_owned(),
                source: ImportSourceKind::SoulImportV1,
                namespace_id: None,
                legacy_choice: LegacyImportChoice::Reject,
            },
            true,
        )
    }

    fn prepare_import_inner(
        &self,
        ctx: &RuntimeContext,
        request: ImportPrepareRequest,
        builtin: bool,
    ) -> Result<PreparedImportView, SessionRefusal> {
        self.tracked(ctx, OperationKind::ImportPrepare, |_operation_uuid| {
            if self.is_synthetic() && !builtin {
                return Err(refusal(
                    "SYNTHETIC_MODE_RESTRICTED",
                    "Only built-in synthetic import fixtures are available.",
                ));
            }
            let (work, lease) = self.in_session(|session, slots| {
                if slots.import_active.is_some() {
                    return Err(busy());
                }
                if let Some(id) = session.runtime_pending_slots().import_operation {
                    if slots.imports.get(&id) != Some(&ctx.caller) {
                        return Err(mismatch(session, _operation_uuid));
                    }
                }
                let work = session.begin_import_prepare_request(request)?;
                let id = Uuid::now_v7();
                slots.import_active = Some((id, ctx.caller.clone()));
                Ok((
                    work,
                    ImportLease {
                        control: Arc::clone(&self.control),
                        id,
                    },
                ))
            })?;
            // Neither ownership bookkeeping nor Session mutex spans execute.
            let completed = work.execute();
            let result = self.in_session(|session, slots| {
                let prepared = session.complete_import_prepare(completed)?;
                slots
                    .imports
                    .insert(prepared.operation.operation_id, ctx.caller.clone());
                Ok(prepared)
            });
            drop(lease);
            result
        })
    }

    pub fn confirm_import(
        &self,
        ctx: &RuntimeContext,
        operation_id: &str,
        preview_id: &str,
    ) -> Result<ImportOperationView, SessionRefusal> {
        self.tracked(ctx, OperationKind::ImportConfirm, |_operation_uuid| {
            let (work, lease) = self.in_session(|session, slots| {
                let id = Uuid::parse_str(operation_id).map_err(|_| unavailable())?;
                match slots.imports.get(&id) {
                    Some(owner) if owner == &ctx.caller => {}
                    Some(_) => return Err(mismatch(session, _operation_uuid)),
                    None => return Err(unavailable()),
                }
                if slots.import_active.is_some() {
                    return Err(busy());
                }
                let work = session.begin_import(operation_id, preview_id)?;
                let id = Uuid::now_v7();
                slots.import_active = Some((id, ctx.caller.clone()));
                Ok((
                    work,
                    ImportLease {
                        control: Arc::clone(&self.control),
                        id,
                    },
                ))
            })?;
            let completed = work.execute();
            let result = self.in_session(|session, slots| {
                let result = session.complete_import(completed);
                if let Ok(view) = session.import_operation(operation_id) {
                    if view.operation.state
                        == soul_store_api::import::ImportOperationState::Committed
                    {
                        slots.committed.insert(view.operation.operation_id);
                    }
                }
                result
            });
            drop(lease);
            result
        })
    }

    pub fn discard_import(
        &self,
        ctx: &RuntimeContext,
        operation_id: &str,
    ) -> Result<ImportOperationView, SessionRefusal> {
        self.tracked(ctx, OperationKind::ImportDiscard, |_operation_uuid| {
            self.in_session(|session, slots| {
                let id = Uuid::parse_str(operation_id).map_err(|_| unavailable())?;
                match slots.imports.get(&id) {
                    Some(owner) if owner == &ctx.caller => {}
                    Some(_) => return Err(mismatch(session, _operation_uuid)),
                    None => return Err(unavailable()),
                }
                if slots.import_active.is_some() {
                    return Err(busy());
                }
                session.discard_import(operation_id)
            })
        })
    }

    pub fn import_operation(
        &self,
        operation_id: &str,
    ) -> Result<ImportOperationView, SessionRefusal> {
        self.held_session().import_operation(operation_id)
    }

    pub fn draft_scoped_reply(
        &self,
        ctx: &RuntimeContext,
        pasted: &str,
        person_id: &str,
        occasion: PreferenceOccasion,
    ) -> Result<ScopedDraftView, SessionRefusal> {
        self.tracked(ctx, OperationKind::ScopedDraft, |_operation_uuid| {
            self.in_session(|session, slots| {
                let result = session.draft_scoped_reply(pasted, person_id, occasion)?;
                slots.result = Some(OwnedSlot {
                    id: result.result_id,
                    caller: ctx.caller.clone(),
                });
                Ok(result)
            })
        })
    }

    pub fn propose_scoped_register(
        &self,
        ctx: &RuntimeContext,
        result_id: &str,
        edit_revision: u64,
        register: VoiceRegister,
    ) -> Result<ScopedRegisterProposal, SessionRefusal> {
        self.tracked(ctx, OperationKind::ScopedPropose, |_operation_uuid| {
            self.in_session(|session, slots| {
                check(
                    session,
                    &slots.result,
                    session.runtime_pending_slots().scoped_result,
                    ctx,
                    _operation_uuid,
                )?;
                let result = session.propose_scoped_register(result_id, edit_revision, register)?;
                slots.proposal = Some(OwnedSlot {
                    id: result.proposal_id,
                    caller: ctx.caller.clone(),
                });
                Ok(result)
            })
        })
    }

    pub fn confirm_scoped_register(
        &self,
        ctx: &RuntimeContext,
        proposal_id: &str,
        expected_revision: Option<Uuid>,
    ) -> Result<ScopedRegisterReceipt, SessionRefusal> {
        self.tracked(ctx, OperationKind::ScopedConfirm, |_operation_uuid| {
            self.in_session(|session, slots| {
                let pending = session.runtime_pending_slots().scoped_proposal;
                check(session, &slots.proposal, pending, ctx, _operation_uuid)?;
                let result = session.confirm_scoped_register(proposal_id, expected_revision);
                result.map_err(|error| {
                    if pending.is_none() && error.reason_code == "ROUTINE" {
                        unavailable()
                    } else {
                        error
                    }
                })
            })
        })
    }

    pub fn discard_scoped_register_proposal(
        &self,
        ctx: &RuntimeContext,
        proposal_id: &str,
    ) -> Result<bool, SessionRefusal> {
        self.tracked(ctx, OperationKind::ScopedDiscard, |_operation_uuid| {
            self.in_session(|session, slots| {
                check(
                    session,
                    &slots.proposal,
                    session.runtime_pending_slots().scoped_proposal,
                    ctx,
                    _operation_uuid,
                )?;
                session.discard_scoped_register_proposal(proposal_id)
            })
        })
    }

    pub fn scoped_register_status(
        &self,
        person_id: &str,
        occasion: PreferenceOccasion,
    ) -> Result<ScopedRegisterStatus, SessionRefusal> {
        self.held_session()
            .scoped_register_status(person_id, occasion)
    }

    pub fn edit_memory(
        &self,
        ctx: &RuntimeContext,
        memory_id: &str,
        change: &MemoryChange,
    ) -> Result<MemoryDetail, SessionRefusal> {
        self.tracked(ctx, OperationKind::MemoryEdit, |_operation_uuid| {
            self.in_session(|session, _| session.edit_memory(memory_id, change))
        })
    }

    pub fn preview_forget(
        &self,
        ctx: &RuntimeContext,
        memory_id: &str,
    ) -> Result<ForgetPreview, SessionRefusal> {
        self.tracked(ctx, OperationKind::ForgetPreview, |_operation_uuid| {
            self.in_session(|session, slots| {
                let preview = session.preview_forget(memory_id)?;
                slots.forget = Some(OwnedSlot {
                    id: Uuid::parse_str(&preview.preview_id).map_err(|_| unavailable())?,
                    caller: ctx.caller.clone(),
                });
                Ok(preview)
            })
        })
    }

    pub fn forget_memory(
        &self,
        ctx: &RuntimeContext,
        confirmation: &ForgetConfirmation,
    ) -> Result<ForgetReceiptView, SessionRefusal> {
        self.tracked(ctx, OperationKind::ForgetConfirm, |_operation_uuid| {
            self.in_session(|session, slots| {
                let pending = session.runtime_pending_slots().forget_preview;
                check(session, &slots.forget, pending, ctx, _operation_uuid)?;
                let result = session.forget_memory(confirmation);
                result.map_err(|error| {
                    if pending.is_none() && error.reason_code == "ROUTINE" {
                        unavailable()
                    } else {
                        error
                    }
                })
            })
        })
    }
}
