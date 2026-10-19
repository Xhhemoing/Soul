//! Immutable import preparations and owned local work. No export prose is durable.
use super::*;
use soul_import::indexed::{self, IndexedImportPlan};
use soul_import::model::{ImportSource, StagedImport};
use soul_store::import_ops::ImportRuntimeLease;
use soul_store_api::import::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPrepareRequest {
    pub text: String,
    pub source: ImportSourceKind,
    pub namespace_id: Option<Uuid>,
    pub legacy_choice: LegacyImportChoice,
}

impl std::fmt::Debug for ImportPrepareRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportPrepareRequest")
            .field("source", &self.source)
            .field("namespace_id", &self.namespace_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportNamespaceView {
    pub namespace_id: Uuid,
    pub source: ImportSourceKind,
    pub state: ImportNamespaceState,
    pub coverage: ImportCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportOperationView {
    pub operation: ImportOperation,
    pub namespace_revoked: bool,
    pub prepared_here: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreparedImportView {
    pub operation: ImportOperation,
    pub preview: ImportPreview,
    pub legacy_events: u64,
    pub notice: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportRecoveryState {
    pub active_worker: bool,
    pub reserved_operations: Option<u64>,
    pub running_operations: Option<u64>,
    pub held_operation_id: Option<Uuid>,
    pub can_start_new_intent: bool,
}

pub(super) struct HeldImport {
    pub operation: ImportOperation,
    revision: Uuid,
    staged: StagedImport,
    plan: IndexedImportPlan,
}

impl std::fmt::Debug for HeldImport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeldImport")
            .field("operation_id", &self.operation.operation_id)
            .finish_non_exhaustive()
    }
}

pub struct SessionImportPrepareWork {
    id: Uuid,
    revision: Uuid,
    runtime: Arc<ImportRuntimeLease>,
    store: Arc<Mutex<SqlCipherStore>>,
    text: String,
    source: ImportSourceKind,
    namespace: Option<Uuid>,
    choice: LegacyImportChoice,
    stale: Option<Uuid>,
    pause: Option<collector_handoff::ImportCollectorPause>,
    // Drop joins a paused collector before advertising physical completion.
    lease: DraftLease,
}

pub struct SessionImportPrepareCompletion {
    id: Uuid,
    revision: Uuid,
    _lease: DraftLease,
    result: Result<HeldImport, SessionRefusal>,
    pause: Option<collector_handoff::CollectorPauseResult>,
}

pub struct SessionImportWork {
    id: Uuid,
    runtime: Arc<ImportRuntimeLease>,
    store: Arc<Mutex<SqlCipherStore>>,
    operation: ImportOperation,
    held: Option<HeldImport>,
    pause: Option<collector_handoff::ImportCollectorPause>,
    lease: DraftLease,
}

pub struct SessionImportCompletion {
    id: Uuid,
    _lease: DraftLease,
    result: Result<ImportOperationView, SessionRefusal>,
    pause: Option<collector_handoff::CollectorPauseResult>,
}

macro_rules! private_debug {
    ($($ty:ty),+ $(,)?) => {$(impl std::fmt::Debug for $ty {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(stringify!($ty)).field("id", &self.id).finish_non_exhaustive()
        }
    })+};
}
private_debug!(
    SessionImportPrepareWork,
    SessionImportPrepareCompletion,
    SessionImportWork,
    SessionImportCompletion
);

fn problem(code: &str, explanation: &str) -> SessionRefusal {
    SessionRefusal {
        reason_code: code.to_owned(),
        explanation: explanation.to_owned(),
    }
}

fn import_error(error: soul_import::ImportError) -> SessionRefusal {
    use soul_import::ImportError::*;
    match error {
        LegacyChoiceRequired => problem("IMPORT_LEGACY_CHOICE_REQUIRED", "历史数据没有消息编号，无法可靠补造去重索引。请明确确认仅为本次及以后建立索引，历史可能重叠，再重新准备。"),
        IdentityConflict => problem("IMPORT_ID_CONFLICT", "相同来源消息编号的正文、发送者或时间不一致，或账号身份冲突。整次导入未获准，请核对来源。"),
        NamespaceRevoked => problem("IMPORT_NAMESPACE_REVOKED", "这个来源或关联身份已被遗忘，不能重新创建旧密钥或继续重导。"),
        SourceChanged => changed(),
        IdentityUnavailable => problem("IMPORT_IDENTITY_UNAVAILABLE", "历史消息身份无法可靠读取，不能把它当成新消息。请核对本机数据。"),
        NoOwner | AmbiguousOwner {..} | UnknownSender {..} | NamespaceRequired => problem("IMPORT_IDENTITY_REQUIRED", "导入必须有唯一的本人、可解析的发送者和明确的来源身份。"),
        Store(_) => problem("IMPORT_STORE_UNAVAILABLE", "未能确认导入存储状态。请查询操作回执，不要直接重导。"),
    }
}

fn changed() -> SessionRefusal {
    problem(
        "IMPORT_SOURCE_CHANGED",
        "准备依据已变更，旧确认不能执行。请重新选择来源并准备。",
    )
}
fn unknown() -> SessionRefusal {
    problem(
        "IMPORT_OUTCOME_UNKNOWN",
        "导入结果尚未确认。请查询这次操作回执，不要自动重发或直接重导。",
    )
}

fn source(source: ImportSourceKind) -> ImportSource {
    match source {
        ImportSourceKind::SoulImportV1 => ImportSource::SoulImportV1,
        ImportSourceKind::TelegramDesktop => ImportSource::TelegramDesktop,
    }
}

impl SessionImportPrepareWork {
    pub fn execute(self) -> SessionImportPrepareCompletion {
        let pause = self.pause.map(|pause| pause.stop());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(pause) = &pause {
                pause.require_stopped()?;
            }
            import_commands::within_byte_budget(&self.text).map_err(over_budget)?;
            let staged = match self.source {
                ImportSourceKind::SoulImportV1 => import_commands::read_soul_import_v1(&self.text)?,
                ImportSourceKind::TelegramDesktop => {
                    import_commands::read_telegram(&telegram_document(&self.text)?)?
                }
            };
            import_commands::within_staged_budget(&staged).map_err(over_budget)?;
            let preview = ImportPreview::of(&staged);
            let mut store = hold(&self.store);
            // Reading an export is an attempt even if identity validation or
            // later delivery refuses it. Keep this outside the import commit.
            if preview.messages_with_injection_markers > 0 {
                append_or_store_error(
                    &mut *store,
                    AuditContent::denied(
                        AuditAction::InjectionBlocked,
                        ReasonCode::InjectionMarkersFound,
                    )
                    .counting(AuditCounts {
                        items: Some(preview.messages_with_injection_markers),
                        bytes: None,
                    }),
                    now_unix_seconds(),
                )?;
            }
            if let Some(id) = self.stale {
                store.end_import_operation(
                    &self.runtime,
                    id,
                    ImportOperationState::Reserved,
                    ImportOperationState::Invalidated,
                    Some(ImportOperationError::SourceChanged),
                )?;
            }
            let namespace = if let Some(id) = self.namespace {
                store.import_namespace(id)?
            } else if self.source == ImportSourceKind::TelegramDesktop {
                indexed::telegram_namespace(&*store, &staged)
                    .map_err(import_error)?
                    .unwrap_or_else(|| {
                        indexed::new_namespace(source(self.source), coverage(self.choice))
                    })
            } else {
                indexed::new_namespace(source(self.source), coverage(self.choice))
            };
            let plan = indexed::prepare_indexed(&*store, &staged, namespace, self.choice)
                .map_err(import_error)?;
            let operation = ImportOperation {
                operation_id: Uuid::now_v7(),
                preview_id: Uuid::now_v7(),
                source: self.source,
                namespace_id: plan.namespace().namespace_id,
                legacy_choice: self.choice,
                state: ImportOperationState::Reserved,
                preview: ImportCounts {
                    participants: staged.participants.len() as u64,
                    messages: staged.messages.len() as u64,
                    duplicates: plan.duplicate_messages(),
                    contacts_created: plan.contacts_created() as u64,
                    contacts_matched: plan.contacts_matched() as u64,
                    events_written: plan.new_messages() as u64,
                    evidence_written: plan.new_evidence(&staged) as u64,
                    messages_with_injection_markers: preview.messages_with_injection_markers,
                    ties_rebuilt: 0,
                },
                receipt: None,
                error_code: None,
                audit_state: ImportAuditState::NotApplicable,
                created_at: now_unix_seconds(),
            };
            store.reserve_import_operation(&self.runtime, &operation)?;
            Ok(HeldImport {
                operation,
                revision: self.revision,
                staged,
                plan,
            })
        }))
        .unwrap_or_else(|_| {
            Err(problem(
                "IMPORT_WORKER_FAILED",
                "准备未能正常完成。请查询最近操作，旧原文和批准不会自动恢复。",
            ))
        });
        SessionImportPrepareCompletion {
            id: self.id,
            revision: self.revision,
            _lease: self.lease,
            result,
            pause,
        }
    }
}

