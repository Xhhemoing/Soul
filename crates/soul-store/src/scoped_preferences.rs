//! Exact-scope preferences backed by an ordinary sealed Preference memory.
use rusqlite::{params, OptionalExtension};
use soul_schema::common::{field_aad, SealedSubject};
use soul_schema::contact::ContactClass;
use soul_schema::memory::{ForgetState, MemoryType};
use soul_store_api::scoped_preference::{
    ExactPreferenceScope, ExpectedPreference, ScopedPreferenceState, ScopedRegisterPreference,
    VoiceRegister,
};
use soul_store_api::types::{StoreError, StoreResult};
use soul_store_api::{BlobStore, GraphStore, MemoryStore, ProfileStore};
use uuid::Uuid;

use crate::store::{backend, enum_text, SqlCipherStore};

fn refused() -> StoreError {
    StoreError::ContractViolation("scoped preference is unavailable or its revision changed".into())
}

impl SqlCipherStore {
    /// Metadata includes inactive tombstones, so forgotten is never mistaken
    /// for a scope that has not existed. It does not authorize application.
    pub fn scoped_register_preference(
        &self,
        scope: ExactPreferenceScope,
    ) -> StoreResult<Option<ScopedRegisterPreference>> {
        let raw: Option<(String, String, String, Option<String>, String)> = self.conn.query_row(
            "SELECT preference_id,revision,memory_id,register,state FROM scoped_register_preferences WHERE owner_profile_id=?1 AND person_contact_id=?2 AND occasion=?3",
            params![scope.owner_profile_id.to_string(), scope.person_contact_id.to_string(), enum_text(&scope.occasion)?],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
        ).optional().map_err(backend)?;
        raw.map(|(id, revision, memory, register, state)| {
            Ok(ScopedRegisterPreference {
                preference_id: id.parse().map_err(backend)?,
                revision: revision.parse().map_err(backend)?,
                scope,
                memory_id: memory.parse().map_err(backend)?,
                register: register
                    .map(|r| serde_json::from_value(serde_json::Value::String(r)).map_err(backend))
                    .transpose()?,
                state: serde_json::from_value(serde_json::Value::String(state)).map_err(backend)?,
            })
        })
        .transpose()
    }

    fn validate_register_scope(&self, scope: ExactPreferenceScope) -> StoreResult<()> {
        self.get_profile(scope.owner_profile_id)?;
        let person = self.get_contact(scope.person_contact_id)?;
        if person.contact_class != ContactClass::ThirdParty
            || person.forget_state != ForgetState::Active
        {
            return Err(refused());
        }
        Ok(())
    }

    fn validate_register_memory(&self, memory_id: Uuid) -> StoreResult<()> {
        let memory = self.get_memory(memory_id)?;
        if memory.memory_type != MemoryType::Preference
            || memory.forget_state != ForgetState::Active
            || memory.third_party_content_present != Some(false)
            || !self.has_content_key(memory.content_key_id)
            || memory.summary_ref.is_none()
        {
            return Err(refused());
        }
        for (field, sealed) in [
            ("title_ref", &memory.title_ref),
            ("summary_ref", &memory.summary_ref),
        ] {
            if let Some(sealed) = sealed {
                if sealed.content_key_id != memory.content_key_id
                    || sealed.subject != SealedSubject::Owner
                    || sealed.aad.as_deref() != Some(field_aad(&memory_id, field).as_str())
                {
                    return Err(refused());
                }
                self.open(sealed)?;
            }
        }
        Ok(())
    }

