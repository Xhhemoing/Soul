//! The IPC surface, and nothing else.
//!
//! Commands forward to `soulcore::commands`. Endpoint drafting additionally
//! schedules its owned work on the blocking pool: the Session mutex protects
//! preparation/acceptance, never the HTTP wait. Policy decisions stay in core.
//!
//! One thing this file does *not* do is open a database. WP07 leftover 8 says
//! the process gets one `SqlCipherStore` handle to keep key material,
//! transaction ownership and audit ordering together. SQLite itself allows
//! multiple connections to the same WAL. The handle opens once in `lib.rs`, lives
//! inside `soulcore::commands::session::Session`, and every command here
//! borrows it through [`SessionState`]. `tests/one_store.rs` reads this file
//! and `lib.rs` back to check that no second opening has appeared.

use std::sync::{Arc, Mutex, MutexGuard};

use soul_runtime::{Caller, Runtime, RuntimeContext};
use soul_types::{
    MemoryDetailView, MemoryIdRequest, PeoplePage, PeoplePageRequest, ReadError, RuntimeStatus,
};
use soulcore::commands::draft::{
    Approval, DraftRefusal, DraftValue, E1DraftPlan, PersonSummaryView,
};
use soulcore::commands::fileplan::PlanPreview;
use soulcore::commands::graph::PeopleGraphView;
use soulcore::commands::import::{ImportPreview, ImportReceiptView};
use soulcore::commands::insights::LocalInsightView;
use soulcore::commands::memory::{
    ForgetCleanupView, ForgetConfirmation, ForgetPreview, ForgetReceiptView, MemoryChange,
    MemoryDetail, MemoryList, NewMemory,
};
use soulcore::commands::profile::{GivenAnswer, IntakeReceipt, ProfileScreen, QuestionView};
use soulcore::commands::session::{
    AssistanceActionRequest, AssistanceCategoryPreference, AssistanceCategoryPreferences,
    AssistanceContextView, AssistanceFeedbackView, AssistanceItemDetail, AssistanceItemRequest,
    AssistanceItems, AssistanceLocalPreparation, AssistanceLocalTemplatePreview,
    AssistanceNotificationRuntimeStatus, AssistanceReminderView, AssistanceWorkMetadata,
    AssistanceWorkStatus, BindAssistanceGoalSource, CollectStatus, ContextMemoryId,
    CreateAssistanceGoal, EditAssistanceGoal, FilesView, ImportNamespaceView, ImportOperationView,
    ImportPrepareRequest, ImportRecoveryState, InsightContextRequest, InsightContextView,
    InsightDraftPlan, InsightDraftValue, LinkAssistanceObservation, MemoryContextRequest,
    MemoryContextView, MemoryDraftPlan, MemoryDraftValue, NotificationWithdrawal,
    PreparedImportView, PreviewAssistanceLocalTemplate, RefreshAssistanceObservation,
    RetryAssistanceNotificationWithdrawal, Session, SessionRefusal, SessionStatus,
    SetAssistanceCategoryPreference, SetAssistanceFeedback, SetAssistanceLocalPreparation,
    SetAssistanceReminder, TaskContextRequest, TaskContextView,
};
use soulcore::commands::session::{
    ResearchSaveCancel, ResearchSavePrepare, ResearchSavePreview, ResearchSaveReceipt,
};
use soulcore::commands::shell::{CloudNotice, ConfigSnapshot, WizardAnswers, WizardRefused};
use soulcore::commands::sources::{
    EvidenceEventRequest, EvidenceEventView, EvidenceSourcesRequest, EvidenceSourcesView,
};
use soulcore::commands::store::{AuditChainView, ResearchPreviewView};
use tauri::State;

/// Trusted native metadata. Deliberately does not implement Deserialize: no
/// JSON field can choose the webview caller or acquire a different owner's slot.
pub struct UiCaller(Caller);

impl<'de, R: tauri::Runtime> tauri::ipc::CommandArg<'de, R> for UiCaller {
    fn from_command(
        item: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        Ok(Self(Caller::desktop(item.message.webview().label())))
    }
}

impl UiCaller {
    fn context(self, correlation_id: Option<ContextMemoryId>) -> RuntimeContext {
        RuntimeContext::new(self.0, correlation_id)
    }
}

/// The one session this process has.
///
/// Constructed by the caller rather than by [`Default`], because where the
/// data directory is depends on the machine and a test must be able to say
/// "somewhere else". `lib.rs` builds the real one; `tests/ipc_roundtrip.rs`
/// builds one in a scratch directory and gets the same commands.
#[derive(Debug, Clone)]
pub struct SessionState(Runtime);

