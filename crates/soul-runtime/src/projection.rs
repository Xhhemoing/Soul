//! HTTP projections reuse the exact legacy facade once; errors never carry core prose.
use soul_store_api::import::{ImportAuditState, ImportCounts, ImportOperationState};
use soul_types::*;
use soulcore::commands::memory::{ForgetConfirmation, MemoryChange};
use soulcore::commands::session::{
    ImportOperationView, PreferenceOccasion, ScopedRegisterReceipt, ScopedRegisterStatusState,
    SessionRefusal, VoiceRegister,
};
use uuid::Uuid;

use super::{Runtime, RuntimeContext};

fn error(error: SessionRefusal) -> WriteError {
    let code = match error.reason_code.as_str() {
        "RUNTIME_CALLER_MISMATCH" => WriteErrorCode::CallerMismatch,
        "RUNTIME_CONFIRMATION_UNAVAILABLE" | "IMPORT_SOURCE_CHANGED" => {
            WriteErrorCode::ConfirmationUnavailable
        }
        "IMPORT_BUSY" | "ASSISTANCE_BUSY" | "REMINDER_BUSY" | "IMPORT_PREPARATION_HELD" => {
            WriteErrorCode::StoreBusy
        }
        "SYNTHETIC_MODE_RESTRICTED" => WriteErrorCode::InvalidRequest,
        _ => WriteErrorCode::OperationFailed,
    };
    let message = match code {
        WriteErrorCode::CallerMismatch => "This preparation belongs to another caller.",
        WriteErrorCode::ConfirmationUnavailable => {
            "This confirmation is unavailable; prepare again."
        }
        WriteErrorCode::StoreBusy => "The Session is busy; retry after settlement.",
        WriteErrorCode::InvalidRequest => "Invalid write request.",
        WriteErrorCode::OperationFailed => {
            "The operation could not be completed; read back its current state."
        }
    };
    WriteError {
        code,
        message: message.into(),
        details: None,
    }
}

fn invalid() -> WriteError {
    WriteError {
        code: WriteErrorCode::InvalidRequest,
        message: "Invalid write request.".into(),
        details: None,
    }
}

