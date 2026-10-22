use super::*;
use soul_store_api::forget::ForgetOutcomeOps;
use soul_store_api::ForgetUnit;
use std::sync::{mpsc, Arc, Barrier};
use std::time::Duration;

// Compare actual persisted values without printing ciphertext or wrapped keys.
// Includes the full original audit chain, not merely its length or tip.
pub(super) fn database_rows(store: &SqlCipherStore) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    [
        "SELECT * FROM memories ORDER BY memory_id",
        "SELECT * FROM sealed_blobs ORDER BY blob_id",
        "SELECT * FROM content_keys ORDER BY content_key_id",
        "SELECT * FROM destroyed_content_keys ORDER BY content_key_id",
        "SELECT * FROM memory_content_keys ORDER BY memory_id,content_key_id",
        "SELECT * FROM memory_evidence ORDER BY memory_id,evidence_id",
        "SELECT * FROM audit ORDER BY seq",
    ]
    .iter()
    .map(|sql| {
        let mut statement = store.conn.prepare(sql).unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    })
    .collect()
}

fn setup() -> (tempfile::TempDir, SqlCipherStore, SoulMemory) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SqlCipherStore::open(
        dir.path().join("n1.db"),
        &TestKeyProvider::from_seed("n1 synthetic fixture"),
    )
    .unwrap();
    let memory = fixture(&mut store);
    (dir, store, memory)
}

fn corrupt(store: &SqlCipherStore, memory: &SoulMemory) {
    let id = memory.summary_ref.as_ref().unwrap().blob_id;
    let mut ciphertext = blob(store, id).unwrap().ciphertext.to_vec();
    ciphertext[0] ^= 1;
    assert_eq!(
        store
            .conn
            .execute(
                "UPDATE sealed_blobs SET ciphertext=?1 WHERE blob_id=?2",
                rusqlite::params![ciphertext, id.to_string()],
            )
            .unwrap(),
        1
    );
    assert_eq!(
        store.open(memory.summary_ref.as_ref().unwrap()),
        Err(StoreError::SealBroken(id))
    );
}

fn refuses_without_writes(store: &mut SqlCipherStore, held: Held, budget: usize) {
    let before = database_rows(store);
    assert!(repair(store, held, budget).is_err());
    assert!(database_rows(store) == before);
    assert!(!store.transaction_in_progress());
}

