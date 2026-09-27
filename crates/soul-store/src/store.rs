//! The encrypted main database.
//!
//! Two layers of encryption, as `docs/SECURITY.md` requires, and neither is
//! optional:
//!
//! * SQLCipher encrypts every page of the file under the database DEK, which is
//!   what stops someone who copies `soul.db` off the machine;
//! * each piece of prose is additionally sealed with XChaCha20-Poly1305 under a
//!   per-forget-unit content key, with `row_id|field` as additional
//!   authenticated data, which is what gives forgetting something concrete to
//!   destroy and stops a blob being replayed into another row or column.

use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use uuid::Uuid;

use soul_schema::audit::SoulAuditEntry;
use soul_schema::common::{field_aad, SealAlg, SealedText};
use soul_schema::contact::SoulContact;
use soul_schema::event::SoulEvent;
use soul_schema::evidence::SoulEvidence;
use soul_schema::inference::SoulInference;
use soul_schema::memory::SoulMemory;
use soul_schema::profile::SoulProfile;
use soul_schema::relationship::SoulRelationship;
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;

use soul_store_api::types::{EventFilter, InferenceState, SealRequest, StoreError, StoreResult};
use soul_store_api::{AuditLog, BlobStore, EventStore, GraphStore, MemoryStore, ProfileStore};

use crate::audit::{self, AuditLink, GENESIS_PREV_HASH};
use crate::keys::{KeyProvider, SecretKey};
use crate::sql;

/// AEAD context for a wrapped content key, so a wrapped key cannot be moved to
/// another `content_key_id` and still unwrap.
fn content_key_aad(content_key_id: Uuid) -> String {
    format!("content-key|{content_key_id}")
}

pub(crate) fn backend<E: std::fmt::Display>(error: E) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// Serialize a contract enum to the string the schema uses for it.
pub(crate) fn enum_text<T: serde::Serialize>(value: &T) -> StoreResult<String> {
    match serde_json::to_value(value).map_err(backend)? {
        Value::String(text) => Ok(text),
        other => Err(StoreError::Backend(format!(
            "expected a string-valued enum, got {other}"
        ))),
    }
}

fn to_doc<T: serde::Serialize>(model: &T) -> StoreResult<String> {
    serde_json::to_string(model).map_err(backend)
}

fn from_doc<T: serde::de::DeserializeOwned>(doc: &str) -> StoreResult<T> {
    serde_json::from_str(doc).map_err(backend)
}

/// `?,?,?` for an `IN` list of `count` bound parameters.
pub(crate) fn placeholders(count: usize) -> String {
    let mut out = String::with_capacity(count * 2);
    for index in 0..count {
        if index > 0 {
            out.push(',');
        }
        out.push('?');
    }
    out
}

/// The `WHERE` clause an [`EventFilter`] selects, and the values to bind to it.
///
/// Shared by the listing and the count so the two can never drift into
/// disagreeing about which rows the same filter means.
fn event_where(filter: &EventFilter) -> StoreResult<(String, Vec<String>)> {
    let mut clauses: Vec<&str> = Vec::new();
    let mut bound: Vec<String> = Vec::new();

    if let Some(source) = filter.source.as_ref() {
        clauses.push("source = ?");
        bound.push(enum_text(source)?);
    }
    if let Some(kind) = filter.kind.as_ref() {
        clauses.push("kind = ?");
        bound.push(enum_text(kind)?);
    }
    if let Some(since) = filter.since.as_ref() {
        clauses.push("ts >= ?");
        bound.push(since.clone());
    }

    let mut sql = String::new();
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    Ok((sql, bound))
}

pub(crate) fn as_text(ids: &[Uuid]) -> Vec<String> {
    ids.iter().map(Uuid::to_string).collect()
}

/// What `meta.schema_version` says about a file this build is about to open.
enum StampedVersion {
    /// No `meta` table, or no `schema_version` row in it: a fresh file, or one
    /// written before the stamp existed.
    Unstamped,
    /// A version this build can read, at or below [`sql::STORE_SCHEMA_VERSION`].
    Readable,
    /// Written by a build that knew a schema this one does not, or stamped
    /// with something that is not a version at all.
    Unreadable(String),
}