fn occasion(value: RuntimeOccasion) -> PreferenceOccasion {
    match value {
        RuntimeOccasion::Personal => PreferenceOccasion::Personal,
        RuntimeOccasion::Work => PreferenceOccasion::Work,
        RuntimeOccasion::Study => PreferenceOccasion::Study,
    }
}
fn view_occasion(value: PreferenceOccasion) -> RuntimeOccasion {
    match value {
        PreferenceOccasion::Personal => RuntimeOccasion::Personal,
        PreferenceOccasion::Work => RuntimeOccasion::Work,
        PreferenceOccasion::Study => RuntimeOccasion::Study,
    }
}
fn register(value: RuntimeRegister) -> VoiceRegister {
    match value {
        RuntimeRegister::Casual => VoiceRegister::Casual,
        RuntimeRegister::Plain => VoiceRegister::Plain,
        RuntimeRegister::Formal => VoiceRegister::Formal,
    }
}
fn view_register(value: VoiceRegister) -> RuntimeRegister {
    match value {
        VoiceRegister::Casual => RuntimeRegister::Casual,
        VoiceRegister::Plain => RuntimeRegister::Plain,
        VoiceRegister::Formal => RuntimeRegister::Formal,
    }
}
fn receipt(value: ScopedRegisterReceipt) -> RuntimeScopedReceipt {
    RuntimeScopedReceipt {
        preference_id: value.preference_id.to_string(),
        revision: value.revision.to_string(),
        memory_id: value.memory_id.to_string(),
        register: view_register(value.register),
    }
}
fn counts(c: ImportCounts) -> RuntimeImportCounts {
    RuntimeImportCounts {
        participants: c.participants,
        messages: c.messages,
        duplicates: c.duplicates,
        contacts_created: c.contacts_created,
        contacts_matched: c.contacts_matched,
        events_written: c.events_written,
        evidence_written: c.evidence_written,
        messages_with_injection_markers: c.messages_with_injection_markers,
        ties_rebuilt: c.ties_rebuilt,
    }
}
fn import_view(value: ImportOperationView) -> RuntimeImportOperation {
    let op = value.operation;
    RuntimeImportOperation {
        operation_id: op.operation_id.to_string(),
        preview_id: op.preview_id.to_string(),
        namespace_id: op.namespace_id.to_string(),
        state: match op.state {
            ImportOperationState::Reserved => RuntimeImportState::Reserved,
            ImportOperationState::Running => RuntimeImportState::Running,
            ImportOperationState::Committed => RuntimeImportState::Committed,
            ImportOperationState::Rejected => RuntimeImportState::Rejected,
            ImportOperationState::Discarded => RuntimeImportState::Discarded,
            ImportOperationState::Invalidated => RuntimeImportState::Invalidated,
            ImportOperationState::Interrupted => RuntimeImportState::Interrupted,
        },
        preview: counts(op.preview),
        receipt: op.receipt.map(counts),
        audit_state: match op.audit_state {
            ImportAuditState::NotApplicable => RuntimeAuditState::NotApplicable,
            ImportAuditState::Recorded => RuntimeAuditState::Recorded,
            ImportAuditState::Unconfirmed => RuntimeAuditState::Unconfirmed,
        },
        namespace_revoked: value.namespace_revoked,
        error_code: op.error_code.map(|code| {
            match code {
                soul_store_api::import::ImportOperationError::SourceChanged => "SOURCE_CHANGED",
                soul_store_api::import::ImportOperationError::IdentityConflict => {
                    "IDENTITY_CONFLICT"
                }
                soul_store_api::import::ImportOperationError::IdentityUnavailable => {
                    "IDENTITY_UNAVAILABLE"
                }
                soul_store_api::import::ImportOperationError::LegacyChoiceRequired => {
                    "LEGACY_CHOICE_REQUIRED"
                }
                soul_store_api::import::ImportOperationError::NamespaceRevoked => {
                    "NAMESPACE_REVOKED"
                }
                soul_store_api::import::ImportOperationError::StoreUnavailable => {
                    "STORE_UNAVAILABLE"
                }
                soul_store_api::import::ImportOperationError::WorkerFailed => "WORKER_FAILED",
                soul_store_api::import::ImportOperationError::Interrupted => "INTERRUPTED",
            }
            .into()
        }),
    }
}