fn coverage(choice: LegacyImportChoice) -> ImportCoverage {
    if choice == LegacyImportChoice::ForwardOnly {
        ImportCoverage::ForwardOnly
    } else {
        ImportCoverage::FullFromCreation
    }
}

impl SessionImportWork {
    pub fn execute(self) -> SessionImportCompletion {
        let pause = self.pause.map(|pause| pause.stop());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(pause) = &pause {
                if let Err(error) = pause.require_stopped() {
                    if self.held.is_some() {
                        let mut store = hold(&self.store);
                        store.end_import_operation(
                            &self.runtime,
                            self.operation.operation_id,
                            ImportOperationState::Running,
                            ImportOperationState::Rejected,
                            Some(ImportOperationError::WorkerFailed),
                        )?;
                    }
                    return Err(error);
                }
            }
            let mut store = hold(&self.store);
            if let Some(held) = self.held {
                let outcome = store.transact(|store| -> Result<ImportOperation, SessionRefusal> {
                    let receipt = indexed::commit_indexed(store, &held.staged, &held.plan)
                        .map_err(import_error)?;
                    let at = now_unix_seconds();
                    for entry in &receipt.audit {
                        append_or_store_error(store, entry.clone(), at)?;
                    }
                    let build = graph_commands::rebuild(store, at)?;
                    let counts = ImportCounts {
                        participants: held.staged.participants.len() as u64,
                        messages: held.staged.messages.len() as u64,
                        duplicates: held.plan.duplicate_messages(),
                        contacts_created: receipt.contacts_created.len() as u64,
                        contacts_matched: receipt.contacts_matched.len() as u64,
                        events_written: receipt.events_written.len() as u64,
                        evidence_written: receipt.evidence_written.len() as u64,
                        messages_with_injection_markers: receipt.messages_with_injection_markers,
                        ties_rebuilt: build.edges_written.len() as u64,
                    };
                    let operation = store.commit_import_operation(
                        &self.runtime,
                        self.operation.operation_id,
                        counts,
                    )?;
                    crate::commands::insights::create_for_import(
                        store,
                        &operation,
                        &receipt.contacts_touched(),
                        at,
                    )?;
                    Ok(operation)
                });
                if let Err(error) = outcome {
                    // The same connection must be outside any transaction before
                    // its committed receipt can prove the outcome.
                    if store.transaction_in_progress() {
                        return Err(unknown());
                    }
                    let recorded = store
                        .import_operation(self.operation.operation_id)
                        .map_err(|_| unknown())?;
                    if recorded.state != ImportOperationState::Committed {
                        store
                            .end_import_operation(
                                &self.runtime,
                                recorded.operation_id,
                                ImportOperationState::Running,
                                ImportOperationState::Rejected,
                                Some(match error.reason_code.as_str() {
                                    "IMPORT_ID_CONFLICT" => ImportOperationError::IdentityConflict,
                                    "IMPORT_SOURCE_CHANGED" => ImportOperationError::SourceChanged,
                                    "IMPORT_NAMESPACE_REVOKED" => {
                                        ImportOperationError::NamespaceRevoked
                                    }
                                    "IMPORT_IDENTITY_UNAVAILABLE" => {
                                        ImportOperationError::IdentityUnavailable
                                    }
                                    "IMPORT_STORE_UNAVAILABLE" => {
                                        ImportOperationError::StoreUnavailable
                                    }
                                    _ => ImportOperationError::WorkerFailed,
                                }),
                            )
                            .map_err(|_| unknown())?;
                        return Err(error);
                    }
                }
            }
            view(&store, self.operation.operation_id, false)
        }))
        .unwrap_or_else(|_| Err(unknown()));
        SessionImportCompletion {
            id: self.id,
            _lease: self.lease,
            result,
            pause,
        }
    }
}

