//! Durable, prose-free import operations and sealed natural-identity indexes.
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

use fs4::FileExt;
use rusqlite::OptionalExtension;
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use soul_schema::common::{field_aad, SealedSubject, SealedText};
use soul_store_api::BlobStore;

use soul_store_api::import::{
    ImportAuditState, ImportCounts, ImportIndexStore, ImportMessageIdentity, ImportNamespace,
    ImportNamespaceState, ImportOperation, ImportOperationError, ImportOperationState,
    ImportParticipantIdentity, ImportSourceKind,
};
use soul_store_api::types::{StoreError, StoreResult};

use crate::store::{backend, enum_text, SqlCipherStore};

/// An OS-held claim. The empty file is never removed or interpreted as a flag.
pub struct ImportRuntimeLease {
    _file: File,
    database_path: PathBuf,
    recovery: AtomicU8,
}

impl std::fmt::Debug for ImportRuntimeLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportRuntimeLease").finish_non_exhaustive()
    }
}

fn identity_path(path: &Path) -> StoreResult<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|_| fixed("IMPORT_LOCK_UNAVAILABLE"));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|_| fixed("IMPORT_LOCK_UNAVAILABLE"))?;
    let parent = parent
        .canonicalize()
        .map_err(|_| fixed("IMPORT_LOCK_UNAVAILABLE"))?;
    let name = path
        .file_name()
        .ok_or_else(|| fixed("IMPORT_LOCK_UNAVAILABLE"))?;
    Ok(parent.join(name))
}

fn fixed(code: &str) -> StoreError {
    StoreError::ContractViolation(code.to_owned())
}

impl ImportRuntimeLease {
    pub fn claim(database_path: impl AsRef<Path>) -> StoreResult<Self> {
        let database_path = identity_path(database_path.as_ref())?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(database_path.with_extension("import.lock"))
            .map_err(|_| fixed("IMPORT_LOCK_UNAVAILABLE"))?;
        FileExt::try_lock(&file).map_err(|_| fixed("IMPORT_RUNTIME_BUSY"))?;
        Ok(Self {
            _file: file,
            database_path,
            recovery: AtomicU8::new(0),
        })
    }

    fn check(&self, store: &SqlCipherStore) -> StoreResult<()> {
        if self.database_path != identity_path(store.path())? {
            return Err(fixed("IMPORT_RUNTIME_MISMATCH"));
        }
        Ok(())
    }

    fn ready(&self, store: &SqlCipherStore) -> StoreResult<()> {
        self.check(store)?;
        if self.recovery.load(Ordering::Acquire) != 2 {
            return Err(fixed("IMPORT_RECOVERY_REQUIRED"));
        }
        Ok(())
    }
}

fn encode<T: Serialize>(value: &T) -> StoreResult<String> {
    serde_json::to_string(value).map_err(|_| fixed("IMPORT_RECORD_INVALID"))
}

fn decode<T: DeserializeOwned>(value: &str) -> StoreResult<T> {
    serde_json::from_str(value).map_err(|_| fixed("IMPORT_RECORD_INVALID"))
}