impl SessionState {
    pub fn new(session: Session) -> SessionState {
        SessionState(Runtime::new(session))
    }

    pub fn from_runtime(runtime: Runtime) -> SessionState {
        SessionState(runtime)
    }

    pub fn runtime(&self) -> Runtime {
        self.0.clone()
    }

    pub(crate) fn shared(&self) -> Arc<Mutex<Session>> {
        self.0.shared_session()
    }

    /// The session, with a poisoned lock recovered rather than propagated.
    ///
    /// A panic in one command must not take the whole interface away for the
    /// rest of the run: what a panicking command can leave behind is a
    /// rolled-back transaction, not a half-written session.
    fn held(&self) -> MutexGuard<'_, Session> {
        self.0.held_session()
    }

    /// The explicit synthetic host must not enable real-data capabilities.
    /// Reject before core calls or spawning owned work; ordinary mode is unchanged.
    fn require_product_mode(&self) -> Result<(), SessionRefusal> {
        if self.0.is_synthetic() {
            Err(SessionRefusal {
                reason_code: "SYNTHETIC_MODE_RESTRICTED".into(),
                explanation: "This operation is unavailable in synthetic API mode.".into(),
            })
        } else {
            Ok(())
        }
    }
}

#[tauri::command]
pub fn assistance_local_preparation(
    session: State<'_, SessionState>,
    item_id: String,
) -> Result<AssistanceLocalPreparation, SessionRefusal> {
    session.held().assistance_local_preparation(&item_id)
}

#[tauri::command]
pub fn set_assistance_local_preparation(
    session: State<'_, SessionState>,
    request: SetAssistanceLocalPreparation,
) -> Result<AssistanceLocalPreparation, SessionRefusal> {
    session.held().set_assistance_local_preparation(request)
}

#[tauri::command]
pub async fn preview_assistance_local_template(
    session: State<'_, SessionState>,
    request: PreviewAssistanceLocalTemplate,
) -> Result<AssistanceLocalTemplatePreview, SessionRefusal> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let work = session
            .held()
            .begin_assistance_local_template_preview(request)?;
        let completed = work.execute();
        let result = session
            .held()
            .complete_assistance_local_template_preview(completed);
        result
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub fn assistance_reminder(
    session: State<'_, SessionState>,
    item_id: String,
) -> Result<AssistanceReminderView, SessionRefusal> {
    session.held().assistance_reminder(&item_id)
}

#[tauri::command]
pub fn set_assistance_reminder(
    session: State<'_, SessionState>,
    request: SetAssistanceReminder,
) -> Result<AssistanceReminderView, SessionRefusal> {
    session.held().set_assistance_reminder(request)
}

#[tauri::command]
pub fn assistance_feedback(
    session: State<'_, SessionState>,
    item_id: String,
) -> Result<AssistanceFeedbackView, SessionRefusal> {
    session.held().assistance_feedback(&item_id)
}

#[tauri::command]
pub fn set_assistance_feedback(
    session: State<'_, SessionState>,
    request: SetAssistanceFeedback,
) -> Result<AssistanceFeedbackView, SessionRefusal> {
    session.held().set_assistance_feedback(request)
}

#[tauri::command]
pub fn assistance_category_preferences(
    session: State<'_, SessionState>,
) -> Result<AssistanceCategoryPreferences, SessionRefusal> {
    session.held().assistance_category_preferences()
}

#[tauri::command]
pub fn set_assistance_category_preference(
    session: State<'_, SessionState>,
    request: SetAssistanceCategoryPreference,
) -> Result<AssistanceCategoryPreference, SessionRefusal> {
    session.held().set_assistance_category_preference(request)
}

#[tauri::command]
pub fn retry_assistance_notification_withdrawal(
    session: State<'_, SessionState>,
    request: RetryAssistanceNotificationWithdrawal,
) -> Result<NotificationWithdrawal, SessionRefusal> {
    session
        .held()
        .retry_assistance_notification_withdrawal(request)
}

#[tauri::command]
pub fn assistance_notification_runtime_status(
    session: State<'_, SessionState>,
) -> AssistanceNotificationRuntimeStatus {
    session.held().assistance_notification_runtime_status()
}

#[tauri::command]
pub fn assistance_work_status(
    session: State<'_, SessionState>,
) -> Result<AssistanceWorkStatus, SessionRefusal> {
    session.held().assistance_work_status()
}

