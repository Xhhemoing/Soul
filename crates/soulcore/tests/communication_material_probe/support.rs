use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{MemoryContextRequest, Session};
use uuid::Uuid;

pub fn setup() -> (tempfile::TempDir, Session) {
    let dir = tempfile::tempdir().expect("temporary synthetic store");
    let session = Session::open(dir.path());
    assert!(session.status().store_opened);
    (dir, session)
}

pub fn memory(session: &mut Session, title: &str, summary: &str) -> Uuid {
    let result = session
        .write_memory(&NewMemory {
            memory_type: "preference".into(),
            title: title.into(),
            summary: summary.into(),
        })
        .expect("save synthetic owner memory");
    Uuid::parse_str(&result.memory_id).expect("memory identity")
}

pub fn request(ids: &[Uuid], query: Option<&str>, time_specific: bool) -> MemoryContextRequest {
    MemoryContextRequest {
        candidate_ids: ids.to_vec(),
        query: query.map(str::to_owned),
        memory_type: None,
        time_specific,
    }
}