/// Read `meta.schema_version` without writing anything.
///
/// `docs/DECISIONS.md` D62 gives this build two moves and no others. Forward
/// is additive: every statement in [`sql::DDL`] is `IF NOT EXISTS`, so a
/// version 1 file gains `destroyed_content_keys` and is re-stamped. Backward
/// is not a move at all — this build cannot know what a newer schema promises,
/// and the damage was never the read that fails afterwards but the stamp, which
/// used to be upserted to [`sql::STORE_SCHEMA_VERSION`] unconditionally and so
/// relabelled a newer file as one this build had written. Every later launch
/// then believed the label.
///
/// So this runs before the pragmas, before the DDL and before the upsert, and
/// a [`StampedVersion::Unreadable`] answer has to leave the file exactly as it
/// was found. Recreating it is not on the table either: the forget ledger and
/// the hash-chained audit are not things that can be built again.
fn stamped_version(conn: &Connection, path: &Path) -> StoreResult<StampedVersion> {
    let read_failed = |what: &str, error: rusqlite::Error| {
        StoreError::Backend(format!(
            "the database at {} opened under this key but {what} could not be read: {error}",
            path.display()
        ))
    };

    let meta_tables: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = 'meta'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| read_failed("its table list", error))?;
    if meta_tables == 0 {
        return Ok(StampedVersion::Unstamped);
    }

    // `CAST(... AS TEXT)` so a stamp somebody stored as an integer, or as a
    // float, is something this build reads and judges rather than something it
    // fails to fetch.
    let stamp: Option<Option<String>> = conn
        .query_row(
            "SELECT CAST(value AS TEXT) FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| read_failed("its schema version", error))?;

    Ok(match stamp {
        None => StampedVersion::Unstamped,
        Some(None) => StampedVersion::Unreadable("NULL".into()),
        Some(Some(text)) => match text.trim().parse::<i64>() {
            Ok(version) if version <= sql::STORE_SCHEMA_VERSION => StampedVersion::Readable,
            Ok(version) => StampedVersion::Unreadable(version.to_string()),
            Err(_) => StampedVersion::Unreadable(format!("{text:?}")),
        },
    })
}

/// SQLCipher-backed [`soul_store_api::SoulStore`].
#[derive(Debug)]
pub struct SqlCipherStore {
    pub(crate) conn: Connection,
    pub(crate) schemas: SchemaSet,
    pub(crate) kek: SecretKey,
    path: PathBuf,
    key_provider_label: String,
}