#[tauri::command]
pub fn assistance_item_work(
    session: State<'_, SessionState>,
    item_id: String,
) -> Result<AssistanceWorkMetadata, SessionRefusal> {
    session.held().assistance_item_work(&item_id)
}

#[tauri::command]
pub fn retry_assistance_work(
    session: State<'_, SessionState>,
    request: AssistanceItemRequest,
) -> Result<AssistanceWorkMetadata, SessionRefusal> {
    session.held().retry_assistance_work(request)
}

#[tauri::command]
pub fn assistance_items(
    session: State<'_, SessionState>,
) -> Result<AssistanceItems, SessionRefusal> {
    session.held().assistance_items()
}

#[tauri::command]
pub fn assistance_item(
    session: State<'_, SessionState>,
    item_id: String,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().assistance_item(&item_id)
}

#[tauri::command]
pub fn create_assistance_goal(
    session: State<'_, SessionState>,
    request: CreateAssistanceGoal,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().create_assistance_goal(request)
}

#[tauri::command]
pub fn edit_assistance_goal(
    session: State<'_, SessionState>,
    request: EditAssistanceGoal,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().edit_assistance_goal(request)
}

#[tauri::command]
pub fn link_assistance_observation(
    session: State<'_, SessionState>,
    request: LinkAssistanceObservation,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().link_assistance_observation(request)
}

#[tauri::command]
pub fn refresh_assistance_observation(
    session: State<'_, SessionState>,
    request: RefreshAssistanceObservation,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().refresh_assistance_observation(request)
}

#[tauri::command]
pub fn bind_assistance_goal_source(
    session: State<'_, SessionState>,
    request: BindAssistanceGoalSource,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().bind_assistance_goal_source(request)
}

#[tauri::command]
pub fn act_assistance_item(
    session: State<'_, SessionState>,
    request: AssistanceActionRequest,
) -> Result<AssistanceItemDetail, SessionRefusal> {
    session.held().act_assistance_item(request)
}

#[tauri::command]
pub fn prepare_assistance_context(
    session: State<'_, SessionState>,
    request: AssistanceItemRequest,
) -> Result<AssistanceContextView, SessionRefusal> {
    session.held().prepare_assistance_context(request)
}

#[tauri::command]
pub fn evidence_sources(
    session: State<'_, SessionState>,
    request: EvidenceSourcesRequest,
) -> Result<EvidenceSourcesView, SessionRefusal> {
    session.held().evidence_sources(&request)
}

#[tauri::command]
pub fn read_evidence_event(
    session: State<'_, SessionState>,
    request: EvidenceEventRequest,
) -> Result<EvidenceEventView, SessionRefusal> {
    session.held().read_evidence_event(&request)
}

#[tauri::command]
pub fn config_snapshot(session: State<'_, SessionState>) -> ConfigSnapshot {
    session.held().snapshot()
}

#[tauri::command]
pub fn local_insights(
    session: State<'_, SessionState>,
) -> Result<Vec<LocalInsightView>, SessionRefusal> {
    session.held().local_insights()
}

#[tauri::command]
pub fn ignore_local_insight(
    session: State<'_, SessionState>,
    bundle_id: String,
) -> Result<LocalInsightView, SessionRefusal> {
    session.held().ignore_local_insight(&bundle_id)
}

#[tauri::command]
pub fn recompute_local_insight(
    session: State<'_, SessionState>,
    bundle_id: String,
) -> Result<LocalInsightView, SessionRefusal> {
    session.held().recompute_local_insight(&bundle_id)
}

#[tauri::command]
pub fn prepare_insight_context(
    session: State<'_, SessionState>,
    request: InsightContextRequest,
) -> Result<InsightContextView, SessionRefusal> {
    session.held().prepare_insight_context(request)
}

#[tauri::command]
pub fn local_draft_with_insight(
    session: State<'_, SessionState>,
    pasted: String,
    context_id: String,
) -> Result<InsightDraftValue, SessionRefusal> {
    session
        .held()
        .local_draft_with_insight(&pasted, &context_id)
}

#[tauri::command]
pub fn prepare_draft_with_insight(
    session: State<'_, SessionState>,
    pasted: String,
    context_id: String,
    include_counts: bool,
    include_original: bool,
    include_goal: Option<bool>,
) -> Result<InsightDraftPlan, SessionRefusal> {
    session.held().prepare_draft_with_insight_and_goal(
        &pasted,
        &context_id,
        include_counts,
        include_original,
        include_goal.unwrap_or(false),
    )
}

