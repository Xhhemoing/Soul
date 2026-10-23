//! Explicit owner register for an exact person/occasion, ordinary local replies only.
use super::*;
use soul_profile::voice::VoiceSetting;
use soul_schema::contact::ContactClass;
use soul_schema::memory::ForgetState;
use soul_store_api::scoped_preference::{
    ExactPreferenceScope, ExpectedPreference, ScopedPreferenceState,
};
use soul_store_api::types::StoreError;
use soul_store_api::{BlobStore, GraphStore, MemoryStore, ProfileStore};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedRegisterRecipient {
    pub person_id: Uuid,
    pub identifier_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedRegisterReceipt {
    pub preference_id: Uuid,
    pub revision: Uuid,
    pub memory_id: Uuid,
    pub register: VoiceRegister,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedRegisterStatusState {
    Absent,
    Active,
    Inactive,
    Forgotten,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedRegisterStatus {
    pub person_id: Uuid,
    pub occasion: PreferenceOccasion,
    pub state: ScopedRegisterStatusState,
    pub preference: Option<ScopedRegisterReceipt>,
    pub memory_id: Option<Uuid>,
    pub notice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedDraftView {
    pub result_id: Uuid,
    pub person_id: Uuid,
    pub occasion: PreferenceOccasion,
    pub draft: DraftValue,
    pub applied_preference: Option<ScopedRegisterReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopedRegisterProposal {
    pub proposal_id: Uuid,
    pub result_id: Uuid,
    pub edit_revision: u64,
    pub person_id: Uuid,
    pub occasion: PreferenceOccasion,
    pub register: VoiceRegister,
    pub expected_revision: Option<Uuid>,
    pub notice: String,
}

#[derive(Debug)]
pub(super) struct HeldScopedResult {
    id: Uuid,
    generation: Uuid,
    scope: ExactPreferenceScope,
    expected: ExpectedPreference,
}

#[derive(Debug)]
pub(super) struct HeldScopedProposal {
    view: ScopedRegisterProposal,
    scope: ExactPreferenceScope,
    expected: ExpectedPreference,
    generation: Uuid,
}

fn changed() -> SessionRefusal {
    SessionRefusal {
        reason_code: "ROUTINE".to_owned(),
        explanation: "这次范围、草稿或偏好已变化，请重新生成并确认。".to_owned(),
    }
}

fn validate_scope(
    store: &SqlCipherStore,
    scope: ExactPreferenceScope,
) -> Result<(), SessionRefusal> {
    store
        .get_profile(scope.owner_profile_id)
        .map_err(|_| SessionRefusal {
            reason_code: "ROUTINE".to_owned(),
            explanation: "当前档案不可用，请先在档案页设置语气，再按人物和场合起草。".to_owned(),
        })?;
    let person = store.get_contact(scope.person_contact_id)?;
    if person.contact_class != ContactClass::ThirdParty
        || person.forget_state != ForgetState::Active
    {
        return Err(changed());
    }
    Ok(())
}

fn revision(expected: ExpectedPreference) -> Option<Uuid> {
    match expected {
        ExpectedPreference::Absent => None,
        ExpectedPreference::Revision { revision, .. } => Some(revision),
    }
}

fn notice(scope: ExactPreferenceScope, register: VoiceRegister) -> String {
    let occasion = match scope.occasion {
        PreferenceOccasion::Personal => "私人",
        PreferenceOccasion::Work => "工作",
        PreferenceOccasion::Study => "学习",
    };
    let register = match register {
        VoiceRegister::Casual => "随意",
        VoiceRegister::Plain => "平实",
        VoiceRegister::Formal => "正式",
    };
    format!("保存的是我的表达偏好：我对人物 {} 在{}场合使用{}语域。确认后，Soul 会在以后我明确选择同一人物和场合的普通本地回复模板中读取并应用，持续到更改或遗忘；这是全局语域的局部例外，全局值与锁不变。普通记忆编辑会停用此偏好，须重新明确确认。不会保存草稿或编辑正文，也不应用于模型端点、手选记忆、洞察或辅助事项。", scope.person_contact_id, occasion, register)
}

fn status_read_failed() -> SessionRefusal {
    SessionRefusal {
        reason_code: "ROUTINE".to_owned(),
        explanation: "暂时无法读取此范围的偏好，请稍后重试。".to_owned(),
    }
}

fn status_notice(state: ScopedRegisterStatusState) -> String {
    match state {
        ScopedRegisterStatusState::Absent => "本范围尚未保存语域偏好；这是本次读取结果。",
        ScopedRegisterStatusState::Active => "本范围已确认的语域偏好当前可用；起草时会再次核对。",
        ScopedRegisterStatusState::Inactive => "本范围偏好已停用；须再次明确确认才会应用。",
        ScopedRegisterStatusState::Forgotten => "本范围偏好已遗忘，确认说明的内容密钥已销毁。",
        ScopedRegisterStatusState::Unavailable => {
            "本范围偏好当前不可用，请检查对应记忆或稍后重试。"
        }
    }
    .to_owned()
}

/// This is a fallible read, unlike has_content_key's conservative boolean.
/// Missing/broken sealed data is unavailable; backend failures remain errors.
fn status_memory_readable(
    store: &SqlCipherStore,
    memory: &soul_schema::memory::SoulMemory,
) -> Result<bool, SessionRefusal> {
    if memory.summary_ref.is_none() {
        return Ok(false);
    }
    for (field, sealed) in [
        ("title_ref", &memory.title_ref),
        ("summary_ref", &memory.summary_ref),
    ] {
        let Some(sealed) = sealed else { continue };
        if sealed.content_key_id != memory.content_key_id
            || sealed.subject != soul_schema::common::SealedSubject::Owner
            || sealed.aad.as_deref()
                != Some(soul_schema::common::field_aad(&memory.memory_id, field).as_str())
        {
            return Ok(false);
        }
        match store.open(sealed) {
            // The confirmation prose is deliberately discarded, never put in
            // the status DTO or reused to infer an executable setting.
            Ok(_) => {}
            Err(
                StoreError::ContentKeyDestroyed(_)
                | StoreError::BlobMissing(_)
                | StoreError::SealBroken(_),
            ) => return Ok(false),
            Err(_) => return Err(status_read_failed()),
        }
    }
    Ok(true)
}

impl Session {
    pub(super) fn runtime_scoped_pending_ids(&self) -> (Option<Uuid>, Option<Uuid>) {
        (
            self.held_scoped_result.as_ref().map(|held| held.id),
            self.held_scoped_proposal
                .as_ref()
                .map(|held| held.view.proposal_id),
        )
    }

    /// A read-time snapshot, not a new proposal or a reusable application
    /// receipt. Every path preserves held work and writes no audit or data.
    pub fn scoped_register_status(
        &self,
        person_id: &str,
        occasion: PreferenceOccasion,
    ) -> Result<ScopedRegisterStatus, SessionRefusal> {
        self.require_store_idle()?;
        let scope = ExactPreferenceScope {
            owner_profile_id: OWNER_PROFILE_ID,
            person_contact_id: parse_id(person_id, "人物")?,
            occasion,
        };
        let shared = self.opened_store().map_err(|_| status_read_failed())?;
        let store = hold(&shared);
        validate_scope(&store, scope).map_err(|_| SessionRefusal {
            reason_code: "ROUTINE".to_owned(),
            explanation: "当前档案或所选人物不可用，请检查档案并重新选择有效人物。".to_owned(),
        })?;
        let snapshot = |state, preference, memory_id| ScopedRegisterStatus {
            person_id: scope.person_contact_id,
            occasion,
            state,
            preference,
            memory_id,
            notice: status_notice(state),
        };
        let Some(row) = store
            .scoped_register_preference(scope)
            .map_err(|_| status_read_failed())?
        else {
            return Ok(snapshot(ScopedRegisterStatusState::Absent, None, None));
        };
        let unavailable = || {
            snapshot(
                ScopedRegisterStatusState::Unavailable,
                None,
                Some(row.memory_id),
            )
        };
        let memory = match store.get_memory(row.memory_id) {
            Ok(memory) => memory,
            Err(StoreError::NotFound { .. }) => return Ok(unavailable()),
            Err(_) => return Err(status_read_failed()),
        };
        let destroyed = store
            .content_key_destroyed(memory.content_key_id)
            .map_err(|_| status_read_failed())?;
        if destroyed
            && memory.forget_state == ForgetState::Forgotten
            && row.state == ScopedPreferenceState::Inactive
            && row.register.is_none()
        {
            return Ok(snapshot(
                ScopedRegisterStatusState::Forgotten,
                None,
                Some(row.memory_id),
            ));
        }
        if destroyed || memory.forget_state != ForgetState::Active {
            return Ok(unavailable());
        }
        if !status_memory_readable(&store, &memory)? {
            return Ok(unavailable());
        }
        if row.state == ScopedPreferenceState::Inactive && row.register.is_none() {
            return Ok(snapshot(
                ScopedRegisterStatusState::Inactive,
                None,
                Some(row.memory_id),
            ));
        }
        match store
            .lookup_confirmed_register(scope)
            .map_err(|_| status_read_failed())?
        {
            Some(current) => match current.register {
                Some(register) => Ok(snapshot(
                    ScopedRegisterStatusState::Active,
                    Some(ScopedRegisterReceipt {
                        preference_id: current.preference_id,
                        revision: current.revision,
                        memory_id: current.memory_id,
                        register,
                    }),
                    Some(current.memory_id),
                )),
                None => Ok(unavailable()),
            },
            None => Ok(unavailable()),
        }
    }

    pub(super) fn clear_scoped_draft(&mut self) -> bool {
        let result = self.held_scoped_result.take().is_some();
        let proposal = self.held_scoped_proposal.take().is_some();
        result || proposal
    }

    pub fn scoped_register_recipients(
        &self,
    ) -> Result<Vec<ScopedRegisterRecipient>, SessionRefusal> {
        self.require_store_idle()?;
        let shared = self.opened_store()?;
        let store = hold(&shared);
        Ok(store
            .list_contacts()?
            .into_iter()
            .filter(|person| {
                person.contact_class == ContactClass::ThirdParty
                    && person.forget_state == ForgetState::Active
            })
            .map(|person| ScopedRegisterRecipient {
                person_id: person.contact_id,
                identifier_hint: format!("人物 {}", person.contact_id),
            })
            .collect())
    }

    pub fn draft_scoped_reply(
        &mut self,
        pasted: &str,
        person_id: &str,
        occasion: PreferenceOccasion,
    ) -> Result<ScopedDraftView, SessionRefusal> {
        self.require_store_idle()?;
        self.require_draft_slot()?;
        self.invalidate_drafts();
        let scope = ExactPreferenceScope {
            owner_profile_id: OWNER_PROFILE_ID,
            person_contact_id: parse_id(person_id, "人物")?,
            occasion,
        };
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        validate_scope(&store, scope)?;
        let expected = store
            .scoped_register_preference(scope)?
            .as_ref()
            .map(|row| row.expected())
            .unwrap_or(ExpectedPreference::Absent);
        let applied = store.lookup_confirmed_register(scope)?;
        let mut brief = draft::brief(&store, OWNER_PROFILE_ID)?;
        let applied_preference = applied.and_then(|row| {
            row.register.map(|register| {
                brief.set_by_user(VoiceSetting::Register(register));
                ScopedRegisterReceipt {
                    preference_id: row.preference_id,
                    revision: row.revision,
                    memory_id: row.memory_id,
                    register,
                }
            })
        });
        // The same guard covers scope resolution, rendering and its audit.
        let drafted = draft::draft_pasted(&self.draft, &mut self.policy, brief, pasted)?;
        store.transact(|store| -> Result<(), SessionRefusal> {
            for entry in &drafted.audit {
                append_or_store_error(store, entry.clone(), now_unix_seconds())?;
            }
            Ok(())
        })?;
        let result_id = Uuid::now_v7();
        self.held_scoped_result = Some(HeldScopedResult {
            id: result_id,
            generation: self.draft_revision,
            scope,
            expected,
        });
        Ok(ScopedDraftView {
            result_id,
            person_id: scope.person_contact_id,
            occasion,
            draft: drafted.draft,
            applied_preference,
        })
    }

    pub fn propose_scoped_register(
        &mut self,
        result_id: &str,
        edit_revision: u64,
        register: VoiceRegister,
    ) -> Result<ScopedRegisterProposal, SessionRefusal> {
        self.require_store_idle()?;
        self.held_scoped_proposal = None;
        let id = parse_id(result_id, "本次草稿")?;
        let result = self.held_scoped_result.as_ref().ok_or_else(changed)?;
        if result.id != id || result.generation != self.draft_revision {
            return Err(changed());
        }
        let shared = self.opened_store()?;
        let store = hold(&shared);
        validate_scope(&store, result.scope)?;
        store.assert_scoped_register_revision(result.scope, result.expected)?;
        let view = ScopedRegisterProposal {
            proposal_id: Uuid::now_v7(),
            result_id: id,
            edit_revision,
            person_id: result.scope.person_contact_id,
            occasion: result.scope.occasion,
            register,
            expected_revision: revision(result.expected),
            notice: notice(result.scope, register),
        };
        self.held_scoped_proposal = Some(HeldScopedProposal {
            view: view.clone(),
            scope: result.scope,
            expected: result.expected,
            generation: result.generation,
        });
        Ok(view)
    }

    pub fn confirm_scoped_register(
        &mut self,
        proposal_id: &str,
        expected_revision: Option<Uuid>,
    ) -> Result<ScopedRegisterReceipt, SessionRefusal> {
        self.require_store_idle()?;
        let id = parse_id(proposal_id, "偏好确认")?;
        let proposal = self.held_scoped_proposal.take().ok_or_else(changed)?;
        let result = self.held_scoped_result.as_ref().ok_or_else(changed)?;
        if proposal.view.proposal_id != id
            || proposal.view.expected_revision != expected_revision
            || proposal.generation != self.draft_revision
            || result.id != proposal.view.result_id
            || result.scope != proposal.scope
            || result.expected != proposal.expected
        {
            return Err(changed());
        }
        // Any submitted confirmation is one-use, including an audit/SQL failure.
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let at = now_unix_seconds();
        let saved = store.confirm_scoped_register::<SessionRefusal, _>(
            proposal.scope,
            proposal.expected,
            proposal.view.register,
            |store| {
                let current = store.scoped_register_preference(proposal.scope)?;
                let old = current
                    .map(|row| store.get_memory(row.memory_id))
                    .transpose()?;
                let memory = if let Some(old) =
                    old.filter(|memory| memory.forget_state == ForgetState::Active)
                {
                    memory_commands::write_change(
                        store,
                        old.memory_id,
                        &MemoryChange {
                            title: Some("我的范围语域偏好".to_owned()),
                            summary: Some(proposal.view.notice.clone()),
                            memory_type: None,
                        },
                        at,
                    )?
                } else {
                    memory_commands::write_new(
                        store,
                        &NewMemory {
                            memory_type: "preference".to_owned(),
                            title: "我的范围语域偏好".to_owned(),
                            summary: proposal.view.notice.clone(),
                        },
                        at,
                    )?
                };
                parse_id(&memory.memory_id, "偏好记忆")
            },
        )?;
        Ok(ScopedRegisterReceipt {
            preference_id: saved.preference_id,
            revision: saved.revision,
            memory_id: saved.memory_id,
            register: proposal.view.register,
        })
    }

    pub fn discard_scoped_register_proposal(
        &mut self,
        proposal_id: &str,
    ) -> Result<bool, SessionRefusal> {
        let id = parse_id(proposal_id, "偏好确认")?;
        if self
            .held_scoped_proposal
            .as_ref()
            .is_some_and(|held| held.view.proposal_id == id)
        {
            self.held_scoped_proposal = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