impl SqlCipherStore {
    /// Open, or create, the encrypted database at `path`.
    ///
    /// Fails rather than falling back if the linked SQLite is not a SQLCipher
    /// build: a store that silently writes unencrypted pages would satisfy
    /// every functional test in this crate and none of its promises.
    pub fn open(path: impl AsRef<Path>, keys: &dyn KeyProvider) -> StoreResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(backend)?;
            }
        }

        let dek = keys.database_key().map_err(backend)?;
        let kek = keys.key_encryption_key().map_err(backend)?;

        let conn = Connection::open(&path).map_err(backend)?;
        // The raw-key form. A passphrase would be run through key derivation
        // by SQLCipher; the DEK is already 32 bytes of key material.
        conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", dek.to_hex()))
            .map_err(backend)?;

        // A plain SQLite build answers this pragma with no rows at all.
        let cipher_version: Option<String> = conn
            .query_row("PRAGMA cipher_version", [], |row| row.get(0))
            .optional()
            .map_err(backend)?;
        if cipher_version.is_none_or(|version| version.trim().is_empty()) {
            return Err(StoreError::Backend(
                "this SQLite build reports no cipher_version, so it is not SQLCipher and the \
                 database would be written in the clear"
                    .into(),
            ));
        }

        // The first read is what actually verifies the key.
        conn.query_row("SELECT count(*) FROM sqlite_schema", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| {
            StoreError::Backend(format!(
                "the database at {} did not open under this key: {error}",
                path.display()
            ))
        })?;

        // Everything below this line writes to the file, so the question of
        // whether this build may write to it at all is settled here.
        if let StampedVersion::Unreadable(found) = stamped_version(&conn, &path)? {
            return Err(StoreError::Backend(format!(
                "the database at {} is stamped meta.schema_version = {found}, and this build \
                 understands {}: it was written by a newer Soul, or by something that is not \
                 this one. Nothing has been written to it — not a pragma, not a table, and not \
                 the stamp — because an older build cannot know what a newer schema promises, \
                 and stamping the file down to {} would hide where it came from, from every \
                 launch after this one. Install the newer Soul, or move this database aside.",
                path.display(),
                sql::STORE_SCHEMA_VERSION,
                sql::STORE_SCHEMA_VERSION,
            )));
        }

        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(backend)?;
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(backend)?;
        // Forgetting is a `DELETE` of a wrapped content key, and plain SQLite
        // would leave those bytes where they were, on a page that has merely
        // joined the free list — still inside a file the DEK opens. SQLCipher
        // happens to turn secure deletion on for us when its codec attaches,
        // which is a fact about a dependency's internals and not something
        // this crate says anywhere. Asking for it here, and refusing to hand
        // back a store that answers anything else, makes it ours to keep.
        conn.pragma_update(None, "secure_delete", "ON")
            .map_err(backend)?;
        let secure_delete: i64 = conn
            .query_row("PRAGMA secure_delete", [], |row| row.get(0))
            .map_err(backend)?;
        if secure_delete != 1 {
            return Err(StoreError::Backend(format!(
                "this SQLite build answered PRAGMA secure_delete with {secure_delete} after being \
                 asked for 1, so a destroyed content key would stay legible in a free page"
            )));
        }
        conn.execute_batch(sql::DDL).map_err(backend)?;
        // Only ever reached for a file at or below this version, so this moves
        // the stamp forward — 1 to 2 once the additive DDL above has given the
        // file what version 2 means — and never back down.
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [sql::STORE_SCHEMA_VERSION.to_string()],
        )
        .map_err(backend)?;

        Ok(SqlCipherStore {
            conn,
            schemas: SchemaSet::load()
                .map_err(|error| StoreError::Backend(format!("frozen contracts: {error}")))?,
            kek,
            path,
            key_provider_label: keys.describe(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Which provider supplied the root secrets. Never key material.
    pub fn key_provider_label(&self) -> &str {
        &self.key_provider_label
    }

    /// Close the connection explicitly, reporting a failure instead of
    /// swallowing it the way `Drop` has to.
    pub fn close(self) -> StoreResult<()> {
        self.conn.close().map_err(|(_, error)| backend(error))
    }

    /// Reject anything that would not validate against its own contract, so a
    /// backend bug shows up at the write rather than at the next read.
    pub(crate) fn check<T: serde::Serialize>(&self, id: SchemaId, model: &T) -> StoreResult<()> {
        self.schemas
            .validate_model(id, model)
            .map(|_| ())
            .map_err(|failure| StoreError::ContractViolation(failure.to_string()))
    }

    fn kek_cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(Key::from_slice(self.kek.expose()))
    }

    /// The content key behind `content_key_id`, or `None` once it is destroyed.
    pub(crate) fn content_key(&self, content_key_id: Uuid) -> StoreResult<Option<SecretKey>> {
        let row: Option<(Vec<u8>, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT wrapped_key, wrap_nonce FROM content_keys WHERE content_key_id = ?1",
                [content_key_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(backend)?;

        let Some((wrapped, nonce)) = row else {
            return Ok(None);
        };
        if nonce.len() != 24 {
            return Err(StoreError::Backend(format!(
                "content key {content_key_id} has a {}-byte wrapping nonce, expected 24",
                nonce.len()
            )));
        }
        let aad = content_key_aad(content_key_id);
        let raw = self
            .kek_cipher()
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &wrapped,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| {
                StoreError::Backend(format!(
                    "content key {content_key_id} did not unwrap under this KEK"
                ))
            })?;
        if raw.len() != crate::keys::KEY_LEN {
            return Err(StoreError::Backend(format!(
                "content key {content_key_id} unwrapped to {} bytes",
                raw.len()
            )));
        }
        let mut bytes = [0u8; crate::keys::KEY_LEN];
        bytes.copy_from_slice(&raw);
        Ok(Some(SecretKey::from_bytes(bytes)))
    }

    /// Whether this id has already been through a forget.
    ///
    /// The wrapped key is gone by then, and nothing else in the database says
    /// the id ever existed, so a caller asking to seal under it again would be
    /// handed a brand new key — and the tombstoned rows that name the id would
    /// have a live key behind them once more.
    pub fn content_key_destroyed(&self, content_key_id: Uuid) -> StoreResult<bool> {
        self.conn
            .query_row(
                "SELECT 1 FROM destroyed_content_keys WHERE content_key_id = ?1",
                [content_key_id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|row| row.is_some())
            .map_err(backend)
    }

    fn ensure_content_key(&mut self, content_key_id: Uuid) -> StoreResult<SecretKey> {
        if let Some(existing) = self.content_key(content_key_id)? {
            return Ok(existing);
        }
        if self.content_key_destroyed(content_key_id)? {
            return Err(StoreError::ContentKeyDestroyed(content_key_id));
        }
        let fresh = SecretKey::random();
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let aad = content_key_aad(content_key_id);
        let wrapped = self
            .kek_cipher()
            .encrypt(
                &nonce,
                Payload {
                    msg: fresh.expose(),
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| StoreError::Backend("wrapping a content key failed".into()))?;
        self.conn
            .execute(
                "INSERT INTO content_keys (content_key_id, wrapped_key, wrap_nonce)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![content_key_id.to_string(), wrapped, nonce.to_vec()],
            )
            .map_err(backend)?;
        Ok(fresh)
    }

    /// How many content keys are still held. Zero of these means nothing sealed
    /// under them can ever be read again.
    pub fn content_key_count(&self) -> StoreResult<u64> {
        self.conn
            .query_row("SELECT count(*) FROM content_keys", [], |row| row.get(0))
            .map_err(backend)
    }

    pub fn blob_count(&self) -> StoreResult<u64> {
        self.conn
            .query_row("SELECT count(*) FROM sealed_blobs", [], |row| row.get(0))
            .map_err(backend)
    }

    /// The audit chain as stored, oldest first.
    pub fn audit_links(&self) -> StoreResult<Vec<AuditLink>> {
        let mut statement = self
            .conn
            .prepare("SELECT seq, prev_hash, entry_hash, doc FROM audit ORDER BY seq")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(backend)?;

        let mut links = Vec::new();
        for row in rows {
            let (seq, prev_hash, entry_hash, doc) = row.map_err(backend)?;
            links.push(AuditLink {
                seq: seq as u64,
                prev_hash,
                entry_hash,
                entry: from_doc(&doc)?,
            });
        }
        Ok(links)
    }

    /// Walk the chain and report the first place it breaks.
    pub fn verify_audit_chain(&self) -> StoreResult<()> {
        audit::verify(&self.audit_links()?)
            .map_err(|error| StoreError::ContractViolation(error.to_string()))
    }

    fn audit_tip(&self) -> StoreResult<(u64, String)> {
        let row: Option<(i64, String)> = self
            .conn
            .query_row(
                "SELECT seq, entry_hash FROM audit ORDER BY seq DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(backend)?;
        Ok(match row {
            Some((seq, hash)) => (seq as u64 + 1, hash),
            None => (0, GENESIS_PREV_HASH.to_owned()),
        })
    }

    /// Fold the write-ahead log back into the main database file.
    pub fn checkpoint(&self) -> StoreResult<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|_| ())
            .map_err(backend)
    }

    /// `PRAGMA secure_delete` as SQLite reports it: 0 off, 1 on, 2 for the
    /// `FAST` compromise that only zeroes what it can do without extra page
    /// writes. [`SqlCipherStore::open`] refuses to hand back a store that
    /// answers anything but 1, because forgetting rests on the freed bytes
    /// being gone rather than merely unlinked.
    pub fn secure_delete(&self) -> StoreResult<i64> {
        self.conn
            .query_row("PRAGMA secure_delete", [], |row| row.get(0))
            .map_err(backend)
    }

    /// `PRAGMA integrity_check`, for tests that reopen after a crash.
    pub fn integrity_check(&self) -> StoreResult<String> {
        self.conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(backend)
    }

    /// Run `work` with every write it makes inside one SQLite transaction.
    ///
    /// This is the entry point `soul-store-api` deliberately does not have, and
    /// it exists for one shape of caller: a command that writes many rows which
    /// only mean something together. An import is the case that forced it. Each
    /// write below used to commit on its own, so a `synchronous=FULL` commit
    /// fsync was paid once per message — seconds for an eight-thousand message
    /// export on an ordinary filesystem, and linear in the export from there —
    /// and a crash halfway through left an import that could not be re-run
    /// without writing every surviving event a second time.
    ///
    /// Both problems have the same answer. Inside `work` the per-row writes
    /// nest as savepoints, which are bookkeeping in the same open transaction
    /// rather than durability points of their own, and the single commit at the
    /// end is the only fsync. A `work` that returns an error, panics, or is cut
    /// off by a power loss leaves nothing: the transaction is rolled back, or —
    /// when the process never got that far — was never committed to begin with.
    ///
    /// `BEGIN IMMEDIATE` rather than the deferred default: the write lock is
    /// taken up front, so a transaction that is going to be refused is refused
    /// before `work` has sealed anything.
    ///
    /// Not reentrant, and it says so by failing rather than by silently joining
    /// the transaction already open. Nesting these would make the inner one's
    /// commit look durable while the outer one could still roll it away, which
    /// is exactly the confusion this method exists to remove.
    pub fn transact<T, E, F>(&mut self, work: F) -> Result<T, E>
    where
        F: FnOnce(&mut SqlCipherStore) -> Result<T, E>,
        E: From<StoreError>,
    {
        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|error| E::from(backend(error)))?;

        // A panic inside `work` would otherwise unwind past the rollback and
        // leave the connection mid-transaction. `soulcore` recovers a poisoned
        // store mutex rather than propagating it, so the next command would
        // find a connection whose next write silently joined a transaction
        // nobody is going to commit.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(self)));

        match outcome {
            Ok(Ok(value)) => match self.conn.execute_batch("COMMIT") {
                Ok(()) => Ok(value),
                Err(error) => {
                    self.roll_back_quietly();
                    Err(E::from(backend(error)))
                }
            },
            Ok(Err(refused)) => {
                self.roll_back_quietly();
                Err(refused)
            }
            Err(panicked) => {
                self.roll_back_quietly();
                std::panic::resume_unwind(panicked)
            }
        }
    }

    /// Undo the open transaction, keeping whatever went wrong first.
    ///
    /// A `ROLLBACK` that itself fails means SQLite has already ended the
    /// transaction — the usual cause is a statement that rolled it back — and
    /// reporting that instead of the original error would name the symptom.
    fn roll_back_quietly(&self) {
        let _ = self.conn.execute_batch("ROLLBACK");
    }
}

impl EventStore for SqlCipherStore {
    fn append_event(&mut self, event: SoulEvent) -> StoreResult<Uuid> {
        self.check(SchemaId::Event, &event)?;
        let id = event.event_id;

        let exists: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM events WHERE event_id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        if exists.is_some() {
            return Err(StoreError::AlreadyExists { kind: "event", id });
        }

        let row = (
            id.to_string(),
            event.ts.as_str().to_owned(),
            enum_text(&event.source)?,
            enum_text(&event.kind)?,
            enum_text(&event.actor_subject)?,
            enum_text(&event.privacy.subject)?,
            event.body_ref.as_ref().map(|s| s.blob_id.to_string()),
            to_doc(&event)?,
        );

        // A savepoint rather than a transaction, so that the same write is
        // correct on its own and inside a [`SqlCipherStore::transact`]. On its
        // own the outermost savepoint *is* the transaction — SQLite opens one
        // for it and the release below commits it, fsync included, which is
        // what AC-24's crash tests interrupt. Under an import's wrap it is
        // bookkeeping in a transaction that has not committed yet, and a crash
        // here loses the whole import rather than every message after this one.
        let tx = self.conn.savepoint().map_err(backend)?;
        tx.execute(
            "INSERT INTO events
                (event_id, ts, source, kind, actor_subject, privacy_subject, body_blob_id, doc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7],
        )
        .map_err(backend)?;

        // AC-24 injects here: the row is written but the transaction has not
        // been committed, which is the state a power loss would leave behind.
        fail::fail_point!(crate::failpoints::STORE_EVENT_COMMIT_MID);

        tx.commit().map_err(backend)?;
        Ok(id)
    }

    fn get_event(&self, event_id: Uuid) -> StoreResult<SoulEvent> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM events WHERE event_id = ?1",
                [event_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("event", event_id)),
        }
    }

    fn list_events(&self, filter: &EventFilter) -> StoreResult<Vec<SoulEvent>> {
        let (where_clause, bound) = event_where(filter)?;
        let mut sql = format!("SELECT doc FROM events{where_clause} ORDER BY rowid");
        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }

        let mut statement = self.conn.prepare(&sql).map_err(backend)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(bound.iter()), |row| {
                row.get::<_, String>(0)
            })
            .map_err(backend)?;

        let mut events = Vec::new();
        for row in rows {
            events.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(events)
    }

    /// `count(*)`, rather than the trait default that reads and JSON-parses
    /// every matching row only to ask for the length of the vector. The collect
    /// screen asks for this once a second with the store lock held, so the
    /// default turns an idle-looking status poll into a full table scan.
    fn count_events(&self, filter: &EventFilter) -> StoreResult<u64> {
        let (where_clause, bound) = event_where(filter)?;
        let sql = format!("SELECT count(*) FROM events{where_clause}");
        let matched: i64 = self
            .conn
            .query_row(&sql, rusqlite::params_from_iter(bound.iter()), |row| {
                row.get(0)
            })
            .map_err(backend)?;
        let matched = u64::try_from(matched).unwrap_or(0);

        // `list_events` stops at `limit`, so counting the same filter must too.
        Ok(match filter.limit {
            Some(limit) => matched.min(limit as u64),
            None => matched,
        })
    }
}