#[tauri::command]
pub fn prepare_memory_context(
    session: State<'_, SessionState>,
    request: MemoryContextRequest,
) -> Result<MemoryContextView, SessionRefusal> {
    session.held().prepare_memory_context(request)
}

#[tauri::command]
pub fn prepare_task_context(
    session: State<'_, SessionState>,
    request: TaskContextRequest,
) -> Result<TaskContextView, SessionRefusal> {
    session.held().prepare_task_context(request)
}

#[tauri::command]
pub fn draft_reply_with_memory(
    session: State<'_, SessionState>,
    pasted: String,
    context_id: String,
) -> Result<MemoryDraftValue, SessionRefusal> {
    session
        .held()
        .draft_pasted_with_memory(&pasted, &context_id)
}

#[tauri::command]
pub fn prepare_draft_with_memory(
    session: State<'_, SessionState>,
    pasted: String,
    context_id: String,
    owner_ids: Vec<ContextMemoryId>,
    include_original: bool,
) -> Result<MemoryDraftPlan, SessionRefusal> {
    session
        .held()
        .prepare_draft_with_memory(&pasted, &context_id, &owner_ids, include_original)
}

#[tauri::command]
pub fn list_import_namespaces(
    session: State<'_, SessionState>,
) -> Result<Vec<ImportNamespaceView>, SessionRefusal> {
    session.held().import_namespaces()
}

#[tauri::command]
pub fn import_recovery_state(
    session: State<'_, SessionState>,
) -> Result<ImportRecoveryState, SessionRefusal> {
    session.held().import_recovery_state()
}

#[tauri::command]
pub fn recent_import_operations(
    session: State<'_, SessionState>,
) -> Result<Vec<ImportOperationView>, SessionRefusal> {
    session.held().recent_import_operations()
}

#[tauri::command]
pub fn import_operation(
    session: State<'_, SessionState>,
    operation_id: String,
) -> Result<ImportOperationView, SessionRefusal> {
    session.0.import_operation(&operation_id)
}

#[tauri::command]
pub fn discard_import(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    operation_id: String,
) -> Result<ImportOperationView, SessionRefusal> {
    session
        .0
        .discard_import(&caller.context(correlation_id), &operation_id)
}