fn view(
    store: &SqlCipherStore,
    id: Uuid,
    prepared_here: bool,
) -> Result<ImportOperationView, SessionRefusal> {
    let operation = store.import_operation(id)?;
    let namespace_revoked = match store.import_namespace(operation.namespace_id) {
        Ok(namespace) => namespace.state == ImportNamespaceState::Revoked,
        Err(soul_store_api::types::StoreError::NotFound { .. }) => false,
        Err(error) => return Err(error.into()),
    };
    Ok(ImportOperationView {
        operation,
        namespace_revoked,
        prepared_here,
    })
}

impl Session {
    pub fn local_insights(
        &self,
    ) -> Result<Vec<crate::commands::insights::LocalInsightView>, SessionRefusal> {
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        crate::commands::insights::views(&mut store)
    }

    pub fn ignore_local_insight(
        &mut self,
        id: &str,
    ) -> Result<crate::commands::insights::LocalInsightView, SessionRefusal> {
        let id = parse_id(id, "本地洞察")?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        crate::commands::insights::ignore(&mut store, id)
    }

    pub fn recompute_local_insight(
        &mut self,
        id: &str,
    ) -> Result<crate::commands::insights::LocalInsightView, SessionRefusal> {
        let id = parse_id(id, "本地洞察")?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        crate::commands::insights::recompute(&mut store, id, now_unix_seconds())
    }
    pub fn import_recovery_state(&self) -> Result<ImportRecoveryState, SessionRefusal> {
        if self.import_busy() {
            return Ok(ImportRecoveryState {
                active_worker: true,
                reserved_operations: None,
                running_operations: None,
                held_operation_id: None,
                can_start_new_intent: false,
            });
        }
        let shared = self.opened_store()?;
        let store = hold(&shared);
        let (reserved, running) = store.unfinished_import_counts()?;
        let held = self
            .held_import
            .as_ref()
            .filter(|held| held.revision == self.draft_revision)
            .map(|held| held.operation.operation_id);
        Ok(ImportRecoveryState {
            active_worker: false,
            reserved_operations: Some(reserved),
            running_operations: Some(running),
            held_operation_id: held,
            can_start_new_intent: running == 0 && held.is_none(),
        })
    }
    pub fn begin_import_prepare_request(
        &mut self,
        request: ImportPrepareRequest,
    ) -> Result<SessionImportPrepareWork, SessionRefusal> {
        self.begin_import_prepare(
            request.text,
            request.source,
            request.namespace_id,
            request.legacy_choice,
        )
    }
    pub(super) fn import_busy(&self) -> bool {
        self.active_draft.as_ref().is_some_and(|active| {
            active.kind == E1WorkerKind::Import && !active.finished.load(Ordering::Acquire)
        })
    }