impl ProfileStore for SqlCipherStore {
    fn put_profile(&mut self, profile: SoulProfile) -> StoreResult<Uuid> {
        self.check(SchemaId::Profile, &profile)?;
        let id = profile.profile_id;
        self.conn
            .execute(
                "INSERT INTO profiles (profile_id, doc) VALUES (?1, ?2)
                 ON CONFLICT (profile_id) DO UPDATE SET doc = excluded.doc",
                rusqlite::params![id.to_string(), to_doc(&profile)?],
            )
            .map_err(backend)?;
        Ok(id)
    }

    fn get_profile(&self, profile_id: Uuid) -> StoreResult<SoulProfile> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM profiles WHERE profile_id = ?1",
                [profile_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("profile", profile_id)),
        }
    }

    fn put_evidence(&mut self, evidence: SoulEvidence) -> StoreResult<Uuid> {
        self.check(SchemaId::Evidence, &evidence)?;
        let id = evidence.evidence_id;
        self.conn
            .execute(
                "INSERT INTO evidence (evidence_id, subject, doc) VALUES (?1, ?2, ?3)
                 ON CONFLICT (evidence_id) DO UPDATE
                 SET subject = excluded.subject, doc = excluded.doc",
                rusqlite::params![
                    id.to_string(),
                    enum_text(&evidence.subject)?,
                    to_doc(&evidence)?
                ],
            )
            .map_err(backend)?;
        Ok(id)
    }

    fn get_evidence(&self, evidence_id: Uuid) -> StoreResult<SoulEvidence> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM evidence WHERE evidence_id = ?1",
                [evidence_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("evidence", evidence_id)),
        }
    }

    fn list_evidence(&self) -> StoreResult<Vec<SoulEvidence>> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM evidence ORDER BY evidence_id")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(out)
    }

    fn put_inference(&mut self, inference: SoulInference) -> StoreResult<Uuid> {
        self.check(SchemaId::Inference, &inference)?;
        if inference.evidence_ids.is_empty() {
            return Err(StoreError::ContractViolation(
                "an inference with no evidence must not be stored".into(),
            ));
        }
        for evidence_id in &inference.evidence_ids {
            let resolves: Option<i64> = self
                .conn
                .query_row(
                    "SELECT 1 FROM evidence WHERE evidence_id = ?1",
                    [evidence_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(backend)?;
            if resolves.is_none() {
                return Err(StoreError::ContractViolation(format!(
                    "inference {} cites evidence {evidence_id}, which does not resolve",
                    inference.inference_id
                )));
            }
        }

        let id = inference.inference_id;
        let doc = to_doc(&inference)?;
        let live = enum_text(&InferenceState::Live)?;
        let tx = self.conn.savepoint().map_err(backend)?;
        tx.execute(
            "INSERT INTO inferences (inference_id, state, doc) VALUES (?1, ?2, ?3)
             ON CONFLICT (inference_id) DO UPDATE
             SET state = excluded.state, doc = excluded.doc",
            rusqlite::params![id.to_string(), live, doc],
        )
        .map_err(backend)?;
        tx.execute(
            "DELETE FROM inference_evidence WHERE inference_id = ?1",
            [id.to_string()],
        )
        .map_err(backend)?;
        for evidence_id in &inference.evidence_ids {
            tx.execute(
                "INSERT OR IGNORE INTO inference_evidence (inference_id, evidence_id)
                 VALUES (?1, ?2)",
                rusqlite::params![id.to_string(), evidence_id.to_string()],
            )
            .map_err(backend)?;
        }
        tx.commit().map_err(backend)?;
        Ok(id)
    }

    fn get_inference(&self, inference_id: Uuid) -> StoreResult<SoulInference> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM inferences WHERE inference_id = ?1",
                [inference_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("inference", inference_id)),
        }
    }

    fn list_inferences(&self) -> StoreResult<Vec<SoulInference>> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM inferences ORDER BY inference_id")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(out)
    }

    fn inference_state(&self, inference_id: Uuid) -> StoreResult<InferenceState> {
        let state: Option<String> = self
            .conn
            .query_row(
                "SELECT state FROM inferences WHERE inference_id = ?1",
                [inference_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match state {
            Some(state) => serde_json::from_value(Value::String(state.clone())).map_err(|_| {
                StoreError::Backend(format!("unknown inference state {state} in the database"))
            }),
            None => Err(StoreError::not_found("inference", inference_id)),
        }
    }
}

impl MemoryStore for SqlCipherStore {
    fn put_memory(&mut self, memory: SoulMemory) -> StoreResult<Uuid> {
        self.check(SchemaId::Memory, &memory)?;
        let id = memory.memory_id;

        // Every key that has to die when this memory is forgotten.
        let mut key_ids = vec![memory.content_key_id];
        for sealed in [&memory.title_ref, &memory.summary_ref]
            .into_iter()
            .flatten()
        {
            key_ids.push(sealed.content_key_id);
        }
        key_ids.sort();
        key_ids.dedup();

        let evidence_ids = memory.evidence_ids.clone().unwrap_or_default();
        let row = (
            id.to_string(),
            memory.content_key_id.to_string(),
            enum_text(&memory.forget_state)?,
            to_doc(&memory)?,
        );

        let tx = self.conn.savepoint().map_err(backend)?;
        tx.execute(
            "INSERT INTO memories (memory_id, content_key_id, forget_state, doc)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (memory_id) DO UPDATE SET
                 content_key_id = excluded.content_key_id,
                 forget_state   = excluded.forget_state,
                 doc            = excluded.doc",
            rusqlite::params![row.0, row.1, row.2, row.3],
        )
        .map_err(backend)?;
        tx.execute(
            "DELETE FROM memory_content_keys WHERE memory_id = ?1",
            [id.to_string()],
        )
        .map_err(backend)?;
        for key_id in &key_ids {
            tx.execute(
                "INSERT OR IGNORE INTO memory_content_keys (memory_id, content_key_id)
                 VALUES (?1, ?2)",
                rusqlite::params![id.to_string(), key_id.to_string()],
            )
            .map_err(backend)?;
        }
        tx.execute(
            "DELETE FROM memory_evidence WHERE memory_id = ?1",
            [id.to_string()],
        )
        .map_err(backend)?;
        for evidence_id in &evidence_ids {
            tx.execute(
                "INSERT OR IGNORE INTO memory_evidence (memory_id, evidence_id) VALUES (?1, ?2)",
                rusqlite::params![id.to_string(), evidence_id.to_string()],
            )
            .map_err(backend)?;
        }
        tx.commit().map_err(backend)?;
        Ok(id)
    }

    fn get_memory(&self, memory_id: Uuid) -> StoreResult<SoulMemory> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM memories WHERE memory_id = ?1",
                [memory_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("memory", memory_id)),
        }
    }

    fn list_memories(&self) -> StoreResult<Vec<SoulMemory>> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM memories ORDER BY memory_id")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(out)
    }
}