impl SqlCipherStore {
    /// Exact indexed provenance for a local source view. The caller supplies
    /// its trusted resource limit; no raw identifier is returned or logged.
    /// Both authorization seals are size-checked before either is opened.
    pub fn import_source_for_event(
        &self,
        event_id: Uuid,
        max_identity_plaintext_bytes: u64,
    ) -> StoreResult<Option<(ImportMessageIdentity, ImportNamespace)>> {
        let row: Option<(String, String, String, String, String)> = self.conn.query_row(
            "SELECT identity_id,namespace_id,contact_id,content_key_id,doc FROM import_message_keys WHERE event_id=?1",
            [event_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).optional().map_err(backend)?;
        let Some((id, namespace_id, contact, key, doc)) = row else {
            return Ok(None);
        };
        let identity: ImportMessageIdentity = decode(&doc)?;
        if identity.event_id != event_id
            || identity.identity_id.to_string() != id
            || identity.namespace_id.to_string() != namespace_id
            || identity.sender_contact_id.to_string() != contact
            || identity.content_key_id.to_string() != key
            || identity.identity_ref.content_key_id != identity.content_key_id
            || identity.identity_ref.subject
                != if identity.is_owner {
                    SealedSubject::Owner
                } else {
                    SealedSubject::ThirdParty
                }
        {
            return Err(fixed("LOCAL_SOURCE_UNAVAILABLE"));
        }
        let row: Option<(String, String, Option<String>, String)> = self.conn.query_row(
            "SELECT source,state,owner_contact_id,doc FROM import_namespaces WHERE namespace_id=?1",
            [namespace_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).optional().map_err(backend)?;
        let (source, state, owner, doc) = row.ok_or_else(|| fixed("LOCAL_SOURCE_UNAVAILABLE"))?;
        let namespace: ImportNamespace = decode(&doc)?;
        if state != "active" || namespace.state != ImportNamespaceState::Active {
            return Err(fixed("LOCAL_SOURCE_UNAVAILABLE"));
        }
        if self.sealed_plaintext_bytes_for_field(
            event_id,
            "import_message_identity",
            &identity.identity_ref,
        )? > max_identity_plaintext_bytes
        {
            return Err(fixed("LOCAL_SOURCE_TOO_LARGE"));
        }
        if let Some(account) = &namespace.account_ref {
            let owner = namespace
                .owner_contact_id
                .ok_or_else(|| fixed("LOCAL_SOURCE_UNAVAILABLE"))?;
            if self.sealed_plaintext_bytes_for_field(owner, "import_account_identity", account)?
                > max_identity_plaintext_bytes
            {
                return Err(fixed("LOCAL_SOURCE_TOO_LARGE"));
            }
        }
        let namespace = read_namespace(
            self,
            identity.namespace_id,
            &source,
            &state,
            owner.as_deref(),
            &doc,
        )?;
        validate_identity_ref(
            self,
            &identity.identity_ref,
            identity.content_key_id,
            event_id,
            "import_message_identity",
        )?;
        Ok(Some((identity, namespace)))
    }

    pub fn unfinished_import_counts(&self) -> StoreResult<(u64, u64)> {
        self.conn.query_row("SELECT count(CASE WHEN state='reserved' THEN 1 END),count(CASE WHEN state='running' THEN 1 END) FROM import_operations", [], |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, i64>(1)? as u64))).map_err(backend)
    }
    /// Keep ownership with every Arc<Mutex<SqlCipherStore>>, not just Session.
    pub fn attach_import_runtime(
        &mut self,
        lease: std::sync::Arc<ImportRuntimeLease>,
    ) -> StoreResult<()> {
        lease.ready(self)?;
        if self.import_runtime.is_some() {
            return Err(fixed("IMPORT_RUNTIME_ALREADY_ATTACHED"));
        }
        self.import_runtime = Some(lease);
        Ok(())
    }