    pub(super) fn require_store_idle(&self) -> Result<(), SessionRefusal> {
        if self.store_work_busy() {
            if self.active_draft.as_ref().is_some_and(|a| {
                matches!(
                    a.kind,
                    E1WorkerKind::Assistance | E1WorkerKind::LocalPreview
                )
            }) {
                Err(problem("ASSISTANCE_BUSY","本地辅助工作正在使用加密库，请稍后重试读取或修改。端点清除和采集撤回仍可使用；本次拒绝尚未写入审计。"))
            } else if self
                .active_draft
                .as_ref()
                .is_some_and(|a| a.kind == E1WorkerKind::ReminderCheck)
            {
                Err(problem("REMINDER_BUSY","提醒正在核对本地来源，请稍后重试读取或修改。停止提醒和采集撤回仍可使用；本次拒绝尚未写入审计。"))
            } else {
                Err(problem("IMPORT_BUSY", "导入正在使用本机加密库，请等本次操作结束后再读取或修改。端点清除和采集撤回仍可使用；本次拒绝尚未写入审计。"))
            }
        } else {
            Ok(())
        }
    }
    pub(super) fn store_work_busy(&self) -> bool {
        self.active_draft.as_ref().is_some_and(|active| {
            matches!(
                active.kind,
                E1WorkerKind::Import
                    | E1WorkerKind::Assistance
                    | E1WorkerKind::ReminderCheck
                    | E1WorkerKind::LocalPreview
            ) && !active.finished.load(Ordering::Acquire)
        })
    }

