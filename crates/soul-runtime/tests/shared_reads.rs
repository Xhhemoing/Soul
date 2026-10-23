use std::sync::Arc;

use serde_json::json;
use soul_runtime::Runtime;
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_types::{MemoryIdRequest, PeoplePageRequest, ReadErrorCode};
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::Session;

fn fixture() -> (tempfile::TempDir, Runtime) {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::open(directory.path());
    assert!(session.status().store_opened);
    (directory, Runtime::new(session))
}

#[test]
fn runtime_clones_read_mutations_on_the_same_session_and_store() {
    let (_directory, runtime) = fixture();
    let reader = runtime.clone();
    let first = runtime.shared_session();
    let second = reader.shared_session();
    assert!(Arc::ptr_eq(&first, &second));
    let written = first
        .lock()
        .unwrap()
        .write_memory(&NewMemory {
            memory_type: "episodic".into(),
            title: "Synthetic shared runtime".into(),
            summary: "An IPC-style mutation remains visible to a runtime clone.".into(),
        })
        .unwrap();
    let other = runtime
        .held_session()
        .write_memory(&NewMemory {
            memory_type: "episodic".into(),
            title: "Another synthetic memory".into(),
            summary: "A different ID must never be used as a fallback.".into(),
        })
        .unwrap();
    assert_ne!(written.memory_id, other.memory_id);
    let detail = reader
        .memory(MemoryIdRequest {
            memory_id: written.memory_id.clone(),
        })
        .unwrap();
    assert_eq!(detail.memory_id, written.memory_id);
    assert_eq!(detail.title, "Synthetic shared runtime");
    assert_eq!(
        detail.summary,
        "An IPC-style mutation remains visible to a runtime clone."
    );
    assert!(!detail.third_party_content_present);
    assert!(serde_json::to_value(detail)
        .unwrap()
        .get("content_key_id")
        .is_none());
    let a = runtime.held_session().store().unwrap();
    let b = reader.held_session().store().unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn people_pages_are_stable_disjoint_and_contain_only_node_metadata() {
    let (_directory, runtime) = fixture();
    let text =
        soul_testkit::fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap();
    let work = runtime
        .held_session()
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let completion = work.execute();
    let prepared = runtime
        .held_session()
        .complete_import_prepare(completion)
        .unwrap();
    let work = runtime
        .held_session()
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    let completion = work.execute();
    runtime.held_session().complete_import(completion).unwrap();

    let first = runtime
        .people(PeoplePageRequest {
            limit: 2,
            offset: 0,
        })
        .unwrap();
    let last = runtime
        .people(PeoplePageRequest {
            limit: 2,
            offset: 2,
        })
        .unwrap();
    assert_eq!(first.total, 3);
    assert_eq!(first.people.len(), 2);
    assert!(first.has_more);
    assert_eq!(last.people.len(), 1);
    assert!(!last.has_more);
    let ids: Vec<_> = first
        .people
        .iter()
        .chain(&last.people)
        .map(|p| p.contact_id.as_str())
        .collect();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        first,
        runtime
            .people(PeoplePageRequest {
                limit: 2,
                offset: 0
            })
            .unwrap()
    );
    assert!(first.third_party_data_is_local_only);
    let encoded = serde_json::to_value(&first).unwrap();
    assert!(encoded.get("ties").is_none());
    assert!(encoded.get("evidence").is_none());
    for person in encoded["people"].as_array().unwrap() {
        assert!(person.get("title").is_none());
        assert!(person.get("source_refs").is_none());
        assert!(person.get("evidence").is_none());
    }
    assert!(runtime
        .people(PeoplePageRequest {
            limit: 100,
            offset: 3
        })
        .unwrap()
        .people
        .is_empty());
    assert!(runtime
        .people(PeoplePageRequest {
            limit: 1,
            offset: 1_000_000
        })
        .unwrap()
        .people
        .is_empty());
}

#[test]
fn invalid_requests_and_unknown_memories_have_static_safe_errors() {
    let (_directory, runtime) = fixture();
    for request in [
        PeoplePageRequest {
            limit: 0,
            offset: 0,
        },
        PeoplePageRequest {
            limit: 101,
            offset: 0,
        },
        PeoplePageRequest {
            limit: 1,
            offset: 1_000_001,
        },
    ] {
        assert_eq!(
            runtime.people(request).unwrap_err().code,
            ReadErrorCode::InvalidRequest
        );
    }
    let invalid = runtime
        .memory(MemoryIdRequest {
            memory_id: "C:/SYNTHETIC_PRIVATE_PATH/not-a-uuid".into(),
        })
        .unwrap_err();
    assert_eq!(invalid.code, ReadErrorCode::InvalidRequest);
    assert!(!serde_json::to_string(&invalid)
        .unwrap()
        .contains("SYNTHETIC_PRIVATE_PATH"));
    for memory_id in [
        "018f00a0000070008000000000000001",
        "{018f00a0-0000-7000-8000-000000000001}",
        "urn:uuid:018f00a0-0000-7000-8000-000000000001",
        " 018f00a0-0000-7000-8000-000000000001",
    ] {
        assert_eq!(
            runtime
                .memory(MemoryIdRequest {
                    memory_id: memory_id.into()
                })
                .unwrap_err()
                .code,
            ReadErrorCode::InvalidRequest,
        );
    }
    let unknown = runtime
        .memory(MemoryIdRequest {
            memory_id: "018f00a0-0000-7000-8000-000000000001".into(),
        })
        .unwrap_err();
    assert_eq!(unknown.code, ReadErrorCode::ReadFailed);
    assert_eq!(unknown.details, None);
    assert!(
        serde_json::from_value::<PeoplePageRequest>(json!({"limit": 1, "extra": true})).is_err()
    );
    assert!(
        serde_json::from_value::<MemoryIdRequest>(json!({"memory_id": "x", "extra": true}))
            .is_err()
    );
}

#[test]
fn unavailable_status_and_read_errors_do_not_include_the_store_path() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("SYNTHETIC_PRIVATE_PATH");
    std::fs::write(&root, b"a synthetic file prevents a data directory").unwrap();
    let runtime = Runtime::new(Session::open(&root));
    let status = runtime.status();
    assert!(!status.store_opened);
    let encoded = serde_json::to_string(&status).unwrap();
    assert!(!encoded.contains("SYNTHETIC_PRIVATE_PATH"));
    assert!(!encoded.contains("store_notice"));
    let error = runtime.people(PeoplePageRequest::default()).unwrap_err();
    assert_eq!(error.code, ReadErrorCode::ReadFailed);
    assert!(!serde_json::to_string(&error)
        .unwrap()
        .contains("SYNTHETIC_PRIVATE_PATH"));
}

#[test]
fn a_panicking_owner_does_not_replace_or_abandon_the_session() {
    let (_directory, runtime) = fixture();
    let shared = runtime.shared_session();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = shared.lock().unwrap();
        panic!("synthetic command panic");
    }));
    assert!(result.is_err());
    assert!(runtime.status().store_opened);
    assert!(runtime
        .people(PeoplePageRequest::default())
        .unwrap()
        .people
        .is_empty());
}
