use serde_json::Value;
use soul_policy::e1::{DRAFTING_INSTRUCTION, QUOTE_CLOSE, QUOTE_OPEN};
use soul_schema::common::SealedSubject;
use soul_schema::contact::ContactClass;
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::{BlobStore, GraphStore, MemoryStore, SealRequest};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::draft::E1DraftPlan;
use soulcore::commands::session::*;
use soulcore::commands::store::database_path;
use uuid::Uuid;

pub const PASTE: &str = "合成对方消息原文哨兵";

pub fn pointer(item: &AssistanceItemDetail) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item.item_id,
        expected_revision: item.item.revision,
    }
}

pub fn goal_id(item: &AssistanceItemDetail) -> Uuid {
    match item.item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => memory_id,
        _ => panic!("goal"),
    }
}

pub fn fixture(summary: &str) -> (tempfile::TempDir, Session, MockLlm, AssistanceItemDetail) {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    assert!(session.status().store_opened);
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap();
    let work = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    let bundle = session.local_insights().unwrap().remove(0);
    let observation = session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: bundle.bundle_id,
            contact_id: Uuid::parse_str(&bundle.summaries[0].contact_id).unwrap(),
        })
        .unwrap();
    let item = session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "合成沟通目标".into(),
            summary: summary.into(),
            deadline_at: None,
            source_observation: Some(pointer(&observation)),
        })
        .unwrap();
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply("合成固定回复。");
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    (dir, session, endpoint, item)
}

pub fn select(
    session: &mut Session,
    item: &AssistanceItemDetail,
) -> (
    InsightContextView,
    Option<soul_memory::MemorySelectionPreview>,
) {
    let AssistanceContextView::Insight {
        insight,
        goal_preview,
        ..
    } = session.prepare_assistance_context(pointer(item)).unwrap()
    else {
        panic!("bound goal selects insight")
    };
    (insight, goal_preview.map(|preview| *preview))
}

pub fn register_name(session: &Session, name: &str) {
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let mut contact = store
        .list_contacts()
        .unwrap()
        .into_iter()
        .find(|c| c.contact_class == ContactClass::ThirdParty)
        .unwrap();
    contact.display_label_ref = Some(
        store
            .seal(SealRequest::new(
                Uuid::now_v7(),
                contact.contact_id,
                "display_label_ref",
                SealedSubject::ThirdParty,
                name.as_bytes().to_vec(),
            ))
            .unwrap(),
    );
    store.put_contact(contact).unwrap();
}

pub fn edit_direct(dir: &std::path::Path, session: &Session, item: &AssistanceItemDetail) {
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let mut row = store.get_memory(goal_id(item)).unwrap();
    row.summary_ref = Some(
        store
            .seal(SealRequest::new(
                row.content_key_id,
                row.memory_id,
                "summary_ref",
                SealedSubject::Owner,
                "修改后的合成目标".as_bytes().to_vec(),
            ))
            .unwrap(),
    );
    let document = serde_json::to_string(&row).unwrap();
    drop(store);
    // Test-only storage fault: legal new sealed text, but bypass put_memory's
    // automatic assistance revision update. No production mutation does this.
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(dir.join(KEY_BLOB_FILE_NAME)))
    } else {
        Box::new(TestKeyProvider::in_dir(dir))
    };
    let conn = rusqlite::Connection::open(database_path(dir)).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    assert_eq!(
        conn.execute(
            "UPDATE memories SET doc=?1 WHERE memory_id=?2",
            rusqlite::params![document, row.memory_id.to_string()]
        )
        .unwrap(),
        1
    );
    assert!(
        session
            .assistance_item(&item.item.item_id.to_string())
            .unwrap()
            .goal
            .unwrap()
            .summary
            == "修改后的合成目标"
    );
    assert!(
        session
            .assistance_item(&item.item.item_id.to_string())
            .unwrap()
            .item
            .revision
            == item.item.revision
    );
}

pub fn capture(
    session: &mut Session,
    endpoint: &MockLlm,
    plan: &E1DraftPlan,
    preview: &str,
) -> String {
    let before = endpoint.request_count();
    let approval = plan.approval();
    let work = session.begin_draft(&approval).unwrap();
    session.complete_draft(work.execute()).unwrap();
    assert_eq!(endpoint.request_count(), before + 1);
    assert!(session.begin_draft(&approval).is_err());
    assert_eq!(endpoint.request_count(), before + 1);
    let requests = endpoint.requests();
    let body: Value = serde_json::from_str(&requests[before].body).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages[0]["role"] == "system");
    assert!(messages[0]["content"] == DRAFTING_INSTRUCTION);
    assert!(messages[1]["role"] == "user");
    let material = messages[1]["content"].as_str().unwrap();
    assert!(material == format!("{QUOTE_OPEN}\n{preview}\n{QUOTE_CLOSE}"));
    material.to_owned()
}