    pub(super) fn flush_deferred_revoke(&mut self) -> Result<(), SessionRefusal> {
        if let Some((entry, at)) = self.deferred_revoke.clone() {
            let store = self.opened_store()?;
            if let Err(error) =
                collect_commands::record_consent_change(&mut hold(&store), entry, at)
            {
                self.collect_problem = Some("权限已撤回，撤回审计仍未确认。".to_owned());
                return Err(error.into());
            }
            self.deferred_revoke = None;
            self.collect_problem = None;
        }
        Ok(())
    }

    pub fn import_namespaces(&self) -> Result<Vec<ImportNamespaceView>, SessionRefusal> {
        let shared = self.opened_store()?;
        let store = hold(&shared);
        Ok(store
            .import_namespaces()?
            .into_iter()
            .map(|namespace| ImportNamespaceView {
                namespace_id: namespace.namespace_id,
                source: namespace.source,
                state: namespace.state,
                coverage: namespace.coverage,
            })
            .collect())
    }

    pub fn recent_import_operations(&self) -> Result<Vec<ImportOperationView>, SessionRefusal> {
        let shared = self.opened_store()?;
        let store = hold(&shared);
        store
            .recent_import_operations(20)?
            .iter()
            .map(|operation| {
                view(
                    &store,
                    operation.operation_id,
                    self.held_import.as_ref().is_some_and(|held| {
                        held.operation.operation_id == operation.operation_id
                            && held.revision == self.draft_revision
                    }),
                )
            })
            .collect()
    }

    pub fn import_operation(&self, id: &str) -> Result<ImportOperationView, SessionRefusal> {
        let id = parse_id(id, "导入操作")?;
        let shared = self.opened_store()?;
        let store = hold(&shared);
        view(
            &store,
            id,
            self.held_import.as_ref().is_some_and(|held| {
                held.operation.operation_id == id && held.revision == self.draft_revision
            }),
        )
    }

    pub fn begin_import_prepare(
        &mut self,
        text: String,
        source: ImportSourceKind,
        namespace: Option<Uuid>,
        choice: LegacyImportChoice,
    ) -> Result<SessionImportPrepareWork, SessionRefusal> {
        self.require_draft_slot()?;
        if self
            .held_import
            .as_ref()
            .is_some_and(|held| held.revision == self.draft_revision)
        {
            return Err(problem(
                "IMPORT_PREPARATION_HELD",
                "已有一份仍有效的准备。请按它的操作编号明确丢弃，再准备新文件。切页不会替你丢弃。",
            ));
        }
        self.invalidate_drafts();
        import_commands::within_byte_budget(&text).map_err(over_budget)?;
        let store = self.opened_store()?;
        let runtime = self.import_runtime.clone().ok_or_else(unknown)?;
        self.flush_deferred_revoke()?;
        let id = Uuid::now_v7();
        let finished = Arc::new(AtomicBool::new(false));
        self.active_draft = Some(ActiveDraft {
            id,
            kind: E1WorkerKind::Import,
            finished: Arc::clone(&finished),
        });
        Ok(SessionImportPrepareWork {
            id,
            revision: self.draft_revision,
            lease: DraftLease(finished),
            runtime,
            store,
            text,
            source,
            namespace,
            choice,
            stale: self.stale_import.take(),
            pause: self.take_import_collector(),
        })
    }