    pub fn import_operation(&self, id: Uuid) -> StoreResult<ImportOperation> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM import_operations WHERE operation_id=?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        decode(&doc.ok_or_else(|| StoreError::not_found("import operation", id))?)
    }

    pub fn recent_import_operations(&self, limit: usize) -> StoreResult<Vec<ImportOperation>> {
        let mut statement = self.conn.prepare(
            "SELECT doc FROM import_operations ORDER BY created_at DESC, operation_id DESC LIMIT ?1"
        ).map_err(backend)?;
        let docs = statement
            .query_map([limit.min(100) as i64], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        docs.map(|doc| decode(&doc.map_err(backend)?)).collect()
    }

    /// Call once after taking the process-lifetime lease and before admitting work.
    pub fn recover_import_operations(&mut self, lease: &ImportRuntimeLease) -> StoreResult<u64> {
        lease.check(self)?;
        lease
            .recovery
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| fixed("IMPORT_RECOVERY_ALREADY_STARTED"))?;
        let result = self.transact(|store| {
            store.recover_assistance_work()?;
            let mut statement = store
                .conn
                .prepare("SELECT doc FROM import_operations WHERE state IN ('reserved','running')")
                .map_err(backend)?;
            let docs = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(backend)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(backend)?;
            drop(statement);
            for doc in &docs {
                let mut operation: ImportOperation = decode(doc)?;
                operation.state = ImportOperationState::Interrupted;
                operation.error_code = Some(ImportOperationError::Interrupted);
                operation.audit_state = ImportAuditState::NotApplicable;
                store.write_import_operation(&operation)?;
            }
            Ok(docs.len() as u64)
        });
        lease
            .recovery
            .store(if result.is_ok() { 2 } else { 0 }, Ordering::Release);
        result
    }

    pub fn reserve_import_operation(
        &mut self,
        lease: &ImportRuntimeLease,
        operation: &ImportOperation,
    ) -> StoreResult<()> {
        lease.ready(self)?;
        if operation.state != ImportOperationState::Reserved
            || operation.receipt.is_some()
            || operation.error_code.is_some()
            || operation.audit_state != ImportAuditState::NotApplicable
        {
            return Err(fixed("IMPORT_RECORD_INVALID"));
        }
        self.conn.execute(
            "INSERT INTO import_operations(operation_id,preview_id,namespace_id,state,created_at,doc) VALUES(?1,?2,?3,'reserved',?4,?5)",
            rusqlite::params![operation.operation_id.to_string(),operation.preview_id.to_string(),operation.namespace_id.to_string(),operation.created_at,encode(operation)?],
        ).map_err(|_| fixed("IMPORT_OPERATION_EXISTS"))?;
        Ok(())
    }

    /// CAS under the caller's Session lock; no second worker may consume this preview.
    pub fn start_import_operation(
        &mut self,
        lease: &ImportRuntimeLease,
        id: Uuid,
        preview: Uuid,
    ) -> StoreResult<ImportOperation> {
        lease.ready(self)?;
        self.transact(|store| {
            let mut operation = store.import_operation(id)?;
            if operation.preview_id != preview || operation.state != ImportOperationState::Reserved
            {
                return Err(fixed("IMPORT_PREVIEW_MISMATCH"));
            }
            operation.state = ImportOperationState::Running;
            store.write_import_operation(&operation)?;
            Ok(operation)
        })
    }

    /// Must participate in the domain/graph/audit transaction, never a later write.
    pub fn commit_import_operation(
        &mut self,
        lease: &ImportRuntimeLease,
        id: Uuid,
        receipt: ImportCounts,
    ) -> StoreResult<ImportOperation> {
        lease.ready(self)?;
        if self.conn.is_autocommit() {
            return Err(fixed("IMPORT_RECEIPT_REQUIRES_TRANSACTION"));
        }
        let mut operation = self.import_operation(id)?;
        if operation.state != ImportOperationState::Running {
            return Err(fixed("IMPORT_OPERATION_NOT_RUNNING"));
        }
        operation.state = ImportOperationState::Committed;
        operation.receipt = Some(receipt);
        operation.audit_state = ImportAuditState::Recorded;
        self.write_import_operation(&operation)?;
        Ok(operation)
    }

    pub fn end_import_operation(
        &mut self,
        lease: &ImportRuntimeLease,
        id: Uuid,
        expected: ImportOperationState,
        next: ImportOperationState,
        error: Option<ImportOperationError>,
    ) -> StoreResult<ImportOperation> {
        lease.ready(self)?;
        if !matches!(
            (expected, next),
            (
                ImportOperationState::Reserved,
                ImportOperationState::Discarded | ImportOperationState::Invalidated
            ) | (
                ImportOperationState::Running,
                ImportOperationState::Rejected | ImportOperationState::Invalidated
            )
        ) {
            return Err(fixed("IMPORT_TRANSITION_INVALID"));
        }
        self.transact(|store| {
            let mut operation = store.import_operation(id)?;
            if operation.state != expected {
                return Err(fixed("IMPORT_OPERATION_STATE_CHANGED"));
            }
            operation.state = next;
            operation.error_code = error;
            operation.audit_state = ImportAuditState::NotApplicable;
            store.write_import_operation(&operation)?;
            Ok(operation)
        })
    }

    fn write_import_operation(&mut self, operation: &ImportOperation) -> StoreResult<()> {
        self.conn
            .execute(
                "UPDATE import_operations SET state=?2, doc=?3 WHERE operation_id=?1",
                rusqlite::params![
                    operation.operation_id.to_string(),
                    enum_text(&operation.state)?,
                    encode(operation)?
                ],
            )
            .map_err(backend)?;
        Ok(())
    }
}

impl ImportIndexStore for SqlCipherStore {
    fn import_namespace(&self, id: Uuid) -> StoreResult<ImportNamespace> {
        let row: Option<(String,String,Option<String>,String)> = self.conn.query_row(
            "SELECT source,state,owner_contact_id,doc FROM import_namespaces WHERE namespace_id=?1",
            [id.to_string()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))
            .optional().map_err(backend)?;
        let (source, state, owner, doc) =
            row.ok_or_else(|| StoreError::not_found("import namespace", id))?;
        read_namespace(self, id, &source, &state, owner.as_deref(), &doc)
    }

    fn import_namespaces(&self) -> StoreResult<Vec<ImportNamespace>> {
        let mut statement=self.conn.prepare("SELECT namespace_id,source,state,owner_contact_id,doc FROM import_namespaces ORDER BY namespace_id").map_err(backend)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(backend)?;
        rows.map(|row| {
            let (id, source, state, owner, doc) = row.map_err(backend)?;
            read_namespace(
                self,
                id.parse().map_err(|_| fixed("IMPORT_RECORD_INVALID"))?,
                &source,
                &state,
                owner.as_deref(),
                &doc,
            )
        })
        .collect()
    }

