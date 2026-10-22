//! Literal, local task diagnostics. Task prose is never an endpoint grant.
use super::*;
pub use soul_memory::{TaskAssessment, TaskRequirement, TaskRequirementStatus, TaskStatus};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskContextRequest {
    pub candidate_ids: Vec<Uuid>,
    pub task: String,
}

impl std::fmt::Debug for TaskContextRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskContextRequest")
            .field("candidate_ids", &self.candidate_ids)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskContextView {
    pub memory: MemoryContextView,
    pub assessment: TaskAssessment,
}

impl Session {
    pub fn prepare_task_context(
        &mut self,
        request: TaskContextRequest,
    ) -> Result<TaskContextView, SessionRefusal> {
        self.require_draft_slot()?;
        self.invalidate_drafts();
        let shared = self.opened_store()?;
        let result =
            soul_memory::select_task_context(&*hold(&shared), request.candidate_ids, &request.task)
                .map_err(|_| memory_context::changed())?;
        let context_id = (result.assessment.status == TaskStatus::Matched).then(Uuid::now_v7);
        let preview = result.selection.preview();
        self.held_memory = context_id.map(|id| memory_context::HeldMemory {
            id,
            revision: self.draft_revision,
            selection: result.selection,
            assistance: None,
        });
        Ok(TaskContextView {
            memory: MemoryContextView {
                context_id,
                preview,
            },
            assessment: result.assessment,
        })
    }
}