    /// Resolve only this exact scope and verify all dependencies before use.
    /// Callers render while retaining the same store guard; detached metadata
    /// is not a reusable authorization receipt.
    pub fn lookup_confirmed_register(
        &self,
        scope: ExactPreferenceScope,
    ) -> StoreResult<Option<ScopedRegisterPreference>> {
        let Some(row) = self.scoped_register_preference(scope)? else {
            return Ok(None);
        };
        if row.state != ScopedPreferenceState::Active || row.register.is_none() {
            return Ok(None);
        }
        match self
            .validate_register_scope(scope)
            .and_then(|_| self.validate_register_memory(row.memory_id))
        {
            Ok(()) => Ok(Some(row)),
            Err(
                StoreError::NotFound { .. }
                | StoreError::ContractViolation(_)
                | StoreError::ContentKeyDestroyed(_)
                | StoreError::BlobMissing(_)
                | StoreError::SealBroken(_),
            ) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn assert_scoped_register_revision(
        &self,
        scope: ExactPreferenceScope,
        expected: ExpectedPreference,
    ) -> StoreResult<()> {
        let current = self.scoped_register_preference(scope)?;
        if current
            .as_ref()
            .map(ScopedRegisterPreference::expected)
            .unwrap_or(ExpectedPreference::Absent)
            != expected
        {
            return Err(refused());
        }
        Ok(())
    }

    /// Low-level CAS primitive. Confirmation callers use
    /// `confirm_scoped_register` so Memory writes, invalidation and activation
    /// share a transaction. This operation never parses Memory prose.
    pub fn put_scoped_register(
        &mut self,
        scope: ExactPreferenceScope,
        expected: ExpectedPreference,
        memory_id: Uuid,
        register: VoiceRegister,
    ) -> StoreResult<ScopedRegisterPreference> {
        self.assert_scoped_register_revision(scope, expected)?;
        self.validate_register_scope(scope)?;
        self.validate_register_memory(memory_id)?;
        let current = self.scoped_register_preference(scope)?;
        if let Some(old) = &current {
            if old.memory_id != memory_id {
                let old_memory = self.get_memory(old.memory_id)?;
                let fresh = self.get_memory(memory_id)?;
                if old_memory.forget_state != ForgetState::Forgotten
                    || self.has_content_key(old_memory.content_key_id)
                    || fresh.content_key_id == old_memory.content_key_id
                {
                    return Err(refused());
                }
            }
        }
        let preference_id = current
            .as_ref()
            .map(|r| r.preference_id)
            .unwrap_or_else(Uuid::now_v7);
        let revision = Uuid::now_v7();
        let changed = match expected {
            ExpectedPreference::Absent => self.conn.execute(
                "INSERT INTO scoped_register_preferences(preference_id,revision,owner_profile_id,person_contact_id,occasion,memory_id,register,state) VALUES(?1,?2,?3,?4,?5,?6,?7,'active')",
                params![preference_id.to_string(),revision.to_string(),scope.owner_profile_id.to_string(),scope.person_contact_id.to_string(),enum_text(&scope.occasion)?,memory_id.to_string(),enum_text(&register)?],
            ),
            ExpectedPreference::Revision { preference_id, revision: previous } => self.conn.execute(
                "UPDATE scoped_register_preferences SET revision=?2,memory_id=?3,register=?4,state='active' WHERE preference_id=?1 AND revision=?5",
                params![preference_id.to_string(),revision.to_string(),memory_id.to_string(),enum_text(&register)?,previous.to_string()],
            ),
        }.map_err(backend)?;
        if changed != 1 {
            return Err(refused());
        }
        Ok(ScopedRegisterPreference {
            preference_id,
            revision,
            scope,
            memory_id,
            register: Some(register),
            state: ScopedPreferenceState::Active,
        })
    }

    /// Own the complete confirmation transaction. Ordinary Memory updates
    /// advance the binding revision; re-read that revision only after checking
    /// the original expectation inside this same transaction.
    pub fn confirm_scoped_register<E, F>(
        &mut self,
        scope: ExactPreferenceScope,
        expected: ExpectedPreference,
        register: VoiceRegister,
        write_memory: F,
    ) -> Result<ScopedRegisterPreference, E>
    where
        E: From<StoreError>,
        F: FnOnce(&mut SqlCipherStore) -> Result<Uuid, E>,
    {
        self.transact(|store| {
            store.assert_scoped_register_revision(scope, expected)?;
            store.validate_register_scope(scope)?;
            let memory_id = write_memory(store)?;
            let after_edit = store
                .scoped_register_preference(scope)?
                .as_ref()
                .map(ScopedRegisterPreference::expected)
                .unwrap_or(ExpectedPreference::Absent);
            store
                .put_scoped_register(scope, after_edit, memory_id, register)
                .map_err(E::from)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestKeyProvider;
    use soul_schema::common::{EvidenceBand, NotAClinicalClaim, SchemaVersion, SealedSubject};
    use soul_schema::contact::{ContactClass, SoulContact};
    use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
    use soul_schema::profile::{AxisPosition, SoulProfile, TraitAxis};
    use soul_store_api::forget::{ForgetOps, ForgetUnit};
    use soul_store_api::scoped_preference::{PreferenceOccasion, ScopedPreferenceState};
    use soul_store_api::{BlobStore, GraphStore, MemoryStore, ProfileStore, SealRequest};

    fn fixture(store: &mut SqlCipherStore) -> ExactPreferenceScope {
        let scope = ExactPreferenceScope {
            owner_profile_id: Uuid::now_v7(),
            person_contact_id: Uuid::now_v7(),
            occasion: PreferenceOccasion::Work,
        };
        store
            .put_profile(SoulProfile {
                schema_version: SchemaVersion,
                profile_id: scope.owner_profile_id,
                voice: serde_json::json!({}),
                trait_axes: vec![TraitAxis {
                    axis_id: Uuid::now_v7(),
                    label: None,
                    position: AxisPosition::Unknown,
                    evidence_band: EvidenceBand::None,
                    evidence_ids: None,
                    locked_by_user: None,
                    clinical_claim: NotAClinicalClaim,
                }],
                values: None,
                boundaries: None,
                clinical_claim: NotAClinicalClaim,
            })
            .unwrap();
        store
            .put_contact(SoulContact {
                schema_version: SchemaVersion,
                contact_id: scope.person_contact_id,
                contact_class: ContactClass::ThirdParty,
                display_label_ref: None,
                identifiers: None,
                forget_state: ForgetState::Active,
            })
            .unwrap();
        scope
    }

    fn memory(store: &mut SqlCipherStore) -> SoulMemory {
        let id = Uuid::now_v7();
        let key = Uuid::now_v7();
        let summary = store
            .seal(SealRequest::new(
                key,
                id,
                "summary_ref",
                SealedSubject::Owner,
                b"Use the explicitly confirmed register for this person and occasion.".to_vec(),
            ))
            .unwrap();
        let memory = SoulMemory {
            schema_version: SchemaVersion,
            memory_id: id,
            memory_type: MemoryType::Preference,
            title_ref: None,
            summary_ref: Some(summary),
            source_event_ids: None,
            evidence_ids: None,
            content_key_id: key,
            forget_state: ForgetState::Active,
            third_party_content_present: Some(false),
        };
        store.put_memory(memory.clone()).unwrap();
        memory
    }

    #[test]
    fn scoped_register_exact_scope_cas_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scope.db");
        let keys = TestKeyProvider::from_seed("synthetic scoped register");
        let mut store = SqlCipherStore::open(&path, &keys).unwrap();
        let scope = fixture(&mut store);
        let mem = memory(&mut store);
        let row = store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Formal,
            )
            .unwrap();
        assert_eq!(
            store.lookup_confirmed_register(scope).unwrap(),
            Some(row.clone())
        );
        assert!(store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Casual
            )
            .is_err());
        for different in [
            ExactPreferenceScope {
                occasion: PreferenceOccasion::Personal,
                ..scope
            },
            ExactPreferenceScope {
                person_contact_id: Uuid::now_v7(),
                ..scope
            },
            ExactPreferenceScope {
                owner_profile_id: Uuid::now_v7(),
                ..scope
            },
        ] {
            assert!(store
                .lookup_confirmed_register(different)
                .unwrap()
                .is_none());
        }
        let newer = store
            .put_scoped_register(scope, row.expected(), mem.memory_id, VoiceRegister::Plain)
            .unwrap();
        assert_ne!(newer.revision, row.revision);
        assert!(store
            .put_scoped_register(scope, row.expected(), mem.memory_id, VoiceRegister::Casual)
            .is_err());
        store.close().unwrap();
        let reopened = SqlCipherStore::open(&path, &keys).unwrap();
        assert_eq!(
            reopened.lookup_confirmed_register(scope).unwrap(),
            Some(newer)
        );
    }

    #[test]
    fn scoped_register_ordinary_edit_invalidates_and_forbids_type_or_key_escape() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("edit.db"),
            &TestKeyProvider::from_seed("scoped edit"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let mut mem = memory(&mut store);
        let row = store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Formal,
            )
            .unwrap();
        mem.evidence_ids = Some(vec![]);
        store.put_memory(mem.clone()).unwrap();
        assert!(store.lookup_confirmed_register(scope).unwrap().is_none());
        let inactive = store.scoped_register_preference(scope).unwrap().unwrap();
        assert_eq!(inactive.state, ScopedPreferenceState::Inactive);
        assert_eq!(inactive.register, None);
        assert_ne!(row.revision, inactive.revision);
        mem.memory_type = MemoryType::Semantic;
        assert!(store.put_memory(mem.clone()).is_err());
        mem.memory_type = MemoryType::Preference;
        mem.content_key_id = Uuid::now_v7();
        assert!(store.put_memory(mem).is_err());
    }

    #[test]
    fn scoped_register_all_forget_units_clear_values_and_refuse_resurrection() {
        for mode in 0..3 {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("forget.db");
            let keys = TestKeyProvider::from_seed("scoped forget");
            let mut store = SqlCipherStore::open(&path, &keys).unwrap();
            let scope = fixture(&mut store);
            let mem = memory(&mut store);
            let row = store
                .put_scoped_register(
                    scope,
                    ExpectedPreference::Absent,
                    mem.memory_id,
                    VoiceRegister::Formal,
                )
                .unwrap();
            let unit = match mode {
                0 => ForgetUnit::Memory(mem.memory_id),
                1 => ForgetUnit::ContentKey(mem.content_key_id),
                _ => ForgetUnit::Contact(scope.person_contact_id),
            };
            assert!(store
                .preview_impact(unit)
                .unwrap()
                .content_key_ids
                .contains(&mem.content_key_id));
            store.execute_forget(unit).unwrap();
            assert!(!store.has_content_key(mem.content_key_id));
            assert!(store.open(mem.summary_ref.as_ref().unwrap()).is_err());
            assert!(store.lookup_confirmed_register(scope).unwrap().is_none());
            let tombstone = store.scoped_register_preference(scope).unwrap().unwrap();
            assert_eq!(tombstone.register, None);
            assert_ne!(row.revision, tombstone.revision);
            assert!(store
                .put_scoped_register(
                    scope,
                    tombstone.expected(),
                    mem.memory_id,
                    VoiceRegister::Plain
                )
                .is_err());
            store.close().unwrap();
            let reopened = SqlCipherStore::open(&path, &keys).unwrap();
            assert!(reopened.lookup_confirmed_register(scope).unwrap().is_none());
        }
    }

    #[test]
    fn scoped_register_same_count_change_invalidates_forget_quote() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("quote.db"),
            &TestKeyProvider::from_seed("scoped quote"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let mem = memory(&mut store);
        let row = store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Formal,
            )
            .unwrap();
        let quote = store
            .preview_forget_scope(ForgetUnit::Memory(mem.memory_id))
            .unwrap();
        store
            .put_scoped_register(scope, row.expected(), mem.memory_id, VoiceRegister::Plain)
            .unwrap();
        assert_eq!(
            store
                .preview_impact(ForgetUnit::Memory(mem.memory_id))
                .unwrap(),
            *quote.impact()
        );
        assert!(!store.forget_scope_is_current(&quote).unwrap());
    }

