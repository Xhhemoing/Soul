//! Table definitions for the encrypted main database.
//!
//! Two rules shape this schema.
//!
//! The first is that no column holds prose. Anything a person wrote lives in
//! `sealed_blobs` as ciphertext and is reachable only through the content key
//! that seals it. The `doc` columns hold the serialized contract model, which
//! by construction carries sealed pointers rather than text.
//!
//! The second is that anything the product has to *count* gets its own column
//! or its own junction table. A forget preview that has to be honest about how
//! many inferences it will orphan cannot get there by parsing JSON in Rust; it
//! needs an index to join against, which is what `inference_evidence`,
//! `memory_evidence`, `memory_content_keys` and `relationship_evidence` are for.

/// Value of `meta.schema_version`, bumped when the schema gains something a
/// reader has to know about.
///
/// 2 added `destroyed_content_keys`. Every statement below is
/// `IF NOT EXISTS`, so a version 1 file gains the table the next time it is
/// opened; what it cannot gain is a record of the forgets that already
/// happened, and ids destroyed before this version can still be re-minted.
///
/// An index is not such a gain, which is why `events_by_source` arrived after 2
/// without moving this number. It changes what a query costs and not what it
/// answers, an older file grows it on the next open like any other additive
/// statement here, and a build that predates it reads the same rows the same
/// way. Bumping for it would buy nothing and cost something real: the refusal
/// below is symmetric, so the next build back would stop opening databases it
/// understands perfectly.
///
/// `docs/DECISIONS.md` D62 is the whole policy, and it has no fourth clause.
/// Forward is additive `IF NOT EXISTS` DDL, applied on open, followed by
/// moving the stamp up — which is exactly the 1→2 path above. A file stamped
/// *newer* than this constant is refused by [`crate::store::SqlCipherStore::open`]
/// before anything is written to it, stamp included: an older build has no way
/// to know what the newer schema promises, and writing this number over a
/// larger one would turn a recoverable "wrong Soul installed" into a file every
/// later launch mistakes for its own. Nothing here ever recreates a database —
/// the forget ledger and the hash-chained audit cannot be built a second time —
/// and there is deliberately no migrations table and no numbered migration
/// files to fall out of step with the schema they claim to describe.
pub const STORE_SCHEMA_VERSION: i64 = 2;

pub const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Content keys, each wrapped under the KEK. Deleting a row here is what
-- "forget" means: the ciphertext in sealed_blobs becomes undecryptable.
CREATE TABLE IF NOT EXISTS content_keys (
    content_key_id TEXT PRIMARY KEY,
    wrapped_key    BLOB NOT NULL,
    wrap_nonce     BLOB NOT NULL
);

-- Ids that have been through a forget. Deleting the wrapped key leaves the id
-- itself free to be asked for again, and minting a fresh key under it would
-- hand a live key back to rows the forget turned into tombstones. Nothing here
-- is a secret: it is the bare UUID, which SECURITY.md already lets outlive the
-- row in the audit chain.
CREATE TABLE IF NOT EXISTS destroyed_content_keys (
    content_key_id TEXT PRIMARY KEY
);