fn total_changes(store: &SqlCipherStore) -> u64 {
    store
        .conn
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn n1_original_and_other_authentic_payload_are_not_overwritten() {
    for authentic_replacement in [false, true] {
        let (_dir, mut store, memory) = setup();
        let held = capture(&mut store, memory.memory_id, 128).unwrap();
        if authentic_replacement {
            // Adversarial fixture only: deliberately authenticate another
            // payload under the same nonce. The repair must not overwrite it.
            let key = store.content_key(memory.content_key_id).unwrap().unwrap();
            let ciphertext = XChaCha20Poly1305::new(Key::from_slice(key.expose()))
                .encrypt(
                    XNonce::from_slice(&held.blob.nonce),
                    Payload {
                        msg: b"other valid value",
                        aad: field_aad(&memory.memory_id, FIELD).as_bytes(),
                    },
                )
                .unwrap();
            store
                .conn
                .execute(
                    "UPDATE sealed_blobs SET ciphertext=?1 WHERE blob_id=?2",
                    rusqlite::params![
                        ciphertext,
                        memory.summary_ref.as_ref().unwrap().blob_id.to_string()
                    ],
                )
                .unwrap();
        }
        let before = database_rows(&store);
        let changes = total_changes(&store);
        assert_eq!(repair(&mut store, held, 128).unwrap(), 0);
        assert_eq!(total_changes(&store), changes);
        assert!(database_rows(&store) == before);
    }
}

#[test]
fn n1_nonempty_associations_refuse_capture_and_commit_even_when_readable() {
    for evidence in [false, true] {
        for during_capture in [false, true] {
            let (_dir, mut store, mut memory) = setup();
            let held = if during_capture {
                None
            } else {
                Some(capture(&mut store, memory.memory_id, 128).unwrap())
            };
            if evidence {
                memory.evidence_ids = Some(vec![Uuid::now_v7()]);
            } else {
                memory.source_event_ids = Some(vec![Uuid::now_v7()]);
            }
            store.put_memory(memory.clone()).unwrap();
            assert!(store.open(memory.summary_ref.as_ref().unwrap()).is_ok());
            let before = database_rows(&store);
            if let Some(held) = held {
                corrupt(&store, &memory);
                refuses_without_writes(&mut store, held, 128);
            } else {
                assert!(capture(&mut store, memory.memory_id, 128).is_err());
                assert!(database_rows(&store) == before);
            }
        }
    }
}

#[test]
fn n1_empty_associations_are_allowed_but_read_errors_are_not_empty() {
    let (_dir, mut store, mut memory) = setup();
    memory.source_event_ids = Some(vec![]);
    memory.evidence_ids = Some(vec![]);
    store.put_memory(memory.clone()).unwrap();
    let held = capture(&mut store, memory.memory_id, 128).unwrap();
    corrupt(&store, &memory);
    assert_eq!(repair(&mut store, held, 128).unwrap(), 1);
    store
        .conn
        .execute(
            "UPDATE memories SET doc='malformed' WHERE memory_id=?1",
            [memory.memory_id.to_string()],
        )
        .unwrap();
    let before = database_rows(&store);
    assert!(capture(&mut store, memory.memory_id, 128).is_err());
    assert!(database_rows(&store) == before);
}

#[test]
fn n1_dependency_changes_refuse_without_residual_writes() {
    for change in [
        "nonce",
        "row",
        "field",
        "subject",
        "blob_key",
        "wrapped_key",
        "wrap_nonce",
        "key_missing",
        "tombstone",
        "memory_key",
        "type",
        "pointer",
        "aad",
        "char_count",
        "pending",
        "forgotten",
        "memory_missing",
        "blob_missing",
        "denormalized",
        "title_pointer",
        "third_party",
    ] {
        let (_dir, mut store, mut memory) = setup();
        let held = capture(&mut store, memory.memory_id, 128).unwrap();
        corrupt(&store, &memory);
        let id = memory.summary_ref.as_ref().unwrap().blob_id.to_string();
        match change {
            "nonce" => {
                store
                    .conn
                    .execute(
                        "UPDATE sealed_blobs SET nonce=zeroblob(24) WHERE blob_id=?1",
                        [&id],
                    )
                    .unwrap();
            }
            "row" => {
                store
                    .conn
                    .execute(
                        "UPDATE sealed_blobs SET row_id=?1 WHERE blob_id=?2",
                        rusqlite::params![Uuid::now_v7().to_string(), id],
                    )
                    .unwrap();
            }
            "field" => {
                store
                    .conn
                    .execute(
                        "UPDATE sealed_blobs SET field='title_ref' WHERE blob_id=?1",
                        [&id],
                    )
                    .unwrap();
            }
            "subject" => {
                store
                    .conn
                    .execute(
                        "UPDATE sealed_blobs SET subject='third_party' WHERE blob_id=?1",
                        [&id],
                    )
                    .unwrap();
            }
            "blob_key" => {
                store
                    .conn
                    .execute(
                        "UPDATE sealed_blobs SET content_key_id=?1 WHERE blob_id=?2",
                        rusqlite::params![Uuid::now_v7().to_string(), id],
                    )
                    .unwrap();
            }
            "wrapped_key" => {
                store.conn.execute("UPDATE content_keys SET wrapped_key=zeroblob(length(wrapped_key)) WHERE content_key_id=?1", [memory.content_key_id.to_string()]).unwrap();
            }
            "wrap_nonce" => {
                store
                    .conn
                    .execute(
                        "UPDATE content_keys SET wrap_nonce=zeroblob(24) WHERE content_key_id=?1",
                        [memory.content_key_id.to_string()],
                    )
                    .unwrap();
            }
            "key_missing" => {
                store
                    .conn
                    .execute(
                        "DELETE FROM content_keys WHERE content_key_id=?1",
                        [memory.content_key_id.to_string()],
                    )
                    .unwrap();
            }
            "tombstone" => {
                store
                    .conn
                    .execute(
                        "INSERT INTO destroyed_content_keys VALUES (?1)",
                        [memory.content_key_id.to_string()],
                    )
                    .unwrap();
            }
            "memory_missing" => {
                store
                    .conn
                    .execute(
                        "DELETE FROM memories WHERE memory_id=?1",
                        [memory.memory_id.to_string()],
                    )
                    .unwrap();
            }
            "blob_missing" => {
                store
                    .conn
                    .execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [&id])
                    .unwrap();
            }
            "denormalized" => {
                store
                    .conn
                    .execute(
                        "UPDATE memories SET forget_state='forgotten' WHERE memory_id=?1",
                        [memory.memory_id.to_string()],
                    )
                    .unwrap();
            }
            _ => {
                match change {
                    "memory_key" => memory.content_key_id = Uuid::now_v7(),
                    "type" => memory.memory_type = MemoryType::Preference,
                    "pointer" => memory.summary_ref = memory.title_ref.clone(),
                    "title_pointer" => memory.title_ref = memory.summary_ref.clone(),
                    "aad" => memory.summary_ref.as_mut().unwrap().aad = Some("wrong".into()),
                    "char_count" => memory.summary_ref.as_mut().unwrap().char_count += 1,
                    "pending" => memory.forget_state = ForgetState::PendingForget,
                    "forgotten" => memory.forget_state = ForgetState::Forgotten,
                    "third_party" => memory.third_party_content_present = Some(true),
                    _ => unreachable!(),
                }
                store.put_memory(memory.clone()).unwrap();
            }
        }
        refuses_without_writes(&mut store, held, 128);
    }
}

