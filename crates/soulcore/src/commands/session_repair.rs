//! N2 synthetic Session experiment. No product recovery surface.

use super::*;
use soul_store::local_ciphertext_repair::{capture_checked, repair, Held};

pub(super) struct HeldRepair {
    request_id: Uuid,
    memory_id: Uuid,
    original: Held,
}

impl std::fmt::Debug for HeldRepair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HeldRepair(<redacted>)")
    }
}

impl Session {
    fn prepare_repair(&mut self, memory_id: Uuid) -> Result<Uuid, SessionRefusal> {
        self.held_repair = None;
        self.require_draft_slot()?;
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let original = capture_checked(
            &mut store,
            memory_id,
            soul_memory::SelectionLimits::default().max_entry_bytes,
            |store| {
                soul_memory::read(store, memory_id)
                    .map(|_| ())
                    .map_err(|_| {
                        soul_store_api::types::StoreError::ContractViolation("N2_REJECTED".into())
                    })
            },
        )
        .map_err(|_| repair_refused())?;
        let request_id = Uuid::now_v7();
        self.held_repair = Some(HeldRepair {
            request_id,
            memory_id,
            original,
        });
        Ok(request_id)
    }

    fn confirm_repair(
        &mut self,
        request_id: Uuid,
        memory_id: Uuid,
    ) -> Result<usize, SessionRefusal> {
        if !self
            .held_repair
            .as_ref()
            .is_some_and(|held| held.request_id == request_id && held.memory_id == memory_id)
        {
            return Err(repair_refused());
        }
        let held = self.held_repair.take().expect("matched repair request");
        // Invalidation never releases an owned worker. Even a busy refusal
        // consumes this confirmation and prevents old draft/context reuse.
        self.invalidate_drafts();
        self.require_draft_slot()?;
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        repair(
            &mut store,
            held.original,
            soul_memory::SelectionLimits::default().max_entry_bytes,
        )
        .map_err(|_| repair_refused())
    }

    fn cancel_repair(&mut self, request_id: Uuid, memory_id: Uuid) {
        if self
            .held_repair
            .as_ref()
            .is_some_and(|held| held.request_id == request_id && held.memory_id == memory_id)
        {
            self.held_repair = None;
        }
    }

    pub(super) fn invalidate_repair(&mut self, memory_id: Uuid) {
        if self
            .held_repair
            .as_ref()
            .is_some_and(|held| held.memory_id == memory_id)
        {
            self.held_repair = None;
        }
    }
}

fn repair_refused() -> SessionRefusal {
    SessionRefusal {
        reason_code: "N2_REJECTED".into(),
        explanation: "N2 synthetic repair request refused".into(),
    }
}

#[path = "session_repair/tests.rs"]
mod tests;