#[tauri::command]
pub async fn prepare_import(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    request: ImportPrepareRequest,
) -> Result<PreparedImportView, SessionRefusal> {
    session.require_product_mode()?;
    let runtime = session.runtime();
    let context = caller.context(correlation_id);
    tauri::async_runtime::spawn_blocking(move || runtime.prepare_import(&context, request))
        .await
        .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub async fn confirm_import(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    operation_id: String,
    preview_id: String,
) -> Result<ImportOperationView, SessionRefusal> {
    let runtime = session.runtime();
    let context = caller.context(correlation_id);
    tauri::async_runtime::spawn_blocking(move || {
        runtime.confirm_import(&context, &operation_id, &preview_id)
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

/// Whether the wizard has been through before, and whether there is a
/// database to read. Both are answers only the core can give.
#[tauri::command]
pub fn session_status(session: State<'_, SessionState>) -> SessionStatus {
    session.0.legacy_status()
}

#[tauri::command]
pub fn runtime_status(session: State<'_, SessionState>) -> RuntimeStatus {
    session.0.status()
}

#[tauri::command]
pub fn runtime_event_snapshot(
    session: State<'_, SessionState>,
) -> soul_types::RuntimeEventSnapshot {
    session.0.event_snapshot()
}

#[tauri::command]
pub async fn prepare_synthetic_import(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    request: soul_types::SyntheticImportRequest,
) -> Result<PreparedImportView, SessionRefusal> {
    let runtime = session.runtime();
    let context = caller.context(correlation_id);
    tauri::async_runtime::spawn_blocking(move || {
        runtime.prepare_synthetic_import(&context, request)
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub fn runtime_people(
    session: State<'_, SessionState>,
    request: PeoplePageRequest,
) -> Result<PeoplePage, ReadError> {
    session.0.people(request)
}

#[tauri::command]
pub fn runtime_memory(
    session: State<'_, SessionState>,
    request: MemoryIdRequest,
) -> Result<MemoryDetailView, ReadError> {
    session.0.memory(request)
}

#[tauri::command]
pub fn complete_wizard(
    session: State<'_, SessionState>,
    answers: WizardAnswers,
) -> Result<ConfigSnapshot, WizardRefused> {
    session.held().complete_wizard(&answers)
}

#[tauri::command]
pub fn cloud_toggle(session: State<'_, SessionState>, requested_on: bool) -> CloudNotice {
    session.held().cloud_toggle(requested_on)
}

/// The directories the user has authorized, and the ones that have gone.
#[tauri::command]
pub fn files_view(session: State<'_, SessionState>) -> FilesView {
    session.held().files()
}

#[tauri::command]
pub fn authorize_directory(
    session: State<'_, SessionState>,
    path: String,
) -> Result<FilesView, SessionRefusal> {
    session.require_product_mode()?;
    session.held().authorize(&path)
}

/// Scan one authorized directory and describe what tidying it would mean.
/// Read-only: there is no command on this surface that carries a plan out.
#[tauri::command]
pub async fn preview_plan(
    session: State<'_, SessionState>,
    path: String,
) -> Result<PlanPreview, SessionRefusal> {
    session.require_product_mode()?;
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let work = session.held().begin_file_plan(&path)?;
        let completed = work.execute();
        let result = session.held().complete_file_plan(completed);
        result
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub fn people_graph(session: State<'_, SessionState>) -> Result<PeopleGraphView, SessionRefusal> {
    session.0.legacy_people()
}

#[tauri::command]
pub async fn person_summary(
    session: State<'_, SessionState>,
    contact_id: String,
) -> Result<PersonSummaryView, SessionRefusal> {
    session.require_product_mode()?;
    let session = session.inner().clone();
    // The owned closure completes and audits even if the IPC waiter leaves.
    tauri::async_runtime::spawn_blocking(move || {
        let work = session.held().begin_person_summary(&contact_id)?;
        let completed = work.execute();
        let result = session.held().complete_person_summary(completed);
        result
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub fn correct_tie(
    session: State<'_, SessionState>,
    relationship_id: String,
    band: String,
) -> Result<PeopleGraphView, SessionRefusal> {
    session.held().correct_tie(&relationship_id, &band)
}

#[tauri::command]
pub fn release_tie(
    session: State<'_, SessionState>,
    relationship_id: String,
) -> Result<PeopleGraphView, SessionRefusal> {
    session.held().release_tie(&relationship_id)
}

/// Draft a reply to something the user pasted. Never sends it, and on this
/// path never builds a request body either.
#[tauri::command]
pub fn draft_reply(
    session: State<'_, SessionState>,
    pasted: String,
) -> Result<DraftValue, SessionRefusal> {
    session.held().draft_pasted(&pasted)
}

#[tauri::command]
pub fn scoped_register_recipients(
    session: State<'_, SessionState>,
) -> Result<Vec<soulcore::commands::session::ScopedRegisterRecipient>, SessionRefusal> {
    session.held().scoped_register_recipients()
}

#[tauri::command]
pub fn scoped_register_status(
    session: State<'_, SessionState>,
    person_id: String,
    occasion: soulcore::commands::session::PreferenceOccasion,
) -> Result<soulcore::commands::session::ScopedRegisterStatus, SessionRefusal> {
    session.0.scoped_register_status(&person_id, occasion)
}

#[tauri::command]
pub fn draft_scoped_reply(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    pasted: String,
    person_id: String,
    occasion: soulcore::commands::session::PreferenceOccasion,
) -> Result<soulcore::commands::session::ScopedDraftView, SessionRefusal> {
    session.0.draft_scoped_reply(
        &caller.context(correlation_id),
        &pasted,
        &person_id,
        occasion,
    )
}

#[tauri::command]
pub fn propose_scoped_register(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    result_id: String,
    edit_revision: u64,
    register: soulcore::commands::session::VoiceRegister,
) -> Result<soulcore::commands::session::ScopedRegisterProposal, SessionRefusal> {
    session.0.propose_scoped_register(
        &caller.context(correlation_id),
        &result_id,
        edit_revision,
        register,
    )
}

#[tauri::command]
pub fn confirm_scoped_register(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    proposal_id: String,
    expected_revision: Option<soulcore::commands::session::ContextMemoryId>,
) -> Result<soulcore::commands::session::ScopedRegisterReceipt, SessionRefusal> {
    session.0.confirm_scoped_register(
        &caller.context(correlation_id),
        &proposal_id,
        expected_revision,
    )
}

#[tauri::command]
pub fn discard_scoped_register_proposal(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    proposal_id: String,
) -> Result<bool, SessionRefusal> {
    session
        .0
        .discard_scoped_register_proposal(&caller.context(correlation_id), &proposal_id)
}

/// Step one of the endpoint path: describe the request, and stop.
///
/// `include_original` is the user's second confirmation, and it is optional so
/// that a call which does not mention it is the placeheld one. What an absent
/// answer means is the core's to decide, so it is forwarded as it arrived.
#[tauri::command]
pub fn prepare_draft(
    session: State<'_, SessionState>,
    pasted: String,
    include_original: Option<bool>,
) -> Result<E1DraftPlan, SessionRefusal> {
    session.held().prepare_draft(&pasted, include_original)
}

/// Step two: the user read the counts on screen and approved this exact
/// preparation. An approval that does not echo both halves sends nothing.
#[tauri::command]
pub async fn generate_draft(
    session: State<'_, SessionState>,
    approval: Approval,
) -> Result<DraftValue, SessionRefusal> {
    session.require_product_mode()?;
    let session = session.inner().clone();
    // The closure owns the Session handle and completes/audits even when the
    // IPC future is dropped. Aborting an await must not abandon a live worker.
    tauri::async_runtime::spawn_blocking(move || {
        let work = session.held().begin_draft(&approval)?;
        let completed = work.execute();
        let result = session.held().complete_draft(completed);
        result
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

/// The user read the plan and said no.
#[tauri::command]
pub fn discard_draft(session: State<'_, SessionState>) -> bool {
    session.held().discard_draft()
}

/// The user entered their own OpenAI-compatible endpoint. This process only:
/// there is no field in `config.json` that could carry it to the next launch,
/// and nothing is contacted by naming it.
#[tauri::command]
pub fn set_user_endpoint(
    session: State<'_, SessionState>,
    url: String,
) -> Result<ConfigSnapshot, SessionRefusal> {
    session.require_product_mode()?;
    session.held().set_user_endpoint(&url)
}

/// The user took the address away again. The guard goes back to refusing every
/// origin, which is the state a fresh launch is in.
#[tauri::command]
pub fn clear_user_endpoint(session: State<'_, SessionState>) -> ConfigSnapshot {
    session.held().clear_user_endpoint()
}

/// What a `soul-import-v1` file contains, as counts. Writes nothing.
#[tauri::command]
pub fn preview_soul_import_v1(
    session: State<'_, SessionState>,
    text: String,
) -> Result<ImportPreview, SessionRefusal> {
    session.require_product_mode()?;
    session.held().refuse_legacy_preview(&text)
}

/// The same for a Telegram Desktop `result.json`. Writes nothing.
#[tauri::command]
pub fn preview_telegram(
    session: State<'_, SessionState>,
    text: String,
) -> Result<ImportPreview, SessionRefusal> {
    session.require_product_mode()?;
    session.held().refuse_legacy_preview(&text)
}

/// Seal a `soul-import-v1` file into the store. AC-04.
#[tauri::command]
pub fn commit_soul_import_v1(
    session: State<'_, SessionState>,
    text: String,
) -> Result<ImportReceiptView, SessionRefusal> {
    session.require_product_mode()?;
    session.held().refuse_legacy_import(&text)
}

/// Seal a Telegram Desktop export into the store. AC-05.
#[tauri::command]
pub fn commit_telegram(
    session: State<'_, SessionState>,
    text: String,
) -> Result<ImportReceiptView, SessionRefusal> {
    session.require_product_mode()?;
    session.held().refuse_legacy_import(&text)
}

/// The eleven questions the wizard draws, with their options.
#[tauri::command]
pub fn questionnaire(session: State<'_, SessionState>) -> Vec<QuestionView> {
    session.held().questionnaire()
}

/// Record what the user answered. Blank answers leave their axes `unknown`.
#[tauri::command]
pub fn answer_questionnaire(
    session: State<'_, SessionState>,
    answers: Vec<GivenAnswer>,
) -> Result<IntakeReceipt, SessionRefusal> {
    session.held().answer_questionnaire(&answers)
}

#[tauri::command]
pub fn profile_screen(session: State<'_, SessionState>) -> Result<ProfileScreen, SessionRefusal> {
    session.held().profile()
}

/// The user corrected an axis. AC-07: it is pinned against later inference.
#[tauri::command]
pub fn correct_axis(
    session: State<'_, SessionState>,
    axis_id: String,
    position: String,
) -> Result<ProfileScreen, SessionRefusal> {
    session.held().correct_axis(&axis_id, &position)
}

#[tauri::command]
pub fn set_voice(
    session: State<'_, SessionState>,
    field: String,
    option: String,
) -> Result<ProfileScreen, SessionRefusal> {
    session.held().set_voice(&field, &option)
}

#[tauri::command]
pub fn memory_list(session: State<'_, SessionState>) -> Result<MemoryList, SessionRefusal> {
    session.held().memories()
}

#[tauri::command]
pub fn memory_detail(
    session: State<'_, SessionState>,
    memory_id: String,
) -> Result<MemoryDetail, SessionRefusal> {
    session.0.legacy_memory(&memory_id)
}

#[tauri::command]
pub fn create_memory(
    session: State<'_, SessionState>,
    memory: NewMemory,
) -> Result<MemoryDetail, SessionRefusal> {
    session.held().write_memory(&memory)
}

#[tauri::command]
pub fn update_memory(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    memory_id: String,
    change: MemoryChange,
) -> Result<MemoryDetail, SessionRefusal> {
    session
        .0
        .edit_memory(&caller.context(correlation_id), &memory_id, &change)
}

/// What forgetting this memory would cost. Reading the price destroys nothing.
#[tauri::command]
pub fn preview_forget(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    memory_id: String,
) -> Result<ForgetPreview, SessionRefusal> {
    session
        .0
        .preview_forget(&caller.context(correlation_id), &memory_id)
}

/// Destroy the content key behind one memory, after the user echoed the
/// preview they were shown. Irreversible, and not a file write.
#[tauri::command]
pub fn forget_memory(
    session: State<'_, SessionState>,
    caller: UiCaller,
    correlation_id: Option<ContextMemoryId>,
    confirmation: ForgetConfirmation,
) -> Result<ForgetReceiptView, SessionRefusal> {
    session
        .0
        .forget_memory(&caller.context(correlation_id), &confirmation)
}

/// Only retry the existing store's WAL cleanup, never the destructive act.
#[tauri::command]
pub fn retry_forget_cleanup(
    session: State<'_, SessionState>,
) -> Result<ForgetCleanupView, SessionRefusal> {
    soulcore::commands::memory::retry_cleanup_for_session(&session.held())
}

/// What the research track would see. On screen only; nothing is written.
#[tauri::command]
pub fn research_preview(
    session: State<'_, SessionState>,
) -> Result<ResearchPreviewView, SessionRefusal> {
    session.held().research()
}

#[tauri::command]
pub async fn prepare_research_save(
    session: State<'_, SessionState>,
    request: ResearchSavePrepare,
) -> Result<ResearchSavePreview, SessionRefusal> {
    session.require_product_mode()?;
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || session.held().prepare_research_save(request))
        .await
        .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub async fn confirm_research_save(
    session: State<'_, SessionState>,
    request_id: String,
) -> Result<ResearchSaveReceipt, SessionRefusal> {
    session.require_product_mode()?;
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let work = session.held().begin_research_save(&request_id)?;
        let completed = work.execute();
        let result = session.held().complete_research_save(completed);
        result
    })
    .await
    .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

#[tauri::command]
pub async fn cancel_research_save(
    session: State<'_, SessionState>,
    request_id: String,
) -> Result<ResearchSaveCancel, SessionRefusal> {
    let session = session.inner().clone();
    tauri::async_runtime::spawn_blocking(move || session.held().cancel_research_save(&request_id))
        .await
        .map_err(|_| SessionRefusal::from(DraftRefusal::WorkerFailed))?
}

/// The audit chain, played back and checked.
#[tauri::command]
pub fn audit_chain(session: State<'_, SessionState>) -> Result<AuditChainView, SessionRefusal> {
    session.held().audit()
}

/// Whether collection may run, whether it is running, and how much it wrote.
#[tauri::command]
pub fn collect_status(session: State<'_, SessionState>) -> CollectStatus {
    session.held().collect_status()
}

/// The user said foreground duration may be collected. This process only:
/// there is no field in `config.json` that could carry it to the next launch.
#[tauri::command]
pub fn grant_collect_consent(
    session: State<'_, SessionState>,
) -> Result<CollectStatus, SessionRefusal> {
    session.require_product_mode()?;
    session.held().grant_collect_consent()
}

/// The user took it back. Nothing further is written within a second.
#[tauri::command]
pub fn revoke_collect_consent(
    session: State<'_, SessionState>,
) -> Result<CollectStatus, SessionRefusal> {
    session.held().revoke_collect_consent()
}

/// What the drafting screen says before there is a draft on it.
///
/// Read over the IPC rather than written in TypeScript, for the reason WP09
/// gave the cloud notice the same treatment: it is a promise about what this
/// build does, and a promise kept in the interface is one the Rust tests
/// cannot check.
#[tauri::command]
pub fn draft_notices() -> DraftNotices {
    DraftNotices::of_this_build()
}

/// The sentences the drafting screen renders verbatim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftNotices {
    /// `soulcore`'s constant, not a paraphrase of it.
    pub not_sent: String,
    /// What the user reads before approving a generation request. Also
    /// `soulcore`'s constant.
    pub e1_plan: String,
    /// Always false. There is no command on this surface that sends anything,
    /// and `tests/command_surface.rs` is what keeps that list short.
    pub can_send: bool,
}

impl DraftNotices {
    fn of_this_build() -> DraftNotices {
        DraftNotices {
            not_sent: soulcore::commands::draft::NOT_SENT_NOTICE.to_owned(),
            e1_plan: soulcore::commands::draft::E1_PLAN_NOTICE.to_owned(),
            can_send: false,
        }
    }
}

/// The command names the WebView is allowed to call.
///
/// Spelled out so `tests/command_surface.rs` can compare this list against
/// `apps/desktop/src/core.ts`, which is the only place the other side names
/// them. A command added to one and not the other is a failing test.
pub const COMMAND_NAMES: &[&str] = &[
    "runtime_event_snapshot",
    "prepare_synthetic_import",
    "runtime_status",
    "runtime_people",
    "runtime_memory",
    "scoped_register_recipients",
    "scoped_register_status",
    "draft_scoped_reply",
    "propose_scoped_register",
    "confirm_scoped_register",
    "discard_scoped_register_proposal",
    "assistance_local_preparation",
    "set_assistance_local_preparation",
    "preview_assistance_local_template",
    "assistance_reminder",
    "set_assistance_reminder",
    "assistance_feedback",
    "set_assistance_feedback",
    "assistance_category_preferences",
    "set_assistance_category_preference",
    "retry_assistance_notification_withdrawal",
    "assistance_notification_runtime_status",
    "assistance_work_status",
    "assistance_item_work",
    "retry_assistance_work",
    "assistance_items",
    "assistance_item",
    "create_assistance_goal",
    "edit_assistance_goal",
    "link_assistance_observation",
    "refresh_assistance_observation",
    "bind_assistance_goal_source",
    "act_assistance_item",
    "prepare_assistance_context",
    "evidence_sources",
    "read_evidence_event",
    "prepare_insight_context",
    "local_draft_with_insight",
    "prepare_draft_with_insight",
    "local_insights",
    "ignore_local_insight",
    "recompute_local_insight",
    "import_recovery_state",
    "prepare_memory_context",
    "prepare_task_context",
    "draft_reply_with_memory",
    "prepare_draft_with_memory",
    "list_import_namespaces",
    "recent_import_operations",
    "import_operation",
    "discard_import",
    "prepare_import",
    "confirm_import",
    "config_snapshot",
    "session_status",
    "complete_wizard",
    "cloud_toggle",
    "files_view",
    "authorize_directory",
    "preview_plan",
    "people_graph",
    "person_summary",
    "correct_tie",
    "release_tie",
    "draft_reply",
    "draft_notices",
    "prepare_draft",
    "generate_draft",
    "discard_draft",
    "set_user_endpoint",
    "clear_user_endpoint",
    "preview_soul_import_v1",
    "preview_telegram",
    "commit_soul_import_v1",
    "commit_telegram",
    "questionnaire",
    "answer_questionnaire",
    "profile_screen",
    "correct_axis",
    "set_voice",
    "memory_list",
    "memory_detail",
    "create_memory",
    "update_memory",
    "preview_forget",
    "forget_memory",
    "retry_forget_cleanup",
    "research_preview",
    "prepare_research_save",
    "confirm_research_save",
    "cancel_research_save",
    "audit_chain",
    "collect_status",
    "grant_collect_consent",
    "revoke_collect_consent",
];

#[cfg(test)]
mod shared_state_tests {
    use super::*;

    #[test]
    fn worker_clone_uses_the_same_session_and_store() {
        let directory =
            std::env::temp_dir().join(format!("soul-state-clone-{}", std::process::id()));
        let state = SessionState::new(Session::open(&directory));
        let worker = state.clone();
        assert!(Arc::ptr_eq(&state.shared(), &worker.shared()));
        state
            .held()
            .set_user_endpoint("http://127.0.0.1:9")
            .unwrap();
        assert!(worker.held().snapshot().llm_endpoint_configured);
        worker.held().clear_user_endpoint();
        assert!(!state.held().snapshot().llm_endpoint_configured);
        let first = state.held().store().expect("synthetic store opens");
        let second = worker.held().store().expect("same store opens");
        assert!(Arc::ptr_eq(&first, &second));
        drop((first, second, worker, state));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
