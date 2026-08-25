//! AC-14: what goes in comes back out, and the audit chain carries none of it.
//!
//! The two halves pull in opposite directions, which is the point. Read/write
//! consistency wants the prose preserved exactly — every codepoint, including
//! the fullwidth quotes and the newline-free run of Chinese that a naive
//! round-trip through a text column would mangle. The audit requirement wants
//! that same prose to appear nowhere in the log. A design that satisfies one by
//! accident usually fails the other, so both are asserted against the same
//! bytes.
//!
//! Everything runs against a real `SqlCipherStore`, and the consistency check
//! is repeated after a close and reopen. An in-process cache would pass the
//! first read and say nothing about whether the ciphertext on disk is openable.

mod common;

use common::*;

use soul_memory::{create, list, read, update, MemoryDraft, MemoryEdit, MemoryError};
use soul_schema::memory::{ForgetState, MemoryType};
use soul_store_api::{AuditLog, MemoryStore, SoulStore};
use soul_testkit::LeakageChecker;

#[test]
fn every_memory_reads_back_exactly_as_it_was_written_including_after_a_restart() {
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("crud.db");

    let stored = {
        let mut store = open(&path);
        let stored: Vec<_> = fixture
            .memories
            .iter()
            .map(|source| {
                let memory = create(&mut store, &source.draft(), NOW).expect("create");
                (source, memory)
            })
            .collect();

        for (source, memory) in &stored {
            let content = read(&store, memory.memory_id).expect("read what was just written");
            assert_eq!(
                content.title, source.title,
                "{}: the title must survive sealing unchanged",
                source.id,
            );
            assert_eq!(
                content.summary, source.summary,
                "{}: the summary must survive sealing unchanged",
                source.id,
            );
            assert_eq!(content.memory.memory_type, source.memory_type);
            assert_eq!(content.memory.forget_state, ForgetState::Active);
        }

        let ids: Vec<_> = stored
            .iter()
            .map(|(source, memory)| (source.id.clone(), memory.memory_id))
            .collect();
        store.flush().expect("flush");
        store.close().expect("close, as if the application exited");
        ids
    };

    // Reopened from the file. If the prose only round-tripped through a cache,
    // this is where it stops matching.
    let store = open(&path);
    for (id, memory_id) in &stored {
        let source = fixture.get(id);
        let content = read(&store, *memory_id).expect("read after a restart");
        assert_eq!(
            content.title, source.title,
            "{id}: the title must still be openable from the file",
        );
        assert_eq!(
            content.summary, source.summary,
            "{id}: the summary must still be openable from the file",
        );
    }
}

#[test]
fn an_edit_reseals_under_the_same_key_and_the_new_text_is_what_reads_back() {
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path().join("edit.db"));

    let source = fixture.get(&fixture.edit.target);
    let created = create(&mut store, &source.draft(), NOW).expect("create");

    let edited = update(&mut store, created.memory_id, &fixture.edit.edit(), NOW).expect("update");
    assert_eq!(
        edited.content_key_id, created.content_key_id,
        "an edit that minted a second key would leave the old title readable \
         after the memory was forgotten",
    );
    assert_eq!(edited.memory_id, created.memory_id);

    let content = read(&store, created.memory_id).expect("read the edited memory");
    assert_eq!(content.title, fixture.edit.title);
    assert_eq!(content.summary, fixture.edit.summary);

    // A partial edit leaves the other field alone rather than blanking it.
    update(
        &mut store,
        created.memory_id,
        &MemoryEdit::default().as_type(MemoryType::Semantic),
        NOW,
    )
    .expect("retype");
    let content = read(&store, created.memory_id).expect("read after retyping");
    assert_eq!(content.memory.memory_type, MemoryType::Semantic);
    assert_eq!(
        content.title, fixture.edit.title,
        "changing the type must not disturb the prose",
    );
    assert_eq!(content.summary, fixture.edit.summary);
}