impl Runtime {
    pub fn safe_prepare_synthetic_import(
        &self,
        ctx: &RuntimeContext,
        request: SyntheticImportRequest,
    ) -> Result<RuntimePreparedImport, WriteError> {
        let value = self.prepare_synthetic_import(ctx, request).map_err(error)?;
        let operation = import_view(ImportOperationView {
            operation: value.operation,
            namespace_revoked: false,
            prepared_here: true,
        });
        Ok(RuntimePreparedImport {
            operation,
            notice: value.notice,
        })
    }
    pub fn safe_confirm_import(
        &self,
        ctx: &RuntimeContext,
        operation_id: &str,
        preview_id: &str,
    ) -> Result<RuntimeImportOperation, WriteError> {
        self.confirm_import(ctx, operation_id, preview_id)
            .map(import_view)
            .map_err(error)
    }
    pub fn safe_discard_import(
        &self,
        ctx: &RuntimeContext,
        operation_id: &str,
    ) -> Result<RuntimeImportOperation, WriteError> {
        self.discard_import(ctx, operation_id)
            .map(import_view)
            .map_err(error)
    }
    pub fn safe_import_operation(
        &self,
        operation_id: &str,
    ) -> Result<RuntimeImportOperation, WriteError> {
        self.import_operation(operation_id)
            .map(import_view)
            .map_err(error)
    }
    pub fn safe_draft_scoped_reply(
        &self,
        ctx: &RuntimeContext,
        request: ScopedDraftRequest,
    ) -> Result<RuntimeScopedDraft, WriteError> {
        let value = self
            .draft_scoped_reply(
                ctx,
                &request.pasted,
                &request.person_id,
                occasion(request.occasion),
            )
            .map_err(error)?;
        Ok(RuntimeScopedDraft {
            result_id: value.result_id.to_string(),
            person_id: value.person_id.to_string(),
            occasion: view_occasion(value.occasion),
            applied_preference: value.applied_preference.map(receipt),
            draft: RuntimeLocalDraft {
                text: value.draft.text,
                not_sent_notice: value.draft.not_sent_notice,
                source_notice: value.draft.source_notice,
                third_party_turns: value.draft.third_party_turns,
                placeheld_turns: value.draft.placeheld_turns,
                carries_exempted_original: value.draft.carries_exempted_original,
            },
        })
    }
    pub fn safe_propose_scoped_register(
        &self,
        ctx: &RuntimeContext,
        request: ScopedProposalRequest,
    ) -> Result<RuntimeScopedProposal, WriteError> {
        let value = self
            .propose_scoped_register(
                ctx,
                &request.result_id,
                request.edit_revision,
                register(request.register),
            )
            .map_err(error)?;
        Ok(RuntimeScopedProposal {
            proposal_id: value.proposal_id.to_string(),
            result_id: value.result_id.to_string(),
            edit_revision: value.edit_revision,
            person_id: value.person_id.to_string(),
            occasion: view_occasion(value.occasion),
            register: view_register(value.register),
            expected_revision: value.expected_revision.map(|id| id.to_string()),
            notice: value.notice,
        })
    }
    pub fn safe_confirm_scoped_register(
        &self,
        ctx: &RuntimeContext,
        request: ScopedConfirmRequest,
    ) -> Result<RuntimeScopedReceipt, WriteError> {
        let expected = request
            .expected_revision
            .map(|id| Uuid::parse_str(&id).map_err(|_| invalid()))
            .transpose()?;
        self.confirm_scoped_register(ctx, &request.proposal_id, expected)
            .map(receipt)
            .map_err(error)
    }
    pub fn safe_discard_scoped_register_proposal(
        &self,
        ctx: &RuntimeContext,
        proposal_id: &str,
    ) -> Result<bool, WriteError> {
        self.discard_scoped_register_proposal(ctx, proposal_id)
            .map_err(error)
    }
    pub fn safe_scoped_register_status(
        &self,
        person_id: &str,
        scope: RuntimeOccasion,
    ) -> Result<RuntimeScopedStatus, WriteError> {
        let value = self
            .scoped_register_status(person_id, occasion(scope))
            .map_err(error)?;
        Ok(RuntimeScopedStatus {
            person_id: value.person_id.to_string(),
            occasion: view_occasion(value.occasion),
            state: match value.state {
                ScopedRegisterStatusState::Absent => RuntimePreferenceState::Absent,
                ScopedRegisterStatusState::Active => RuntimePreferenceState::Active,
                ScopedRegisterStatusState::Inactive => RuntimePreferenceState::Inactive,
                ScopedRegisterStatusState::Forgotten => RuntimePreferenceState::Forgotten,
                ScopedRegisterStatusState::Unavailable => RuntimePreferenceState::Unavailable,
            },
            preference: value.preference.map(receipt),
            memory_id: value.memory_id.map(|id| id.to_string()),
            notice: value.notice,
        })
    }
    pub fn safe_edit_memory(
        &self,
        ctx: &RuntimeContext,
        memory_id: &str,
        request: MemoryEditRequest,
    ) -> Result<MemoryDetailView, WriteError> {
        let value = self
            .edit_memory(
                ctx,
                memory_id,
                &MemoryChange {
                    title: request.title,
                    summary: request.summary,
                    memory_type: request.memory_type,
                },
            )
            .map_err(error)?;
        Ok(MemoryDetailView {
            memory_id: value.memory_id,
            memory_type: value.memory_type,
            title: value.title,
            summary: value.summary,
            third_party_content_present: value.third_party_content_present,
        })
    }
    pub fn safe_preview_forget(
        &self,
        ctx: &RuntimeContext,
        memory_id: &str,
    ) -> Result<RuntimeForgetPreview, WriteError> {
        let value = self.preview_forget(ctx, memory_id).map_err(error)?;
        Ok(RuntimeForgetPreview {
            preview_id: value.preview_id,
            memory_id: value.memory_id,
            content_key_count: value.content_key_count,
            memories_affected: value.memories_affected,
            contacts_affected: value.contacts_affected,
            sealed_blobs_destroyed: value.sealed_blobs_destroyed,
            inferences_orphaned: value.inferences_orphaned,
            import_namespaces_revoked: value.import_namespaces_revoked,
            local_insights_invalidated: value.local_insights_invalidated,
            assistance_items_invalidated: value.assistance_items_invalidated,
            audit_entries_retained: value.audit_entries_retained,
            destroys_anything: value.destroys_anything,
            notice: value.notice,
        })
    }
    pub fn safe_forget_memory(
        &self,
        ctx: &RuntimeContext,
        request: RuntimeForgetConfirmation,
    ) -> Result<RuntimeForgetReceipt, WriteError> {
        let value = self
            .forget_memory(
                ctx,
                &ForgetConfirmation {
                    preview_id: request.preview_id,
                    memory_id: request.memory_id,
                },
            )
            .map_err(error)?;
        Ok(RuntimeForgetReceipt {
            memory_id: value.memory_id,
            content_keys_destroyed: value.content_keys_destroyed,
            sealed_blobs_destroyed: value.sealed_blobs_destroyed,
            inferences_orphaned: value.inferences_orphaned,
            import_namespaces_revoked: value.import_namespaces_revoked,
            local_insights_invalidated: value.local_insights_invalidated,
            assistance_items_invalidated: value.assistance_items_invalidated,
            matched_preview: value.matched_preview,
            logical_committed: value.logical_committed,
            cleanup: match value.cleanup {
                soul_store_api::forget::ForgetCleanup::Complete => RuntimeForgetCleanup::Complete,
                soul_store_api::forget::ForgetCleanup::Pending { checkpoint } => {
                    RuntimeForgetCleanup::Pending {
                        checkpoint: checkpoint.map(|c| RuntimeWalCheckpoint {
                            busy: c.busy,
                            log_frames: c.log_frames,
                            checkpointed_frames: c.checkpointed_frames,
                        }),
                    }
                }
            },
            audit: match value.audit {
                soul_memory::ForgetAudit::Recorded => RuntimeAuditState::Recorded,
                soul_memory::ForgetAudit::Unconfirmed => RuntimeAuditState::Unconfirmed,
            },
        })
    }
    /// An exact metadata/tombstone read; unreadable prose is never proof of forget.
    pub fn safe_memory_state(&self, memory_id: &str) -> Result<RuntimeMemoryState, WriteError> {
        let id = Uuid::parse_str(memory_id)
            .map_err(|_| invalid())?
            .to_string();
        let list = self.held_session().memories().map_err(error)?;
        let row = list
            .memories
            .into_iter()
            .find(|row| row.memory_id == id)
            .ok_or_else(|| WriteError {
                code: WriteErrorCode::OperationFailed,
                message: "The memory state could not be read.".into(),
                details: None,
            })?;
        Ok(RuntimeMemoryState {
            memory_id: row.memory_id,
            forget_state: row.forget_state,
            third_party_content_present: row.third_party_content_present,
        })
    }
}