-- Field-level ciphertext. row_id and field are also bound into the AEAD tag,
-- so a blob cannot be replayed into another row or another column.
CREATE TABLE IF NOT EXISTS sealed_blobs (
    blob_id        TEXT PRIMARY KEY,
    content_key_id TEXT NOT NULL,
    row_id         TEXT NOT NULL,
    field          TEXT NOT NULL,
    subject        TEXT NOT NULL,
    nonce          BLOB NOT NULL,
    ciphertext     BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS sealed_blobs_by_key ON sealed_blobs (content_key_id);
-- Which keys a row owns. A contact whose display name the export never gave
-- has no label blob to name their content key, so this is the index a forget
-- of that contact joins against.
CREATE INDEX IF NOT EXISTS sealed_blobs_by_row ON sealed_blobs (row_id);

CREATE TABLE IF NOT EXISTS events (
    event_id        TEXT PRIMARY KEY,
    ts              TEXT NOT NULL,
    source          TEXT NOT NULL,
    kind            TEXT NOT NULL,
    actor_subject   TEXT NOT NULL,
    privacy_subject TEXT NOT NULL,
    body_blob_id    TEXT,
    doc             TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS events_by_subject ON events (privacy_subject);
-- The collect screen asks once a second, with the store lock held, how many
-- foreground rows exist. Without this, `count(*) ... WHERE source = ?` reads
-- every event in the database to answer: an import's messages, and the rows a
-- forget left behind as tombstones, none of which the question is about. The
-- index covers that count, so it never reaches the table at all.
CREATE INDEX IF NOT EXISTS events_by_source ON events (source);

CREATE TABLE IF NOT EXISTS profiles (
    profile_id TEXT PRIMARY KEY,
    doc        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS evidence (
    evidence_id TEXT PRIMARY KEY,
    subject     TEXT NOT NULL,
    doc         TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS inferences (
    inference_id TEXT PRIMARY KEY,
    state        TEXT NOT NULL,
    doc          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS inference_evidence (
    inference_id TEXT NOT NULL,
    evidence_id  TEXT NOT NULL,
    PRIMARY KEY (inference_id, evidence_id)
);
CREATE INDEX IF NOT EXISTS inference_evidence_by_evidence
    ON inference_evidence (evidence_id);

CREATE TABLE IF NOT EXISTS memories (
    memory_id      TEXT PRIMARY KEY,
    content_key_id TEXT NOT NULL,
    forget_state   TEXT NOT NULL,
    doc            TEXT NOT NULL
);

-- Every content key a memory depends on: its own, plus the keys behind its
-- title and summary. One row per (memory, key) so the forget preview joins.
CREATE TABLE IF NOT EXISTS memory_content_keys (
    memory_id      TEXT NOT NULL,
    content_key_id TEXT NOT NULL,
    PRIMARY KEY (memory_id, content_key_id)
);
CREATE INDEX IF NOT EXISTS memory_content_keys_by_key
    ON memory_content_keys (content_key_id);

CREATE TABLE IF NOT EXISTS memory_evidence (
    memory_id   TEXT NOT NULL,
    evidence_id TEXT NOT NULL,
    PRIMARY KEY (memory_id, evidence_id)
);

CREATE TABLE IF NOT EXISTS contacts (
    contact_id           TEXT PRIMARY KEY,
    contact_class        TEXT NOT NULL,
    forget_state         TEXT NOT NULL,
    display_label_key_id TEXT,
    doc                  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS contacts_by_label_key
    ON contacts (display_label_key_id);

CREATE TABLE IF NOT EXISTS relationships (
    relationship_id  TEXT PRIMARY KEY,
    from_contact_id  TEXT NOT NULL,
    to_contact_id    TEXT NOT NULL,
    doc              TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS relationships_by_from ON relationships (from_contact_id);
CREATE INDEX IF NOT EXISTS relationships_by_to   ON relationships (to_contact_id);

CREATE TABLE IF NOT EXISTS relationship_evidence (
    relationship_id TEXT NOT NULL,
    evidence_id     TEXT NOT NULL,
    PRIMARY KEY (relationship_id, evidence_id)
);

-- Append-only and hash-chained. seq is assigned by the store, not the caller,
-- so the chain cannot be forged from outside. No column here holds prose, and
-- nothing in this table is ever deleted by a forget.
CREATE TABLE IF NOT EXISTS audit (
    seq        INTEGER PRIMARY KEY,
    entry_id   TEXT NOT NULL UNIQUE,
    ts         TEXT NOT NULL,
    prev_hash  TEXT NOT NULL,
    entry_hash TEXT NOT NULL,
    action     TEXT NOT NULL,
    decision   TEXT NOT NULL,
    doc        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_subjects (
    seq        INTEGER NOT NULL,
    subject_id TEXT NOT NULL,
    PRIMARY KEY (seq, subject_id)
);
CREATE INDEX IF NOT EXISTS audit_subjects_by_subject
    ON audit_subjects (subject_id);
"#;

#[cfg(test)]
mod tests {
    use super::DDL;

    /// The statement itself, spelled out, because the cost it removes is not
    /// visible from anywhere else. Deleting it breaks no query and fails no
    /// other test: the collect screen would keep giving the right number, once
    /// a second under the store lock, by reading every event in the database.
    #[test]
    fn the_events_table_is_indexed_by_source() {
        assert!(
            DDL.contains("CREATE INDEX IF NOT EXISTS events_by_source ON events (source);"),
            "the once-a-second foreground count falls back to a full scan without this index",
        );
    }

    /// D62 rests on this: forward is additive DDL applied on open. A statement
    /// here that is not `IF NOT EXISTS` would not run against an existing file,
    /// it would fail against one — and it would take the open down with it.
    #[test]
    fn every_statement_in_the_ddl_can_be_applied_to_a_database_that_already_has_it() {
        for statement in DDL.split(';') {
            let body: String = statement
                .lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("--"))
                .collect::<Vec<_>>()
                .join(" ");
            let body = body.trim();
            if body.is_empty() {
                continue;
            }
            assert!(
                body.starts_with("CREATE TABLE IF NOT EXISTS")
                    || body.starts_with("CREATE INDEX IF NOT EXISTS"),
                "an open has to be able to run this twice: {body}",
            );
        }
    }
}