impl GraphStore for SqlCipherStore {
    fn put_contact(&mut self, contact: SoulContact) -> StoreResult<Uuid> {
        self.check(SchemaId::Contact, &contact)?;
        let id = contact.contact_id;
        self.conn
            .execute(
                "INSERT INTO contacts
                    (contact_id, contact_class, forget_state, display_label_key_id, doc)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (contact_id) DO UPDATE SET
                     contact_class        = excluded.contact_class,
                     forget_state         = excluded.forget_state,
                     display_label_key_id = excluded.display_label_key_id,
                     doc                  = excluded.doc",
                rusqlite::params![
                    id.to_string(),
                    enum_text(&contact.contact_class)?,
                    enum_text(&contact.forget_state)?,
                    contact
                        .display_label_ref
                        .as_ref()
                        .map(|sealed| sealed.content_key_id.to_string()),
                    to_doc(&contact)?,
                ],
            )
            .map_err(backend)?;
        Ok(id)
    }

    fn get_contact(&self, contact_id: Uuid) -> StoreResult<SoulContact> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM contacts WHERE contact_id = ?1",
                [contact_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("contact", contact_id)),
        }
    }

    fn list_contacts(&self) -> StoreResult<Vec<SoulContact>> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM contacts ORDER BY contact_id")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(out)
    }

    fn put_relationship(&mut self, relationship: SoulRelationship) -> StoreResult<Uuid> {
        self.check(SchemaId::Relationship, &relationship)?;
        let id = relationship.relationship_id;
        let doc = to_doc(&relationship)?;

        let tx = self.conn.savepoint().map_err(backend)?;
        tx.execute(
            "INSERT INTO relationships (relationship_id, from_contact_id, to_contact_id, doc)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (relationship_id) DO UPDATE SET
                 from_contact_id = excluded.from_contact_id,
                 to_contact_id   = excluded.to_contact_id,
                 doc             = excluded.doc",
            rusqlite::params![
                id.to_string(),
                relationship.from_contact_id.to_string(),
                relationship.to_contact_id.to_string(),
                doc,
            ],
        )
        .map_err(backend)?;
        tx.execute(
            "DELETE FROM relationship_evidence WHERE relationship_id = ?1",
            [id.to_string()],
        )
        .map_err(backend)?;
        for evidence_id in &relationship.evidence_ids {
            tx.execute(
                "INSERT OR IGNORE INTO relationship_evidence (relationship_id, evidence_id)
                 VALUES (?1, ?2)",
                rusqlite::params![id.to_string(), evidence_id.to_string()],
            )
            .map_err(backend)?;
        }
        tx.commit().map_err(backend)?;
        Ok(id)
    }

    fn get_relationship(&self, relationship_id: Uuid) -> StoreResult<SoulRelationship> {
        let doc: Option<String> = self
            .conn
            .query_row(
                "SELECT doc FROM relationships WHERE relationship_id = ?1",
                [relationship_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(backend)?;
        match doc {
            Some(doc) => from_doc(&doc),
            None => Err(StoreError::not_found("relationship", relationship_id)),
        }
    }

    fn relationships_for(&self, contact_id: Uuid) -> StoreResult<Vec<SoulRelationship>> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT doc FROM relationships
                 WHERE from_contact_id = ?1 OR to_contact_id = ?1
                 ORDER BY relationship_id",
            )
            .map_err(backend)?;
        let rows = statement
            .query_map([contact_id.to_string()], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(out)
    }

    fn list_relationships(&self) -> StoreResult<Vec<SoulRelationship>> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM relationships ORDER BY relationship_id")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(from_doc(&row.map_err(backend)?)?);
        }
        Ok(out)
    }
}