#[test]
fn n1_candidate_authentication_budget_utf8_and_character_constraints() {
    for invalid in [
        "wrong_bytes",
        "old_ciphertext",
        "budget",
        "utf8",
        "blank",
        "characters",
    ] {
        let (_dir, mut store, memory) = setup();
        let mut held = capture(&mut store, memory.memory_id, 128).unwrap();
        let pointer = memory.summary_ref.as_ref().unwrap();
        match invalid {
            "wrong_bytes" => held.blob.ciphertext[0] ^= 2,
            "old_ciphertext" => {
                // A prior independently sealed payload has a different nonce;
                // even identical plaintext cannot be used as the held original.
                let old = store
                    .seal(SealRequest::new(
                        memory.content_key_id,
                        memory.memory_id,
                        FIELD,
                        SealedSubject::Owner,
                        b"synthetic summary".to_vec(),
                    ))
                    .unwrap();
                held.blob.ciphertext = blob(&store, old.blob_id).unwrap().ciphertext;
            }
            "utf8" | "blank" | "characters" => {
                let bytes: &[u8] = match invalid {
                    "utf8" => &[0xff],
                    "blank" => b"   ",
                    _ => b"short",
                };
                let key = store.content_key(memory.content_key_id).unwrap().unwrap();
                held.blob.ciphertext = Zeroizing::new(
                    XChaCha20Poly1305::new(Key::from_slice(key.expose()))
                        .encrypt(
                            XNonce::from_slice(&held.blob.nonce),
                            Payload {
                                msg: bytes,
                                aad: field_aad(&memory.memory_id, FIELD).as_bytes(),
                            },
                        )
                        .unwrap(),
                );
            }
            _ => {}
        }
        corrupt(&store, &memory);
        refuses_without_writes(&mut store, held, if invalid == "budget" { 2 } else { 128 });
        assert_eq!(
            store.open(pointer),
            Err(StoreError::SealBroken(pointer.blob_id))
        );
    }
}

#[test]
fn n1_capture_rejects_authenticated_invalid_text_and_budget() {
    for bytes in [vec![0xff], b"   ".to_vec(), b"too long".to_vec()] {
        let (_dir, mut store, mut memory) = setup();
        memory.summary_ref = Some(
            store
                .seal(SealRequest::new(
                    memory.content_key_id,
                    memory.memory_id,
                    FIELD,
                    SealedSubject::Owner,
                    bytes.clone(),
                ))
                .unwrap(),
        );
        store.put_memory(memory.clone()).unwrap();
        let before = database_rows(&store);
        assert!(capture(
            &mut store,
            memory.memory_id,
            if bytes == b"too long" { 3 } else { 128 }
        )
        .is_err());
        assert!(database_rows(&store) == before);
    }
}