    #[test]
    fn scoped_register_transaction_failure_leaves_no_memory_key_or_binding() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("rollback.db"),
            &TestKeyProvider::from_seed("scoped rollback"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let result: StoreResult<()> = store.transact(|store| {
            let mem = memory(store);
            store.put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Formal,
            )?;
            Err(StoreError::ContractViolation(
                "synthetic audit failure".into(),
            ))
        });
        assert!(result.is_err());
        assert_eq!(store.content_key_count().unwrap(), 0);
        assert!(store.list_memories().unwrap().is_empty());
        assert!(store.scoped_register_preference(scope).unwrap().is_none());
    }

    #[test]
    fn scoped_register_old_database_keeps_ordinary_preferences_unscoped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        let keys = TestKeyProvider::from_seed("scoped old database");
        let mut store = SqlCipherStore::open(&path, &keys).unwrap();
        let scope = fixture(&mut store);
        let mem = memory(&mut store);
        // Simulate the schema immediately before this feature, preserving its
        // real encrypted ordinary Memory rather than reconstructing prose.
        store.conn.execute_batch("DROP TABLE IF EXISTS scoped_register_preferences; UPDATE meta SET value='7' WHERE key='schema_version';").unwrap();
        store.close().unwrap();
        let reopened = SqlCipherStore::open(&path, &keys).unwrap();
        assert_eq!(crate::sql::STORE_SCHEMA_VERSION, 8);
        assert!(reopened
            .scoped_register_preference(scope)
            .unwrap()
            .is_none());
        assert_eq!(reopened.get_memory(mem.memory_id).unwrap(), mem);
        assert!(reopened.open(mem.summary_ref.as_ref().unwrap()).is_ok());
    }

    #[test]
    fn scoped_register_invalid_person_and_broken_memory_never_apply() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("invalid.db"),
            &TestKeyProvider::from_seed("scoped invalid"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let mem = memory(&mut store);
        let mut person = store.get_contact(scope.person_contact_id).unwrap();
        for class in [
            ContactClass::Group,
            ContactClass::Owner,
            ContactClass::Service,
        ] {
            person.contact_class = class;
            store.put_contact(person.clone()).unwrap();
            assert!(store
                .put_scoped_register(
                    scope,
                    ExpectedPreference::Absent,
                    mem.memory_id,
                    VoiceRegister::Formal
                )
                .is_err());
        }
        person.contact_class = ContactClass::ThirdParty;
        store.put_contact(person.clone()).unwrap();
        store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Formal,
            )
            .unwrap();
        person.forget_state = ForgetState::Forgotten;
        store.put_contact(person).unwrap();
        assert!(store.lookup_confirmed_register(scope).unwrap().is_none());
    }

    #[test]
    fn scoped_register_confirm_wrapper_checks_before_callback_and_reactivates_atomic_edit() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("confirm.db"),
            &TestKeyProvider::from_seed("scoped confirm"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let row = store
            .confirm_scoped_register::<StoreError, _>(
                scope,
                ExpectedPreference::Absent,
                VoiceRegister::Formal,
                |store| Ok(memory(store).memory_id),
            )
            .unwrap();
        let mut called = false;
        let stale = store.confirm_scoped_register::<StoreError, _>(
            scope,
            ExpectedPreference::Absent,
            VoiceRegister::Casual,
            |_| {
                called = true;
                Err(refused())
            },
        );
        assert!(stale.is_err());
        assert!(!called);
        let newer = store
            .confirm_scoped_register::<StoreError, _>(
                scope,
                row.expected(),
                VoiceRegister::Plain,
                |store| {
                    let mut mem = store.get_memory(row.memory_id)?;
                    mem.evidence_ids = Some(vec![]);
                    store.put_memory(mem)?;
                    assert!(store.lookup_confirmed_register(scope)?.is_none());
                    Ok(row.memory_id)
                },
            )
            .unwrap();
        assert_eq!(newer.memory_id, row.memory_id);
        assert_eq!(
            store.lookup_confirmed_register(scope).unwrap(),
            Some(newer.clone())
        );
        let failed = store.confirm_scoped_register::<StoreError, _>(
            scope,
            newer.expected(),
            VoiceRegister::Casual,
            |store| {
                let mut mem = store.get_memory(row.memory_id)?;
                mem.evidence_ids = None;
                store.put_memory(mem)?;
                Err(StoreError::ContractViolation(
                    "synthetic audit failure".into(),
                ))
            },
        );
        assert!(failed.is_err());
        assert_eq!(store.lookup_confirmed_register(scope).unwrap(), Some(newer));
    }

    #[test]
    fn scoped_register_fresh_confirmation_after_forget_uses_new_memory_and_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("fresh.db"),
            &TestKeyProvider::from_seed("scoped fresh confirmation"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let old_mem = memory(&mut store);
        let row = store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                old_mem.memory_id,
                VoiceRegister::Formal,
            )
            .unwrap();
        // A live Memory cannot be abandoned to evade later person forgetting.
        let other = memory(&mut store);
        assert!(store
            .put_scoped_register(scope, row.expected(), other.memory_id, VoiceRegister::Plain)
            .is_err());
        store
            .execute_forget(ForgetUnit::Memory(old_mem.memory_id))
            .unwrap();
        let tombstone = store.scoped_register_preference(scope).unwrap().unwrap();
        assert!(store.put_memory(old_mem.clone()).is_err());
        assert!(store
            .seal(SealRequest::new(
                old_mem.content_key_id,
                old_mem.memory_id,
                "summary_ref",
                SealedSubject::Owner,
                b"synthetic resurrection".to_vec()
            ))
            .is_err());
        let fresh = store
            .confirm_scoped_register::<StoreError, _>(
                scope,
                tombstone.expected(),
                VoiceRegister::Casual,
                |store| Ok(memory(store).memory_id),
            )
            .unwrap();
        assert_ne!(fresh.memory_id, old_mem.memory_id);
        assert_ne!(
            store.get_memory(fresh.memory_id).unwrap().content_key_id,
            old_mem.content_key_id
        );
        assert_eq!(fresh.preference_id, row.preference_id);
        assert!(store
            .put_scoped_register(
                scope,
                row.expected(),
                old_mem.memory_id,
                VoiceRegister::Formal
            )
            .is_err());
    }

    #[test]
    fn scoped_register_fault_during_invalidation_or_forget_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("sql-fault.db"),
            &TestKeyProvider::from_seed("scoped sql fault"),
        )
        .unwrap();
        let scope = fixture(&mut store);
        let mem = memory(&mut store);
        let row = store
            .put_scoped_register(
                scope,
                ExpectedPreference::Absent,
                mem.memory_id,
                VoiceRegister::Formal,
            )
            .unwrap();
        store.conn.execute_batch("CREATE TEMP TRIGGER fail_scoped_update BEFORE UPDATE ON scoped_register_preferences BEGIN SELECT RAISE(ABORT,'synthetic scoped SQL failure'); END;").unwrap();
        let mut edited = mem.clone();
        edited.evidence_ids = Some(vec![]);
        assert!(store.put_memory(edited).is_err());
        assert_eq!(store.get_memory(mem.memory_id).unwrap(), mem);
        assert!(store
            .execute_forget(ForgetUnit::Contact(scope.person_contact_id))
            .is_err());
        assert!(store.has_content_key(mem.content_key_id));
        assert!(store.open(mem.summary_ref.as_ref().unwrap()).is_ok());
        assert_eq!(store.lookup_confirmed_register(scope).unwrap(), Some(row));
    }
}
