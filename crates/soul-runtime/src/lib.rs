//! One Session owner for desktop IPC and the explicitly enabled local API.
//!
//! Imports, scoped preferences, memory edits and forgetting share the ownership
//! facade. Other operations and background workers retain the trusted host
//! Session handle during this extraction; long work keeps begin/execute/complete.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex, MutexGuard};

use soul_types::{
    MemoryDetailView, MemoryIdRequest, PeoplePage, PeoplePageRequest, PersonView, ReadError,
    ReadErrorCode, ReadErrorDetail, RuntimeStatus,
};
use soulcore::commands::graph::PeopleGraphView;
use soulcore::commands::memory::MemoryDetail;
use soulcore::commands::session::{Session, SessionRefusal, SessionStatus};
use uuid::Uuid;

mod ownership;
mod projection;
pub use ownership::{Caller, RuntimeContext};

#[derive(Debug, Clone)]
pub struct Runtime {
    session: Arc<Mutex<Session>>,
    synthetic: bool,
    control: Arc<Mutex<ownership::Control>>,
}

impl Runtime {
    pub fn new(session: Session) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
            synthetic: false,
            control: Arc::new(Mutex::new(ownership::Control::new())),
        }
    }

    /// Explicit host policy; the HTTP driver cannot set or change this flag.
    pub fn new_synthetic(session: Session) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
            synthetic: true,
            control: Arc::new(Mutex::new(ownership::Control::new())),
        }
    }

    pub fn is_synthetic(&self) -> bool {
        self.synthetic
    }

    /// Transitional trusted-host access, never exposed as a generic route.
    pub fn shared_session(&self) -> Arc<Mutex<Session>> {
        Arc::clone(&self.session)
    }

    pub fn held_session(&self) -> MutexGuard<'_, Session> {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn status(&self) -> RuntimeStatus {
        let status = self.legacy_status();
        RuntimeStatus {
            wizard_completed: status.wizard_completed,
            store_opened: status.store_opened,
            key_protection: match status.key_protection.as_str() {
                "dpapi" => "dpapi",
                "developer_key_file" => "developer_key_file",
                _ => "unavailable",
            }
            .to_owned(),
            config_problem: status.config_problem.is_some(),
        }
    }

    /// Bounds the response only. Session::people still reads the complete graph
    /// and resolves its evidence, retaining its existing refusal semantics.
    pub fn people(&self, request: PeoplePageRequest) -> Result<PeoplePage, ReadError> {
        if !(1..=100).contains(&request.limit) {
            return Err(invalid(ReadErrorDetail::Limit));
        }
        if request.offset > 1_000_000 {
            return Err(invalid(ReadErrorDetail::Offset));
        }
        let mut graph = self.legacy_people().map_err(|_| read_failed())?;
        graph.people.sort_by(|a, b| a.contact_id.cmp(&b.contact_id));
        let total = graph.people.len();
        let start = request.offset as usize;
        let people: Vec<_> = graph
            .people
            .into_iter()
            .skip(start)
            .take(request.limit as usize)
            .map(|person| PersonView {
                contact_id: person.contact_id,
                is_you: person.is_you,
                identifier_hint: person.identifier_hint,
                interaction_count: person.interaction_count,
                last_contact_utc: person.last_contact_utc,
                tie_count: person.tie_count,
                forgotten: person.forgotten,
            })
            .collect();
        let has_more = start.saturating_add(people.len()) < total;
        Ok(PeoplePage {
            people,
            offset: request.offset,
            limit: request.limit,
            total,
            has_more,
            third_party_data_is_local_only: graph.third_party_data_is_local_only,
        })
    }

    pub fn memory(&self, request: MemoryIdRequest) -> Result<MemoryDetailView, ReadError> {
        let id =
            Uuid::parse_str(&request.memory_id).map_err(|_| invalid(ReadErrorDetail::MemoryId))?;
        if !id
            .hyphenated()
            .to_string()
            .eq_ignore_ascii_case(&request.memory_id)
        {
            return Err(invalid(ReadErrorDetail::MemoryId));
        }
        let detail = self
            .legacy_memory(&id.to_string())
            .map_err(|_| read_failed())?;
        Ok(MemoryDetailView {
            memory_id: detail.memory_id,
            memory_type: detail.memory_type,
            title: detail.title,
            summary: detail.summary,
            third_party_content_present: detail.third_party_content_present,
        })
    }

    /// Compatibility reads for the existing desktop DTO surface.
    pub fn legacy_status(&self) -> SessionStatus {
        self.held_session().status()
    }

    pub fn legacy_people(&self) -> Result<PeopleGraphView, SessionRefusal> {
        self.held_session().people()
    }

    pub fn legacy_memory(&self, memory_id: &str) -> Result<MemoryDetail, SessionRefusal> {
        self.held_session().memory(memory_id)
    }
}

fn invalid(field: ReadErrorDetail) -> ReadError {
    ReadError {
        code: ReadErrorCode::InvalidRequest,
        message: "Invalid read request.".into(),
        details: Some(field),
    }
}

fn read_failed() -> ReadError {
    ReadError {
        code: ReadErrorCode::ReadFailed,
        message: "The requested data could not be read.".into(),
        details: None,
    }
}