#[test]
fn the_list_view_shows_counts_and_flags_and_never_the_words() {
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path().join("list.db"));

    let created: Vec<_> = fixture
        .memories
        .iter()
        .map(|source| {
            (
                source.id.as_str(),
                create(&mut store, &source.draft(), NOW)
                    .expect("create")
                    .memory_id,
            )
        })
        .collect();

    let digests = list(&store).expect("list");
    assert_eq!(digests.len(), fixture.memories.len());

    // A digest is a struct with no string field, so "no prose" is a type-level
    // fact. What is worth asserting is that its numbers are the real ones.
    for digest in &digests {
        let content = read(&store, digest.memory_id).expect("open the memory behind the digest");
        assert_eq!(
            digest.title_chars,
            content.title.chars().count() as u64,
            "the character count has to come from the sealed pointer, not a guess",
        );
        assert_eq!(digest.summary_chars, content.summary.chars().count() as u64);
        assert_eq!(digest.memory_type, content.memory.memory_type);
        assert_eq!(digest.forget_state, ForgetState::Active);
    }

    let mixed_id = created
        .iter()
        .find(|(id, _)| *id == "kitchen_argument")
        .map(|(_, memory_id)| *memory_id)
        .expect("the mixed-subject memory was created");
    let mixed_digest = digests
        .iter()
        .find(|digest| digest.memory_id == mixed_id)
        .expect("the mixed-subject memory is in the list");
    assert!(
        mixed_digest.third_party_content_present,
        "a memory quoting someone else has to be flagged as such before anything \
         considers sending it anywhere",
    );
    assert_eq!(
        digests
            .iter()
            .filter(|digest| digest.third_party_content_present)
            .count(),
        1,
        "only the one fixture memory holds another person's words",
    );
}

#[test]
fn the_audit_chain_records_the_writes_and_holds_no_part_of_what_was_written() {
    let fixture = MemoryFixture::load();
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("audit.db");

    let ids = {
        let mut store = open(&path);
        let mut ids = Vec::new();
        for source in &fixture.memories {
            ids.push(
                create(&mut store, &source.draft(), NOW)
                    .expect("create")
                    .memory_id,
            );
        }
        let target = fixture.get(&fixture.edit.target);
        let target_id = ids[fixture
            .memories
            .iter()
            .position(|memory| memory.id == target.id)
            .expect("the edit target is in the fixture")];
        update(&mut store, target_id, &fixture.edit.edit(), NOW).expect("update");
        store.flush().expect("flush");
        store.close().expect("close");
        ids
    };

    let store = open(&path);
    let audit = store.list_audit().expect("audit survives the restart");
    assert_eq!(
        audit.len(),
        fixture.memories.len() + 1,
        "one entry per create plus one for the edit",
    );
    store
        .verify_audit_chain()
        .expect("the hash chain must verify after a restart");

    for (entry, memory_id) in audit.iter().zip(&ids) {
        assert_eq!(
            entry.subject_refs.as_deref(),
            Some([*memory_id].as_slice()),
            "an audit entry names the row by id and by nothing else",
        );
        assert_eq!(
            entry.counts.as_ref().and_then(|counts| counts.items),
            Some(2),
            "a create seals two fields, and the count has to say so",
        );
        assert!(
            entry
                .counts
                .as_ref()
                .and_then(|counts| counts.bytes)
                .is_none(),
            "a byte count of the plaintext would be a size oracle for the prose",
        );
    }

    // The audit is JSON on the way to anywhere a human reads it, so the
    // leakage check runs against the serialized chain rather than field by
    // field: a future field carrying prose is then caught without this test
    // being taught about it.
    let encoded = serde_json::to_string(&audit).expect("audit serializes");
    let mut checker = LeakageChecker::new().with_min_ngram(4);
    for (index, prose) in fixture.prose().iter().enumerate() {
        checker.add_third_party_body(format!("prose-{index}"), prose);
    }
    checker.assert_clean("the audit chain", &encoded);
}

#[test]
fn a_memory_with_nothing_in_it_is_refused_and_so_is_an_edit_that_empties_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path().join("empty.db"));

    for blank in ["", "   ", "\n\t"] {
        assert!(
            matches!(
                create(
                    &mut store,
                    &MemoryDraft::own(MemoryType::Episodic, blank, "有摘要没标题"),
                    NOW,
                ),
                Err(MemoryError::Empty("title")),
            ),
            "whitespace is not a title",
        );
        assert!(matches!(
            create(
                &mut store,
                &MemoryDraft::own(MemoryType::Episodic, "有标题没摘要", blank),
                NOW,
            ),
            Err(MemoryError::Empty("summary")),
        ));
    }

    assert!(
        store.list_memories().expect("list").is_empty(),
        "a refused draft must not leave a row behind",
    );
    assert!(
        store.list_audit().expect("audit").is_empty(),
        "and must not leave an audit entry either: nothing happened",
    );

    let good = create(
        &mut store,
        &MemoryDraft::own(MemoryType::Episodic, "标题", "摘要"),
        NOW,
    )
    .expect("create");
    assert!(matches!(
        update(&mut store, good.memory_id, &MemoryEdit::title("  "), NOW),
        Err(MemoryError::Empty("title")),
    ));
    assert_eq!(
        read(&store, good.memory_id).expect("read").title,
        "标题",
        "a refused edit leaves the stored text as it was",
    );
}