    pub fn complete_import_prepare(
        &mut self,
        completed: SessionImportPrepareCompletion,
    ) -> Result<PreparedImportView, SessionRefusal> {
        if !self
            .active_draft
            .as_ref()
            .is_some_and(|active| active.id == completed.id && active.kind == E1WorkerKind::Import)
        {
            return Err(changed());
        }
        self.active_draft = None;
        self.flush_deferred_revoke()?;
        self.finish_import_collector_pause(completed.pause);
        let held = completed.result?;
        if completed.revision != self.draft_revision {
            let store = self.opened_store()?;
            hold(&store).end_import_operation(
                self.import_runtime.as_ref().ok_or_else(unknown)?,
                held.operation.operation_id,
                ImportOperationState::Reserved,
                ImportOperationState::Invalidated,
                Some(ImportOperationError::SourceChanged),
            )?;
            return Err(changed());
        }
        let result = PreparedImportView { operation: held.operation.clone(), preview: ImportPreview::of(&held.staged),
            legacy_events: held.plan.legacy_events(), notice: "只导入本机加密库。确认只引用这份核心持有的准备，不再次发送文件正文。相同来源编号的相同消息跳过，变化则整体拒绝；缺失消息不删除历史。".to_owned() };
        self.held_import = Some(held);
        Ok(result)
    }

    pub fn begin_import(
        &mut self,
        operation_id: &str,
        preview_id: &str,
    ) -> Result<SessionImportWork, SessionRefusal> {
        self.require_draft_slot()?;
        let operation_id = parse_id(operation_id, "导入操作")?;
        let preview_id = parse_id(preview_id, "导入预览")?;
        let store = self.opened_store()?;
        let runtime = self.import_runtime.clone().ok_or_else(unknown)?;
        let current = hold(&store).import_operation(operation_id)?;
        if current.preview_id != preview_id {
            return Err(changed());
        }
        let (operation, held) = if current.state == ImportOperationState::Committed {
            (current, None)
        } else {
            let held = self.held_import.as_ref().ok_or_else(changed)?;
            if held.operation.operation_id != operation_id
                || held.operation.preview_id != preview_id
                || held.revision != self.draft_revision
            {
                return Err(changed());
            }
            let operation =
                hold(&store).start_import_operation(&runtime, operation_id, preview_id)?;
            (operation, self.held_import.take())
        };
        if held.is_some() {
            self.invalidate_drafts();
        }
        let id = Uuid::now_v7();
        let finished = Arc::new(AtomicBool::new(false));
        self.active_draft = Some(ActiveDraft {
            id,
            kind: E1WorkerKind::Import,
            finished: Arc::clone(&finished),
        });
        let pause = if held.is_some() {
            self.take_import_collector()
        } else {
            None
        };
        Ok(SessionImportWork {
            id,
            lease: DraftLease(finished),
            runtime,
            store,
            operation,
            held,
            pause,
        })
    }

    pub fn complete_import(
        &mut self,
        completed: SessionImportCompletion,
    ) -> Result<ImportOperationView, SessionRefusal> {
        if !self
            .active_draft
            .as_ref()
            .is_some_and(|active| active.id == completed.id && active.kind == E1WorkerKind::Import)
        {
            return Err(changed());
        }
        self.active_draft = None;
        // A post-commit audit problem cannot erase an acknowledged import.
        let _ = self.flush_deferred_revoke();
        self.sync_identifiers();
        self.finish_import_collector_pause(completed.pause);
        completed.result
    }

    pub fn discard_import(
        &mut self,
        operation_id: &str,
    ) -> Result<ImportOperationView, SessionRefusal> {
        let id = parse_id(operation_id, "导入操作")?;
        let store = self.opened_store()?;
        let held = self.held_import.as_ref().ok_or_else(changed)?;
        if held.operation.operation_id != id {
            return Err(changed());
        }
        hold(&store).end_import_operation(
            self.import_runtime.as_ref().ok_or_else(unknown)?,
            id,
            ImportOperationState::Reserved,
            ImportOperationState::Discarded,
            None,
        )?;
        self.held_import = None;
        self.invalidate_drafts();
        self.import_operation(operation_id)
    }

    pub fn refuse_legacy_import(&self, _text: &str) -> Result<ImportReceiptView, SessionRefusal> {
        Err(problem(
            "IMPORT_PREPARATION_REQUIRED",
            "此旧提交入口缺少来源与不可变准备。请使用导入页重新准备，再确认操作编号。",
        ))
    }
    pub fn refuse_legacy_preview(&self, _text: &str) -> Result<ImportPreview, SessionRefusal> {
        Err(problem(
            "IMPORT_PREPARATION_REQUIRED",
            "请使用不可变导入准备；旧同步预览入口已停止接收正文。",
        ))
    }
}