#[test]
fn n1_faults_and_condition_update_miss_roll_back_every_write() {
    for point in [
        Point::Locked,
        Point::Validated,
        Point::Updated,
        Point::BeforeCommit,
    ] {
        let (_dir, mut store, memory) = setup();
        let held = capture(&mut store, memory.memory_id, 128).unwrap();
        corrupt(&store, &memory);
        let before = database_rows(&store);
        assert!(repair_with(&mut store, held, 128, |reached, _| {
            if reached == point {
                Err(reject())
            } else {
                Ok(())
            }
        })
        .is_err());
        assert!(database_rows(&store) == before);
        assert!(!store.transaction_in_progress());
    }
    let (_dir, mut store, memory) = setup();
    let held = capture(&mut store, memory.memory_id, 128).unwrap();
    corrupt(&store, &memory);
    let before = database_rows(&store);
    assert!(repair_with(&mut store, held, 128, |point, store| {
        if point == Point::Validated {
            store.conn.execute("UPDATE sealed_blobs SET ciphertext=zeroblob(length(ciphertext)) WHERE blob_id=?1",
                [memory.summary_ref.as_ref().unwrap().blob_id.to_string()]).map_err(backend)?;
        }
        Ok(())
    }).is_err());
    assert!(database_rows(&store) == before);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mutation {
    Edit,
    Source,
    Evidence,
    Forget,
    Unrelated,
}

fn mutate(store: &mut SqlCipherStore, target: Uuid, mutation: Mutation) -> StoreResult<()> {
    if mutation == Mutation::Forget {
        // This is the existing logical destruction implementation. Its cleanup
        // outcome is deliberately not promoted to a physical WAL-erasure claim.
        store.execute_forget_outcome(ForgetUnit::Memory(target))?;
        return Ok(());
    }
    store.transact(|store| {
        let mut memory = store.get_memory(target)?;
        match mutation {
            Mutation::Edit => {
                memory.summary_ref = Some(store.seal(SealRequest::new(
                    memory.content_key_id,
                    target,
                    FIELD,
                    SealedSubject::Owner,
                    b"synthetic edited summary".to_vec(),
                ))?);
            }
            Mutation::Source => memory.source_event_ids = Some(vec![Uuid::now_v7()]),
            Mutation::Evidence => memory.evidence_ids = Some(vec![Uuid::now_v7()]),
            Mutation::Unrelated => memory.memory_type = MemoryType::Preference,
            Mutation::Forget => unreachable!(),
        }
        store.put_memory(memory)?;
        Ok(())
    })
}

fn reopen(dir: &tempfile::TempDir) -> SqlCipherStore {
    SqlCipherStore::open(
        dir.path().join("n1.db"),
        &TestKeyProvider::from_seed("n1 synthetic fixture"),
    )
    .unwrap()
}

fn assert_forgotten_and_independent(
    store: &SqlCipherStore,
    memory: &SoulMemory,
    independent: &SoulMemory,
) {
    assert!(store.content_key(memory.content_key_id).unwrap().is_none());
    assert!(store.content_key_destroyed(memory.content_key_id).unwrap());
    assert_eq!(
        store.get_memory(memory.memory_id).unwrap().forget_state,
        ForgetState::Forgotten
    );
    let remaining: i64 = store
        .conn
        .query_row(
            "SELECT count(*) FROM sealed_blobs WHERE content_key_id=?1",
            [memory.content_key_id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(
        store.open(memory.summary_ref.as_ref().unwrap()),
        Err(StoreError::ContentKeyDestroyed(memory.content_key_id))
    );
    assert!(
        store
            .open(independent.summary_ref.as_ref().unwrap())
            .unwrap()
            == b"synthetic summary"
    );
}

// Both peers are live SQLCipher connections before locking begins. The second
// thread performs a real edit/link/forget transaction with busy_timeout(0);
// SQLite's lock error is observed, not inferred from sleep or scheduling.
// Barrier rendezvous fixes when its first attempt starts; channels bound waits
// and release its second ordinary mutation only after the first owner exits.
#[test]
fn n1_two_connections_contend_then_mutation_commits_before_repair() {
    for mutation in [
        Mutation::Edit,
        Mutation::Source,
        Mutation::Evidence,
        Mutation::Forget,
        Mutation::Unrelated,
    ] {
        let (dir, mut store, memory) = setup();
        let independent = fixture(&mut store);
        let held = capture(&mut store, memory.memory_id, 128).unwrap();
        corrupt(&store, &memory);
        let mut second = reopen(&dir);
        second.conn.busy_timeout(Duration::ZERO).unwrap();
        let start = Arc::new(Barrier::new(2));
        let peer_start = start.clone();
        let (busy_tx, busy_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let target = if mutation == Mutation::Unrelated {
            independent.memory_id
        } else {
            memory.memory_id
        };
        let worker = std::thread::spawn(move || {
            peer_start.wait();
            let first = mutate(&mut second, target, mutation);
            busy_tx.send(first).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            mutate(&mut second, target, mutation)?;
            Ok::<_, StoreError>(second)
        });
        // An explicit lock-holding fixture, not a repair attempt: no consumed
        // repair request is retried after lock contention.
        store.conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        start.wait();
        let contention = busy_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(matches!(&contention, Err(StoreError::Backend(message))
            if message.contains("locked")));
        store.conn.execute_batch("ROLLBACK").unwrap();
        release_tx.send(()).unwrap();
        let second = worker.join().unwrap().unwrap();
        let before = database_rows(&store);
        let result = repair(&mut store, held, 128);
        if mutation == Mutation::Unrelated {
            assert_eq!(result.unwrap(), 1);
            assert_eq!(
                store.get_memory(independent.memory_id).unwrap().memory_type,
                MemoryType::Preference
            );
        } else {
            assert!(result.is_err());
            assert!(database_rows(&store) == before);
        }
        eprintln!(
            "N1 {mutation:?}: contention observed; mutation committed; repair {}",
            if mutation == Mutation::Unrelated {
                "committed"
            } else {
                "refused"
            }
        );
        drop(second);
        drop(store);
        if mutation == Mutation::Forget {
            assert_forgotten_and_independent(&reopen(&dir), &memory, &independent);
        }
    }
}

#[test]
fn n1_two_connections_repair_commits_before_waiting_mutation() {
    for mutation in [
        Mutation::Edit,
        Mutation::Source,
        Mutation::Evidence,
        Mutation::Forget,
        Mutation::Unrelated,
    ] {
        let (dir, mut store, memory) = setup();
        let independent = fixture(&mut store);
        let held = capture(&mut store, memory.memory_id, 128).unwrap();
        corrupt(&store, &memory);
        let mut second = reopen(&dir);
        second.conn.busy_timeout(Duration::ZERO).unwrap();
        let start = Arc::new(Barrier::new(2));
        let peer_start = start.clone();
        let (busy_tx, busy_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let target = if mutation == Mutation::Unrelated {
            independent.memory_id
        } else {
            memory.memory_id
        };
        let worker = std::thread::spawn(move || {
            peer_start.wait();
            busy_tx.send(mutate(&mut second, target, mutation)).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            mutate(&mut second, target, mutation)?;
            Ok::<_, StoreError>(second)
        });
        assert_eq!(
            repair_with(&mut store, held, 128, |point, _| {
                if point == Point::Validated {
                    start.wait();
                    let contention = busy_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                    assert!(matches!(&contention, Err(StoreError::Backend(message))
                    if message.contains("locked")));
                }
                Ok(())
            })
            .unwrap(),
            1
        );
        // At this exact boundary the actual transaction has committed and no
        // second mutation can have begun its successful attempt.
        assert!(store.open(memory.summary_ref.as_ref().unwrap()).unwrap() == b"synthetic summary");
        release_tx.send(()).unwrap();
        let second = worker.join().unwrap().unwrap();
        let current = store.get_memory(memory.memory_id).unwrap();
        match mutation {
            Mutation::Edit => {
                assert_ne!(current.summary_ref, memory.summary_ref);
                assert!(
                    store.open(current.summary_ref.as_ref().unwrap()).unwrap()
                        == b"synthetic edited summary"
                );
            }
            Mutation::Source => assert!(!current.source_event_ids.unwrap().is_empty()),
            Mutation::Evidence => assert!(!current.evidence_ids.unwrap().is_empty()),
            Mutation::Unrelated => assert_eq!(
                store.get_memory(independent.memory_id).unwrap().memory_type,
                MemoryType::Preference
            ),
            Mutation::Forget => assert_forgotten_and_independent(&store, &memory, &independent),
        }
        eprintln!("N1 {mutation:?}: contention observed; repair committed; mutation committed");
        drop(second);
        drop(store);
        if mutation == Mutation::Forget {
            assert_forgotten_and_independent(&reopen(&dir), &memory, &independent);
        }
    }
}

#[test]
fn n1_busy_repair_consumes_context_without_writes() {
    let (dir, mut store, memory) = setup();
    let held = capture(&mut store, memory.memory_id, 128).unwrap();
    corrupt(&store, &memory);
    let second = reopen(&dir);
    store.conn.busy_timeout(Duration::ZERO).unwrap();
    second.conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    let before = database_rows(&store);
    assert!(
        matches!(repair(&mut store, held, 128), Err(StoreError::Backend(message))
        if message.contains("locked"))
    );
    assert!(database_rows(&store) == before);
    second.conn.execute_batch("ROLLBACK").unwrap();
    // `held` moved; retry would require a fresh normal successful capture,
    // which cannot be made from this still-corrupted payload.
    assert!(capture(&mut store, memory.memory_id, 128).is_err());
}