impl BlobStore for SqlCipherStore {
    fn seal(&mut self, request: SealRequest) -> StoreResult<SealedText> {
        let key = self.ensure_content_key(request.content_key_id)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(key.expose()));
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let aad = field_aad(&request.row_id, &request.field);

        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &request.plaintext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| StoreError::Backend("sealing failed".into()))?;

        let blob_id = Uuid::now_v7();
        let subject = enum_text(&request.subject)?;
        self.conn
            .execute(
                "INSERT INTO sealed_blobs
                    (blob_id, content_key_id, row_id, field, subject, nonce, ciphertext)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    blob_id.to_string(),
                    request.content_key_id.to_string(),
                    request.row_id.to_string(),
                    request.field,
                    subject,
                    nonce.to_vec(),
                    ciphertext,
                ],
            )
            .map_err(backend)?;

        Ok(SealedText {
            content_key_id: request.content_key_id,
            blob_id,
            alg: SealAlg,
            aad: Some(aad),
            subject: request.subject,
            char_count: String::from_utf8_lossy(&request.plaintext).chars().count() as u64,
            placeholder: request.placeholder,
        })
    }

    fn open(&self, sealed: &SealedText) -> StoreResult<Vec<u8>> {
        let key = self
            .content_key(sealed.content_key_id)?
            .ok_or(StoreError::ContentKeyDestroyed(sealed.content_key_id))?;

        let blob: Option<(Vec<u8>, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT nonce, ciphertext FROM sealed_blobs WHERE blob_id = ?1",
                [sealed.blob_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(backend)?;
        let Some((nonce, ciphertext)) = blob else {
            return Err(StoreError::BlobMissing(sealed.blob_id));
        };
        if nonce.len() != 24 {
            return Err(StoreError::SealBroken(sealed.blob_id));
        }

        let aad = sealed.aad.as_deref().unwrap_or_default();
        XChaCha20Poly1305::new(Key::from_slice(key.expose()))
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| StoreError::SealBroken(sealed.blob_id))
    }

    fn has_content_key(&self, content_key_id: Uuid) -> bool {
        self.conn
            .query_row(
                "SELECT 1 FROM content_keys WHERE content_key_id = ?1",
                [content_key_id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|row| row.is_some())
            .unwrap_or(false)
    }
}

impl AuditLog for SqlCipherStore {
    fn append_audit(&mut self, entry: SoulAuditEntry) -> StoreResult<Uuid> {
        let (seq, prev_hash) = self.audit_tip()?;
        let linked = audit::link(entry, seq, &prev_hash).map_err(|message| {
            StoreError::Backend(format!("hashing the audit entry: {message}"))
        })?;
        self.check(SchemaId::Audit, &linked.entry)?;

        let id = linked.entry.entry_id;
        let subjects = linked.entry.subject_refs.clone().unwrap_or_default();
        let row = (
            linked.seq as i64,
            id.to_string(),
            linked.entry.ts.as_str().to_owned(),
            linked.prev_hash.clone(),
            linked.entry_hash.clone(),
            enum_text(&linked.entry.action)?,
            enum_text(&linked.entry.decision)?,
            to_doc(&linked.entry)?,
        );

        let tx = self.conn.savepoint().map_err(backend)?;
        tx.execute(
            "INSERT INTO audit (seq, entry_id, ts, prev_hash, entry_hash, action, decision, doc)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7],
        )
        .map_err(backend)?;
        for subject in &subjects {
            tx.execute(
                "INSERT OR IGNORE INTO audit_subjects (seq, subject_id) VALUES (?1, ?2)",
                rusqlite::params![row.0, subject.to_string()],
            )
            .map_err(backend)?;
        }

        // AC-24, audit side. The row and its subject links are written but the
        // transaction is open: a power loss here must lose the whole entry
        // rather than leave a link the next `prev_hash` cannot reach.
        fail::fail_point!(crate::failpoints::AUDIT_APPEND_PRE_COMMIT);

        tx.commit().map_err(backend)?;

        // The other side of the same question: the entry is durable but the
        // caller never learned its id. Reopening must find a chain that still
        // verifies, one entry longer than the caller believes.
        fail::fail_point!(crate::failpoints::AUDIT_APPEND_POST_WRITE);

        Ok(id)
    }

    fn list_audit(&self) -> StoreResult<Vec<SoulAuditEntry>> {
        Ok(self
            .audit_links()?
            .into_iter()
            .map(|link| link.entry)
            .collect())
    }
}
