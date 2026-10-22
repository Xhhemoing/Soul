//! A separate, one-use approval for a bounded local research file.
use super::*;
use soul_local_files::Destination;
use soul_store::research_retention::{HeldResearchDataset, ResearchDataset, ResearchScope};
use soul_store_api::types::StoreError;

const NOTICE: &str = "只保存本人当前获准且未遗忘的前台事件按 UTC 小时计数，缺失小时不补零。ZIP 内是明文独立副本；原库遗忘不会删除本文件；不能用于恢复 Soul。";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchSavePrepare {
    pub scope: ResearchScope,
    pub destination: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchSavePreview {
    pub request_id: String,
    pub destination: String,
    pub prepared_at_utc: String,
    pub dataset: ResearchDataset,
    pub notice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchSaveReceipt {
    pub request_id: String,
    pub destination: String,
    pub row_count: usize,
    pub included_events: u64,
    pub written_to_disk: bool,
    pub transaction_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchSaveCancel {
    pub cancelled: bool,
    pub already_published: bool,
}

#[derive(Debug)]
pub(super) struct HeldResearchSave {
    id: String,
    destination: Destination,
    destination_name: String,
    prepared_at: String,
    dataset: HeldResearchDataset,
}
#[derive(Debug)]
pub(super) struct ActiveResearchSave {
    id: String,
    control: Arc<SaveControl>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Writing,
    Cancelled,
    Published,
    Failed,
}
#[derive(Debug)]
struct SaveControl {
    phase: Mutex<Phase>,
    cancelled: AtomicBool,
}
impl SaveControl {
    fn phase(&self) -> MutexGuard<'_, Phase> {
        self.phase.lock().unwrap_or_else(|error| error.into_inner())
    }
    fn cancel(&self) -> ResearchSaveCancel {
        let mut phase = self.phase();
        let cancelled = matches!(*phase, Phase::Writing | Phase::Cancelled);
        if cancelled {
            self.cancelled.store(true, Ordering::Release);
            *phase = Phase::Cancelled;
        }
        ResearchSaveCancel {
            cancelled,
            already_published: *phase == Phase::Published,
        }
    }
}
impl Drop for ActiveResearchSave {
    fn drop(&mut self) {
        self.control.cancel();
    }
}
#[derive(Debug)]
pub struct SessionResearchSaveWork {
    held: HeldResearchSave,
    control: Arc<SaveControl>,
    store: Arc<Mutex<SqlCipherStore>>,
}
#[derive(Debug)]
pub struct SessionResearchSaveCompletion {
    result: Result<ResearchSaveReceipt, SessionRefusal>,
}

impl Session {
    pub fn prepare_research_save(
        &mut self,
        request: ResearchSavePrepare,
    ) -> Result<ResearchSavePreview, SessionRefusal> {
        // Even a failed replacement destroys the old unconsumed approval.
        self.held_research = None;
        if self
            .active_research
            .as_ref()
            .is_some_and(|active| *active.control.phase() == Phase::Writing)
        {
            return Err(refused("上一份文件仍在处理，请先取消或等待结果。"));
        }
        self.active_research = None;
        let shared = self.opened_store()?;
        let path = Path::new(&request.destination);
        if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("zip"))
            != Some(true)
        {
            return Err(refused("请选择扩展名为 .zip 的新文件。"));
        }
        let destination = Destination::prepare(path).map_err(|_| {
            refused(
                "目的地不可用。请选择普通本机磁盘上已存在文件夹中的新 ZIP 文件；不覆盖已有文件。",
            )
        })?;
        let dataset = hold(&shared).prepare_research_retention(&request.scope)
            .map_err(|_| refused("无法准备该范围：请使用有效 UTC 整日范围；数据过多或来源异常时需要缩小范围或检查来源。"))?;
        let prepared_at = rfc3339_utc(now_unix_seconds());
        // Check the complete bound before asking for approval. No file is created.
        crate::commands::research_archive::encode(dataset.dataset(), &prepared_at).map_err(
            |_| refused("完整文件超过 1 MiB 或无法编码，请缩小范围后重新准备；不会截断保存。"),
        )?;
        let id = Uuid::now_v7().to_string();
        let preview = ResearchSavePreview {
            request_id: id.clone(),
            destination: request.destination.clone(),
            prepared_at_utc: prepared_at.clone(),
            dataset: dataset.dataset().clone(),
            notice: NOTICE.into(),
        };
        self.held_research = Some(HeldResearchSave {
            id,
            destination,
            destination_name: request.destination,
            prepared_at,
            dataset,
        });
        Ok(preview)
    }
    pub fn begin_research_save(
        &mut self,
        id: &str,
    ) -> Result<SessionResearchSaveWork, SessionRefusal> {
        if self.held_research.as_ref().is_none_or(|held| held.id != id) {
            self.append_audit(&[AuditContent::denied(
                AuditAction::HitlDeny,
                ReasonCode::TokenUnknown,
            )])?;
            return Err(refused("这份批准已失效，请重新准备并确认。"));
        }
        let held = self.held_research.take().expect("matched held request");
        let store = self.opened_store()?;
        // This audits the one-use consent, never a claim that publication succeeded.
        self.append_audit(&[AuditContent::allowed(
            AuditAction::ConsentGrant,
            ReasonCode::ConsentGranted,
        )])?;
        let control = Arc::new(SaveControl {
            phase: Mutex::new(Phase::Writing),
            cancelled: AtomicBool::new(false),
        });
        self.active_research = Some(ActiveResearchSave {
            id: held.id.clone(),
            control: Arc::clone(&control),
        });
        Ok(SessionResearchSaveWork {
            held,
            control,
            store,
        })
    }
    pub fn complete_research_save(
        &mut self,
        completion: SessionResearchSaveCompletion,
    ) -> Result<ResearchSaveReceipt, SessionRefusal> {
        // Keep the terminal phase until the next prepare, so a late cancel can
        // truthfully report an already published file. Never discard a receipt.
        completion.result
    }
    pub fn cancel_research_save(&mut self, id: &str) -> Result<ResearchSaveCancel, SessionRefusal> {
        let outcome = if self
            .held_research
            .as_ref()
            .is_some_and(|held| held.id == id)
        {
            self.held_research = None;
            ResearchSaveCancel {
                cancelled: true,
                already_published: false,
            }
        } else if let Some(active) = self
            .active_research
            .as_ref()
            .filter(|active| active.id == id)
        {
            active.control.cancel()
        } else {
            ResearchSaveCancel {
                cancelled: false,
                already_published: false,
            }
        };
        if outcome.cancelled {
            // Set the cancellation first; never wait for the worker's store lock
            // before making cancellation visible to that worker.
            self.append_audit(&[AuditContent::denied(
                AuditAction::HitlDeny,
                ReasonCode::ConsentRevoked,
            )])?;
        }
        Ok(outcome)
    }
}
impl SessionResearchSaveWork {
    pub fn execute(self) -> SessionResearchSaveCompletion {
        self.execute_before_publish(|| {})
    }

    fn execute_before_publish(
        self,
        before_publish: impl FnOnce(),
    ) -> SessionResearchSaveCompletion {
        let mut result = self.publish(before_publish);
        if result.is_err() {
            {
                let mut phase = self.control.phase();
                if *phase == Phase::Writing {
                    *phase = Phase::Failed;
                }
            }
            // This is a denied terminal result, separate from the earlier
            // consent. No source identifier, observed hour or path is audited.
            let entry = AuditContent::denied(AuditAction::HitlDeny, ReasonCode::Routine);
            if append_or_store_error(&mut *hold(&self.store), entry, now_unix_seconds()).is_err() {
                result = Err(refused(
                    "文件未发布；拒绝审计也未能写入，请检查本机存储状态。",
                ));
            }
        }
        SessionResearchSaveCompletion { result }
    }

    fn publish(
        &self,
        before_publish: impl FnOnce(),
    ) -> Result<ResearchSaveReceipt, SessionRefusal> {
        if self.control.cancelled.load(Ordering::Acquire) {
            return Err(refused("保存已取消。"));
        }
        let bytes = crate::commands::research_archive::encode(
            self.held.dataset.dataset(),
            &self.held.prepared_at,
        )
        .map_err(|_| refused("无法生成完整留存文件，请缩小范围后重新准备。"))?;
        let mut store = hold(&self.store);
        let publication = store.publish_research_retention(&self.held.dataset, || {
            let pending = self.held.destination.write(&bytes, &self.control.cancelled)
                .map_err(|_| StoreError::ContractViolation("RESEARCH_WRITE_REFUSED".into()))?;
            before_publish();
            // Do not hold this mutex while writing. Cancel and publication compete
            // only at this final point, independently of the Session mutex.
            let mut phase = self.control.phase();
            if *phase != Phase::Writing { return Err(StoreError::ContractViolation("RESEARCH_CANCELLED".into())); }
            pending.publish().map_err(|_| StoreError::ContractViolation("RESEARCH_PUBLISH_REFUSED".into()))?;
            *phase = Phase::Published;
            Ok(())
        }).map_err(|_| refused("文件未发布：来源或授权已变更、请求已取消，或目的地写入失败。请重新准备。极少数创建失败可能留下空文件。"))?;
        Ok(ResearchSaveReceipt {
            request_id: self.held.id.clone(),
            destination: self.held.destination_name.clone(),
            row_count: self.held.dataset.dataset().rows.len(),
            included_events: self.held.dataset.dataset().counts.included_events,
            written_to_disk: true,
            transaction_closed: publication.transaction_closed,
        })
    }
}
fn refused(message: &str) -> SessionRefusal {
    SessionRefusal {
        reason_code: ReasonCode::Routine.as_str().into(),
        explanation: message.into(),
    }
}

#[cfg(all(test, windows))]
#[path = "session_research_retention_tests.rs"]
mod tests;