    fn put_import_namespace(&mut self, namespace: &ImportNamespace) -> StoreResult<()> {
        validate_account_ref(self, namespace)?;
        match self.import_namespace(namespace.namespace_id) {
            Ok(existing) => {
                if existing.state == ImportNamespaceState::Revoked
                    || existing.source != namespace.source
                    || existing.coverage != namespace.coverage
                    || (existing.owner_contact_id.is_some()
                        && existing.owner_contact_id != namespace.owner_contact_id)
                    || (existing.account_ref.is_some()
                        && existing.account_ref != namespace.account_ref)
                {
                    return Err(fixed("IMPORT_NAMESPACE_CHANGED"));
                }
            }
            Err(StoreError::NotFound { .. }) => {}
            Err(error) => return Err(error),
        }
        let tx = self.conn.savepoint().map_err(backend)?;
        tx.execute("INSERT INTO import_namespaces(namespace_id,source,state,owner_contact_id,doc) VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(namespace_id) DO UPDATE SET state=excluded.state,owner_contact_id=excluded.owner_contact_id,doc=excluded.doc",
            rusqlite::params![namespace.namespace_id.to_string(),namespace.source.as_str(),enum_text(&namespace.state)?,namespace.owner_contact_id.map(|id|id.to_string()),encode(namespace)?]).map_err(backend)?;
        if namespace.state == ImportNamespaceState::Revoked {
            tx.execute("UPDATE assistance_items SET validity='stale',revision=?2 WHERE namespace_id=?1 AND validity='current'",rusqlite::params![namespace.namespace_id.to_string(),Uuid::now_v7().to_string()]).map_err(backend)?;
            let mut statement = tx
                .prepare("SELECT item_id FROM assistance_items WHERE namespace_id=?1")
                .map_err(backend)?;
            let ids = statement
                .query_map([namespace.namespace_id.to_string()], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(backend)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(backend)?;
            drop(statement);
            for id in ids {
                crate::assistance_work::enqueue(&tx, &id)?;
            }
        }
        tx.commit().map_err(backend)
    }

    fn import_legacy_events(&self, source: ImportSourceKind) -> StoreResult<u64> {
        let text: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key=?1",
                [format!("import.legacy.{}", source.as_str())],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        let recorded: u64 = text
            .unwrap_or_else(|| "0".into())
            .parse()
            .map_err(|_| fixed("IMPORT_RECORD_INVALID"))?;
        let event_source = match source {
            ImportSourceKind::SoulImportV1 => "import.soul_import_v1",
            ImportSourceKind::TelegramDesktop => "import.telegram_desktop",
        };
        // Also recognize append-only callers from this version: an absent
        // natural-key row must never be mistaken for complete history.
        let current:i64=self.conn.query_row("SELECT count(*) FROM events e WHERE e.source=?1 AND NOT EXISTS(SELECT 1 FROM import_indexed_events k WHERE k.event_id=e.event_id)",
            [event_source],|row|row.get(0)).map_err(backend)?;
        Ok(recorded.max(current as u64))
    }

    fn import_key_destroyed(&self, key: Uuid) -> StoreResult<bool> {
        self.content_key_destroyed(key)
    }

    fn import_contact_keys(&self, contact: Uuid) -> StoreResult<Vec<Uuid>> {
        let mut statement=self.conn.prepare("SELECT DISTINCT content_key_id FROM sealed_blobs WHERE row_id=?1 ORDER BY content_key_id").map_err(backend)?;
        let rows = statement
            .query_map([contact.to_string()], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        rows.map(|row| {
            row.map_err(backend)?
                .parse()
                .map_err(|_| fixed("IMPORT_RECORD_INVALID"))
        })
        .collect()
    }

    fn import_participant_batch(
        &self,
        namespace: Uuid,
        after: Option<Uuid>,
        limit: usize,
    ) -> StoreResult<Vec<ImportParticipantIdentity>> {
        identity_batch(
            self,
            "import_participants",
            namespace,
            after,
            limit,
            |identity: &ImportParticipantIdentity, row| {
                if identity.identity_id.to_string() != row.0
                    || identity.namespace_id != namespace
                    || identity.contact_id.to_string() != row.1
                    || identity.content_key_id.to_string() != row.2
                {
                    return Err(fixed("IMPORT_RECORD_INVALID"));
                }
                validate_identity_ref(
                    self,
                    &identity.identity_ref,
                    identity.content_key_id,
                    identity.contact_id,
                    "import_participant_identity",
                )
            },
        )
    }

    fn import_message_batch(
        &self,
        namespace: Uuid,
        after: Option<Uuid>,
        limit: usize,
    ) -> StoreResult<Vec<ImportMessageIdentity>> {
        identity_batch(
            self,
            "import_message_keys",
            namespace,
            after,
            limit,
            |identity: &ImportMessageIdentity, row| {
                if identity.identity_id.to_string() != row.0
                    || identity.namespace_id != namespace
                    || identity.sender_contact_id.to_string() != row.1
                    || identity.content_key_id.to_string() != row.2
                    || row.3.as_deref() != Some(identity.event_id.to_string().as_str())
                {
                    return Err(fixed("IMPORT_RECORD_INVALID"));
                }
                validate_identity_ref(
                    self,
                    &identity.identity_ref,
                    identity.content_key_id,
                    identity.event_id,
                    "import_message_identity",
                )
            },
        )
    }

    fn put_import_participant(&mut self, identity: &ImportParticipantIdentity) -> StoreResult<()> {
        validate_identity_ref(
            self,
            &identity.identity_ref,
            identity.content_key_id,
            identity.contact_id,
            "import_participant_identity",
        )?;
        self.conn.execute("INSERT INTO import_participants(identity_id,namespace_id,contact_id,content_key_id,doc) VALUES(?1,?2,?3,?4,?5)",
            rusqlite::params![identity.identity_id.to_string(),identity.namespace_id.to_string(),identity.contact_id.to_string(),identity.content_key_id.to_string(),encode(identity)?]).map_err(backend)?;
        Ok(())
    }

    fn put_import_message(&mut self, identity: &ImportMessageIdentity) -> StoreResult<()> {
        validate_identity_ref(
            self,
            &identity.identity_ref,
            identity.content_key_id,
            identity.event_id,
            "import_message_identity",
        )?;
        self.conn.execute("INSERT INTO import_message_keys(identity_id,namespace_id,event_id,contact_id,content_key_id,doc) VALUES(?1,?2,?3,?4,?5,?6)",
            rusqlite::params![identity.identity_id.to_string(),identity.namespace_id.to_string(),identity.event_id.to_string(),identity.sender_contact_id.to_string(),identity.content_key_id.to_string(),encode(identity)?]).map_err(backend)?;
        self.conn
            .execute(
                "INSERT INTO import_indexed_events(event_id) VALUES(?1)",
                [identity.event_id.to_string()],
            )
            .map_err(backend)?;
        Ok(())
    }
}

fn validate_identity_ref(
    store: &SqlCipherStore,
    sealed: &SealedText,
    key: Uuid,
    row: Uuid,
    field: &str,
) -> StoreResult<()> {
    if sealed.content_key_id != key
        || sealed.aad.as_deref() != Some(field_aad(&row, field).as_str())
        || store.import_key_destroyed(key)?
    {
        return Err(fixed("IMPORT_IDENTITY_UNAVAILABLE"));
    }
    store
        .open(sealed)
        .map_err(|_| fixed("IMPORT_IDENTITY_UNAVAILABLE"))?;
    Ok(())
}

fn validate_account_ref(store: &SqlCipherStore, namespace: &ImportNamespace) -> StoreResult<()> {
    if let Some(sealed) = namespace.account_ref.as_ref() {
        let owner = namespace
            .owner_contact_id
            .ok_or_else(|| fixed("IMPORT_RECORD_INVALID"))?;
        if sealed.subject != SealedSubject::Owner
            || sealed.aad.as_deref() != Some(field_aad(&owner, "import_account_identity").as_str())
        {
            return Err(fixed("IMPORT_IDENTITY_UNAVAILABLE"));
        }
        // Revoked namespaces retain sealed pointers as tombstone metadata;
        // their destroyed content keys must never be recreated or required.
        if namespace.state == ImportNamespaceState::Active {
            validate_identity_ref(
                store,
                sealed,
                sealed.content_key_id,
                owner,
                "import_account_identity",
            )?;
        }
    }
    Ok(())
}

fn read_namespace(
    store: &SqlCipherStore,
    id: Uuid,
    source: &str,
    state: &str,
    owner: Option<&str>,
    doc: &str,
) -> StoreResult<ImportNamespace> {
    let mut namespace: ImportNamespace = decode(doc)?;
    if namespace.namespace_id != id
        || namespace.source.as_str() != source
        || namespace
            .owner_contact_id
            .map(|value| value.to_string())
            .as_deref()
            != owner
    {
        return Err(fixed("IMPORT_RECORD_INVALID"));
    }
    match state {
        "revoked" => namespace.state = ImportNamespaceState::Revoked,
        "active" if namespace.state == ImportNamespaceState::Active => {}
        _ => return Err(fixed("IMPORT_RECORD_INVALID")),
    }
    validate_account_ref(store, &namespace)?;
    Ok(namespace)
}

type IdentityColumns = (String, String, String, Option<String>);

#[cfg(test)]
mod local_source_tests {
    use super::*;
    use soul_schema::common::SealedSubject;
    use soul_store_api::import::ImportCoverage;
    use soul_store_api::types::SealRequest;

    fn fixture() -> (
        tempfile::TempDir,
        SqlCipherStore,
        ImportMessageIdentity,
        ImportNamespace,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("source.db"),
            &crate::TestKeyProvider::from_seed("local source fixture"),
        )
        .unwrap();
        let owner = Uuid::now_v7();
        let peer = Uuid::now_v7();
        let event = Uuid::now_v7();
        let key = Uuid::now_v7();
        let namespace = ImportNamespace {
            namespace_id: Uuid::now_v7(),
            source: ImportSourceKind::SoulImportV1,
            state: ImportNamespaceState::Active,
            coverage: ImportCoverage::FullFromCreation,
            owner_contact_id: Some(owner),
            account_ref: None,
        };
        store.put_import_namespace(&namespace).unwrap();
        let identity = ImportMessageIdentity {
            identity_id: Uuid::now_v7(),
            namespace_id: namespace.namespace_id,
            event_id: event,
            sender_contact_id: peer,
            content_key_id: key,
            identity_ref: store
                .seal(SealRequest::new(
                    key,
                    event,
                    "import_message_identity",
                    SealedSubject::ThirdParty,
                    br#"["conversation","message"]"#.to_vec(),
                ))
                .unwrap(),
            group: false,
            is_owner: false,
        };
        store.put_import_message(&identity).unwrap();
        (dir, store, identity, namespace)
    }

    #[test]
    fn local_source_lookup_is_exact_and_validates_columns_and_document() {
        let (_dir, store, identity, namespace) = fixture();
        let (actual, ns) = store
            .import_source_for_event(identity.event_id, 262_144)
            .unwrap()
            .unwrap();
        assert_eq!(actual, identity);
        assert_eq!(ns, namespace);
        assert!(store
            .import_source_for_event(Uuid::now_v7(), 262_144)
            .unwrap()
            .is_none());
        store
            .conn
            .execute(
                "UPDATE import_message_keys SET contact_id=?2 WHERE event_id=?1",
                rusqlite::params![identity.event_id.to_string(), Uuid::now_v7().to_string()],
            )
            .unwrap();
        assert!(store
            .import_source_for_event(identity.event_id, 262_144)
            .is_err());
    }

    #[test]
    fn local_source_identity_and_account_have_actual_byte_limits_before_open() {
        let (_dir, mut store, identity, mut namespace) = fixture();
        assert_eq!(
            store
                .import_source_for_event(identity.event_id, 1)
                .unwrap_err(),
            fixed("LOCAL_SOURCE_TOO_LARGE")
        );
        let account = store
            .seal(SealRequest::new(
                Uuid::now_v7(),
                namespace.owner_contact_id.unwrap(),
                "import_account_identity",
                SealedSubject::Owner,
                vec![b'x'; 1025],
            ))
            .unwrap();
        namespace.account_ref = Some(account);
        store.put_import_namespace(&namespace).unwrap();
        assert_eq!(
            store
                .import_source_for_event(identity.event_id, 1024)
                .unwrap_err(),
            fixed("LOCAL_SOURCE_TOO_LARGE")
        );
        // False char_count metadata cannot hide oversized ciphertext.
        store
            .conn
            .execute(
                "UPDATE sealed_blobs SET ciphertext=zeroblob(300000) WHERE blob_id=?1",
                [identity.identity_ref.blob_id.to_string()],
            )
            .unwrap();
        assert_eq!(
            store
                .import_source_for_event(identity.event_id, 262_144)
                .unwrap_err(),
            fixed("LOCAL_SOURCE_TOO_LARGE")
        );
    }

    #[test]
    fn local_source_revocation_is_rejected_before_any_identity_authentication() {
        let (_dir, store, identity, namespace) = fixture();
        store
            .conn
            .execute(
                "UPDATE sealed_blobs SET ciphertext=zeroblob(length(ciphertext)) WHERE blob_id=?1",
                [identity.identity_ref.blob_id.to_string()],
            )
            .unwrap();
        store
            .conn
            .execute(
                "UPDATE import_namespaces SET state='revoked' WHERE namespace_id=?1",
                [namespace.namespace_id.to_string()],
            )
            .unwrap();
        assert_eq!(
            store
                .import_source_for_event(identity.event_id, 262_144)
                .unwrap_err(),
            fixed("LOCAL_SOURCE_UNAVAILABLE")
        );
    }

    #[test]
    fn local_source_body_preflight_binds_row_field_key_subject_and_nonce() {
        let (_dir, mut store, identity, _) = fixture();
        let body = store
            .seal(SealRequest::new(
                identity.content_key_id,
                identity.event_id,
                "body_ref",
                SealedSubject::ThirdParty,
                "🦊".as_bytes().to_vec(),
            ))
            .unwrap();
        assert_eq!(
            store
                .event_body_plaintext_bytes(identity.event_id, &body)
                .unwrap(),
            4
        );
        assert!(store
            .event_body_plaintext_bytes(Uuid::now_v7(), &body)
            .is_err());
        let mut wrong = body.clone();
        wrong.subject = SealedSubject::Owner;
        assert!(store
            .event_body_plaintext_bytes(identity.event_id, &wrong)
            .is_err());
        wrong = body.clone();
        wrong.content_key_id = Uuid::now_v7();
        assert!(store
            .event_body_plaintext_bytes(identity.event_id, &wrong)
            .is_err());
        assert!(store
            .event_body_plaintext_bytes(identity.event_id, &identity.identity_ref)
            .is_err());
        store
            .conn
            .execute(
                "UPDATE sealed_blobs SET nonce=zeroblob(1000000) WHERE blob_id=?1",
                [body.blob_id.to_string()],
            )
            .unwrap();
        assert!(store
            .event_body_plaintext_bytes(identity.event_id, &body)
            .is_err());
    }

    #[test]
    fn local_source_identity_row_replay_and_destroyed_key_are_not_missing_records() {
        let (_dir, mut store, identity, _) = fixture();
        let other = store
            .seal(SealRequest::new(
                identity.content_key_id,
                Uuid::now_v7(),
                "import_message_identity",
                SealedSubject::ThirdParty,
                b"private synthetic identifier".to_vec(),
            ))
            .unwrap();
        let mut replay = identity.clone();
        replay.identity_ref = other;
        store
            .conn
            .execute(
                "UPDATE import_message_keys SET doc=?2 WHERE event_id=?1",
                rusqlite::params![identity.event_id.to_string(), encode(&replay).unwrap()],
            )
            .unwrap();
        let error = store
            .import_source_for_event(identity.event_id, 262_144)
            .unwrap_err();
        assert!(!format!("{error:?}").contains("private synthetic identifier"));
        store
            .conn
            .execute(
                "UPDATE import_message_keys SET doc=?2 WHERE event_id=?1",
                rusqlite::params![identity.event_id.to_string(), encode(&identity).unwrap()],
            )
            .unwrap();
        store
            .conn
            .execute(
                "DELETE FROM content_keys WHERE content_key_id=?1",
                [identity.content_key_id.to_string()],
            )
            .unwrap();
        assert!(store
            .import_source_for_event(identity.event_id, 262_144)
            .is_err());
    }
}

fn identity_batch<T: DeserializeOwned, F>(
    store: &SqlCipherStore,
    table: &str,
    namespace: Uuid,
    after: Option<Uuid>,
    limit: usize,
    validate: F,
) -> StoreResult<Vec<T>>
where
    F: Fn(&T, &IdentityColumns) -> StoreResult<()>,
{
    let event_column = if table == "import_message_keys" {
        "event_id"
    } else {
        "NULL"
    };
    let mut statement=store.conn.prepare(&format!("SELECT identity_id,contact_id,content_key_id,{event_column},doc FROM {table} WHERE namespace_id=?1 AND identity_id>?2 ORDER BY identity_id LIMIT ?3")).map_err(backend)?;
    let rows = statement
        .query_map(
            rusqlite::params![
                namespace.to_string(),
                after.map(|id| id.to_string()).unwrap_or_default(),
                limit.clamp(1, 256) as i64
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .map_err(backend)?;
    rows.map(|row| {
        let (id, contact, key, event, doc) = row.map_err(backend)?;
        let identity = decode(&doc)?;
        validate(&identity, &(id, contact, key, event))?;
        Ok(identity)
    })
    .collect()
}

#[cfg(test)]
mod identity_binding_tests {
    use super::*;
    use crate::TestKeyProvider;
    use soul_store_api::types::SealRequest;

    fn namespace() -> ImportNamespace {
        ImportNamespace {
            namespace_id: Uuid::now_v7(),
            source: ImportSourceKind::SoulImportV1,
            state: ImportNamespaceState::Active,
            coverage: soul_store_api::import::ImportCoverage::FullFromCreation,
            owner_contact_id: None,
            account_ref: None,
        }
    }

    #[test]
    fn valid_seals_from_another_row_cannot_be_replayed_as_participant_or_message_identities() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("identity.db"),
            &crate::TestKeyProvider::from_seed("identity aad"),
        )
        .unwrap();
        let ns = namespace();
        store.put_import_namespace(&ns).unwrap();
        let key = Uuid::now_v7();
        let contact = Uuid::now_v7();
        let foreign_contact = Uuid::now_v7();
        let own = store
            .seal(SealRequest::new(
                key,
                contact,
                "import_participant_identity",
                SealedSubject::Owner,
                b"[\"uid\",\"self\"]".to_vec(),
            ))
            .unwrap();
        let foreign = store
            .seal(SealRequest::new(
                key,
                foreign_contact,
                "import_participant_identity",
                SealedSubject::Owner,
                b"[\"uid\",\"self\"]".to_vec(),
            ))
            .unwrap();
        assert!(store.open(&foreign).is_ok());
        let participant = ImportParticipantIdentity {
            identity_id: Uuid::now_v7(),
            namespace_id: ns.namespace_id,
            contact_id: contact,
            content_key_id: key,
            identity_ref: own,
        };
        let mut replay = participant.clone();
        replay.identity_ref = foreign;
        assert!(store.put_import_participant(&replay).is_err());
        store.put_import_participant(&participant).unwrap();
        store
            .conn
            .execute(
                "UPDATE import_participants SET doc=?2 WHERE identity_id=?1",
                rusqlite::params![
                    participant.identity_id.to_string(),
                    encode(&replay).unwrap()
                ],
            )
            .unwrap();
        assert!(store
            .import_participant_batch(ns.namespace_id, None, 256)
            .is_err());

        let event = Uuid::now_v7();
        let foreign_event = Uuid::now_v7();
        let own = store
            .seal(SealRequest::new(
                key,
                event,
                "import_message_identity",
                SealedSubject::Owner,
                b"[\"chat\",\"message\"]".to_vec(),
            ))
            .unwrap();
        let foreign = store
            .seal(SealRequest::new(
                key,
                foreign_event,
                "import_message_identity",
                SealedSubject::Owner,
                b"[\"chat\",\"message\"]".to_vec(),
            ))
            .unwrap();
        let message = ImportMessageIdentity {
            identity_id: Uuid::now_v7(),
            namespace_id: ns.namespace_id,
            event_id: event,
            sender_contact_id: contact,
            content_key_id: key,
            identity_ref: own,
            group: false,
            is_owner: true,
        };
        let mut replay = message.clone();
        replay.identity_ref = foreign;
        assert!(store.put_import_message(&replay).is_err());
        store.put_import_message(&message).unwrap();
        store
            .conn
            .execute(
                "UPDATE import_message_keys SET doc=?2 WHERE identity_id=?1",
                rusqlite::params![message.identity_id.to_string(), encode(&replay).unwrap()],
            )
            .unwrap();
        assert!(store
            .import_message_batch(ns.namespace_id, None, 256)
            .is_err());
    }

    #[test]
    fn account_identity_is_bound_to_its_owner_on_write_and_read_and_sql_revocation_wins() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SqlCipherStore::open(
            dir.path().join("account.db"),
            &TestKeyProvider::from_seed("account aad"),
        )
        .unwrap();
        let mut ns = namespace();
        let key = Uuid::now_v7();
        let owner = Uuid::now_v7();
        ns.owner_contact_id = Some(owner);
        ns.account_ref = Some(
            store
                .seal(SealRequest::new(
                    key,
                    owner,
                    "import_account_identity",
                    SealedSubject::Owner,
                    b"[\"uid\",\"self\"]".to_vec(),
                ))
                .unwrap(),
        );
        let mut replay = ns.clone();
        replay.account_ref = Some(
            store
                .seal(SealRequest::new(
                    key,
                    Uuid::now_v7(),
                    "import_account_identity",
                    SealedSubject::Owner,
                    b"[\"uid\",\"self\"]".to_vec(),
                ))
                .unwrap(),
        );
        assert!(store.put_import_namespace(&replay).is_err());
        store.put_import_namespace(&ns).unwrap();
        store
            .conn
            .execute(
                "UPDATE import_namespaces SET doc=?2 WHERE namespace_id=?1",
                rusqlite::params![ns.namespace_id.to_string(), encode(&replay).unwrap()],
            )
            .unwrap();
        assert!(store.import_namespace(ns.namespace_id).is_err());
        store
            .conn
            .execute(
                "UPDATE import_namespaces SET state='revoked',doc=?2 WHERE namespace_id=?1",
                rusqlite::params![ns.namespace_id.to_string(), encode(&ns).unwrap()],
            )
            .unwrap();
        assert_eq!(
            store.import_namespace(ns.namespace_id).unwrap().state,
            ImportNamespaceState::Revoked
        );
        assert!(store.put_import_namespace(&ns).is_err());
    }
}
