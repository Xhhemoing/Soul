//! Explicit local opening-template preview. Never a model grant or Draft input.
use super::*;
pub use soul_store_api::preparation::LocalTemplateKind;
use soul_store_api::preparation::{LocalTemplateMetadata, LocalTemplateVoice};
use soul_store_api::ProfileStore;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceLocalPreparationState {
    Off,
    None,
    Pending,
    Running,
    Ready,
    SourceUnavailable,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssistanceLocalPreparation {
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub preference_revision: Option<Uuid>,
    pub enabled: bool,
    pub state: AssistanceLocalPreparationState,
    pub template_id: Option<Uuid>,
    pub failure_code: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAssistanceLocalPreparation {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub expected_preference_revision: Option<Uuid>,
    pub enabled: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewAssistanceLocalTemplate {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub expected_preference_revision: Uuid,
    pub template_id: Uuid,
}
#[derive(Clone, Serialize)]
pub struct AssistanceLocalTemplatePreview {
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub preference_revision: Uuid,
    pub template_id: Uuid,
    pub kind: LocalTemplateKind,
    pub prepared_at: i64,
    pub body: String,
}
impl std::fmt::Debug for AssistanceLocalTemplatePreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssistanceLocalTemplatePreview")
            .field("template_id", &self.template_id)
            .finish_non_exhaustive()
    }
}
pub struct SessionLocalTemplateWork {
    id: Uuid,
    request: PreviewAssistanceLocalTemplate,
    store: Arc<Mutex<SqlCipherStore>>,
    lease: DraftLease,
}
pub struct SessionLocalTemplateCompletion {
    id: Uuid,
    result: Result<(LocalTemplateMetadata, String), SessionRefusal>,
    _lease: DraftLease,
}
impl std::fmt::Debug for SessionLocalTemplateWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLocalTemplateWork")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for SessionLocalTemplateCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLocalTemplateCompletion")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
fn changed() -> SessionRefusal {
    SessionRefusal {
        reason_code: "ASSISTANCE_LOCAL_PREPARATION_CHANGED".into(),
        explanation: "本地模板的事项、来源、偏好或语气已变化；已排入重新准备，请重新核对。".into(),
    }
}
fn profile_unavailable() -> SessionRefusal {
    SessionRefusal {
        reason_code: "ASSISTANCE_LOCAL_PROFILE_UNAVAILABLE".into(),
        explanation: "本机档案的四项语气尚不可读；请核对档案后再准备，不使用默认语气替代。".into(),
    }
}
pub(super) fn voice(store: &SqlCipherStore) -> Result<LocalTemplateVoice, SessionRefusal> {
    // No blank-profile or neutral-voice fallback in automatic preparation.
    let profile = store
        .get_profile(OWNER_PROFILE_ID)
        .map_err(|_| profile_unavailable())?;
    soul_store::preparation::local_template_voice(&profile.voice).map_err(|_| profile_unavailable())
}
fn render(voice: &LocalTemplateVoice) -> Result<String, SessionRefusal> {
    let voice:soul_profile::voice::VoiceProfile=serde_json::from_value(serde_json::json!({"register":voice.register,"directness":voice.directness,"warmth":voice.warmth,"emoji_use":voice.emoji_use})).map_err(|_|profile_unavailable())?;
    soul_draft::render_template(
        &soul_draft::ProfileBrief::new(voice),
        soul_draft::TemplateContext::opening(),
    )
    .map_err(|_| SessionRefusal {
        reason_code: "ASSISTANCE_LOCAL_TEMPLATE_FAILED".into(),
        explanation: "本地模板暂时无法生成，请重新核对后重试。".into(),
    })
}
pub(super) fn prepare_for_claim(
    store: &SqlCipherStore,
    claim: &soul_store_api::assistance::AssistanceWorkClaim,
    item: &AssistanceItemRow,
) -> Result<(), SessionRefusal> {
    if claim.preparation_revision.is_none() {
        return Ok(());
    }
    let pref = store.local_preparation_preference(item.item_id)?;
    if !pref.enabled || pref.revision != claim.preparation_revision {
        return Ok(());
    }
    match voice(store).and_then(|voice| {
        drop(render(&voice)?);
        Ok(voice)
    }) {
        Ok(voice) => {
            store.finish_local_template(
                claim,
                item,
                Some(&voice),
                OWNER_PROFILE_ID,
                now_unix_seconds(),
                None,
            )?;
        }
        Err(error) => {
            store.finish_local_template(
                claim,
                item,
                None,
                OWNER_PROFILE_ID,
                now_unix_seconds(),
                Some(&error.reason_code),
            )?;
        }
    }
    Ok(())
}
fn snapshot_matches(
    store: &SqlCipherStore,
    template: &LocalTemplateMetadata,
) -> Result<bool, SessionRefusal> {
    let item = store.assistance_item(template.item_id)?;
    let pref = store.local_preparation_preference(item.item_id)?;
    Ok(item.is_current()
        && item.revision == template.item_revision
        && item.source == template.source
        && pref.enabled
        && pref.revision == Some(template.preference_revision)
        && pref.owner_profile_id == Some(template.owner_profile_id)
        && template.owner_profile_id == OWNER_PROFILE_ID
        && voice(store)? == template.voice
        && store.local_template(item.item_id)?.as_ref() == Some(template))
}
impl SessionLocalTemplateWork {
    pub fn execute(self) -> SessionLocalTemplateCompletion {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut store = hold(&self.store);
            // None is a committed mismatch, not an Err that rolls enqueue back.
            let result = store.transact(
                |store| -> Result<Option<(LocalTemplateMetadata, String)>, SessionRefusal> {
                    let item = store.assistance_item(self.request.item_id)?;
                    let template = store.local_template(item.item_id)?;
                    let current = template.as_ref().is_some_and(|t| {
                        t.template_id == self.request.template_id
                            && t.item_revision == self.request.expected_revision
                            && t.preference_revision == self.request.expected_preference_revision
                    });
                    let matches = if current {
                        snapshot_matches(store, template.as_ref().expect("present"))?
                    } else {
                        false
                    };
                    if !matches {
                        store.refresh_local_template(item.item_id)?;
                        return Ok(None);
                    }
                    match assistance::validate(store, &item) {
                        Ok(_) => {}
                        Err(error)
                            if matches!(
                                error.reason_code.as_str(),
                                "ASSISTANCE_CHANGED" | "LOCAL_INSIGHT_UNAVAILABLE"
                            ) =>
                        {
                            store.refresh_local_template(item.item_id)?;
                            return Ok(None);
                        }
                        Err(error) => return Err(error),
                    }
                    let template = template.expect("checked");
                    let body = render(&template.voice)?;
                    Ok(Some((template, body)))
                },
            )?;
            result.ok_or_else(changed)
        }))
        .unwrap_or_else(|_| {
            Err(SessionRefusal {
                reason_code: "ASSISTANCE_LOCAL_TEMPLATE_FAILED".into(),
                explanation: "本地预览未完成；请重新核对。".into(),
            })
        });
        SessionLocalTemplateCompletion {
            id: self.id,
            result,
            _lease: self.lease,
        }
    }
}
impl Session {
    pub fn assistance_local_preparation(
        &self,
        item_id: &str,
    ) -> Result<AssistanceLocalPreparation, SessionRefusal> {
        let id = parse_id(item_id, "事项")?;
        let shared = self.opened_store()?;
        let store = hold(&shared);
        let item = store.assistance_item(id)?;
        if !matches!(item.source, AssistanceSource::ManualGoal { .. }) {
            return Err(SessionRefusal {
                reason_code: "ASSISTANCE_LOCAL_PREPARATION_UNAVAILABLE".into(),
                explanation: "此项仅适用于明确手写目标。".into(),
            });
        }
        let pref = store.local_preparation_preference(id)?;
        let work = store.assistance_item_work(id)?;
        let template = store.local_template(id)?;
        let template_id = template
            .filter(|t| {
                t.item_revision == item.revision
                    && pref.enabled
                    && pref.revision == Some(t.preference_revision)
                    && item.is_current()
            })
            .map(|t| t.template_id);
        let state = if !pref.enabled {
            AssistanceLocalPreparationState::Off
        } else if self
            .active_assistance
            .as_ref()
            .is_some_and(|active| active.claim.item_id == id && active.unknown)
        {
            AssistanceLocalPreparationState::Unknown
        } else {
            match work.state {
                AssistanceWorkState::Pending => AssistanceLocalPreparationState::Pending,
                AssistanceWorkState::Running => AssistanceLocalPreparationState::Running,
                AssistanceWorkState::SourceUnavailable => {
                    AssistanceLocalPreparationState::SourceUnavailable
                }
                AssistanceWorkState::Failed => AssistanceLocalPreparationState::Failed,
                AssistanceWorkState::Unknown => AssistanceLocalPreparationState::Unknown,
                _ if pref.failure_code.is_some() => AssistanceLocalPreparationState::Failed,
                _ if template_id.is_some() => AssistanceLocalPreparationState::Ready,
                _ => AssistanceLocalPreparationState::None,
            }
        };
        Ok(AssistanceLocalPreparation {
            item_id: id,
            item_revision: item.revision,
            preference_revision: pref.revision,
            enabled: pref.enabled,
            state,
            template_id,
            failure_code: pref.failure_code.or(work.failure_code),
        })
    }
    pub fn set_assistance_local_preparation(
        &mut self,
        request: SetAssistanceLocalPreparation,
    ) -> Result<AssistanceLocalPreparation, SessionRefusal> {
        let shared = self.opened_store()?;
        hold(&shared).set_local_preparation_preference(
            request.item_id,
            request.expected_revision,
            request.expected_preference_revision,
            request.enabled,
            OWNER_PROFILE_ID,
        )?;
        self.assistance_local_preparation(&request.item_id.to_string())
    }
    pub fn begin_assistance_local_template_preview(
        &mut self,
        request: PreviewAssistanceLocalTemplate,
    ) -> Result<SessionLocalTemplateWork, SessionRefusal> {
        self.require_draft_slot()?;
        let shared = self.opened_store()?;
        let id = Uuid::now_v7();
        let finished = Arc::new(AtomicBool::new(false));
        self.active_draft = Some(ActiveDraft {
            id,
            kind: E1WorkerKind::LocalPreview,
            finished: finished.clone(),
        });
        Ok(SessionLocalTemplateWork {
            id,
            request,
            store: shared,
            lease: DraftLease(finished),
        })
    }
    pub fn complete_assistance_local_template_preview(
        &mut self,
        completed: SessionLocalTemplateCompletion,
    ) -> Result<AssistanceLocalTemplatePreview, SessionRefusal> {
        if !self
            .active_draft
            .as_ref()
            .is_some_and(|a| a.id == completed.id && a.kind == E1WorkerKind::LocalPreview)
        {
            return Err(changed());
        }
        self.active_draft = None;
        let _ = self.flush_deferred_revoke();
        let (template, body) = completed.result?;
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let current = store.transact(|s| -> Result<bool, SessionRefusal> {
            if snapshot_matches(s, &template)? {
                Ok(true)
            } else {
                s.refresh_local_template(template.item_id)?;
                Ok(false)
            }
        })?;
        if !current {
            return Err(changed());
        }
        Ok(AssistanceLocalTemplatePreview {
            item_id: template.item_id,
            item_revision: template.item_revision,
            preference_revision: template.preference_revision,
            template_id: template.template_id,
            kind: template.kind,
            prepared_at: template.prepared_at,
            body,
        })
    }
}
