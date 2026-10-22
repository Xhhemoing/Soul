//! A purpose-limited activity dataset; no plaintext bodies or file writes.

use serde::{Deserialize, Serialize};
use soul_store_api::types::{StoreError, StoreResult};

use crate::SqlCipherStore;

mod query;
use query::SourceSnapshot;

fn refused(reason: &str) -> StoreError {
    StoreError::ContractViolation(reason.into())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchScope {
    pub from_day: String,
    pub to_day: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityBucket {
    pub event_kind: String,
    pub time_bucket_utc: String,
    pub aggregate_count: u64,
}

/// Counts at preparation time, including out-of-range candidates. Unrelated
/// later changes outside the scope do not invalidate a held publication.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchCounts {
    pub candidate_events: u64,
    pub included_events: u64,
    pub excluded_policy_events: u64,
    pub excluded_unavailable_events: u64,
    pub excluded_time_events: u64,
    pub out_of_range_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchDataset {
    pub scope: ResearchScope,
    pub rows: Vec<ActivityBucket>,
    pub counts: ResearchCounts,
}

#[derive(Clone)]
pub struct HeldResearchDataset {
    dataset: ResearchDataset,
    sources: Vec<SourceSnapshot>,
}

impl std::fmt::Debug for HeldResearchDataset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeldResearchDataset")
            .finish_non_exhaustive()
    }
}

impl HeldResearchDataset {
    pub fn dataset(&self) -> &ResearchDataset {
        &self.dataset
    }
}

#[derive(Debug)]
pub struct ResearchPublication<T> {
    pub value: T,
    pub transaction_closed: bool,
}

impl SqlCipherStore {
    pub fn prepare_research_retention(
        &self,
        scope: &ResearchScope,
    ) -> StoreResult<HeldResearchDataset> {
        query::prepare(self, scope)
    }

    pub fn publish_research_retention<T>(
        &mut self,
        held: &HeldResearchDataset,
        publish: impl FnOnce() -> StoreResult<T>,
    ) -> StoreResult<ResearchPublication<T>> {
        self.publish_research_with_store(held, |_| publish())
    }

    fn publish_research_with_store<T>(
        &mut self,
        held: &HeldResearchDataset,
        publish: impl FnOnce(&mut Self) -> StoreResult<T>,
    ) -> StoreResult<ResearchPublication<T>> {
        // Keep the irreversible result outside transact: a COMMIT failure must
        // not misreport a file already published by the caller as unpublished.
        let mut published = None;
        let outcome: StoreResult<()> = self.transact(|store| {
            let current_sources = query::source_snapshots(store, &held.dataset.scope)?;
            if current_sources != held.sources {
                return Err(refused("RESEARCH_SOURCE_CHANGED"));
            }
            published = Some(publish(store)?);
            Ok(())
        });
        match published {
            Some(value) => Ok(ResearchPublication {
                value,
                transaction_closed: outcome.is_ok(),
            }),
            None => {
                outcome?;
                Err(refused("RESEARCH_NOT_PUBLISHED"))
            }
        }
    }
}

#[cfg(test)]
mod tests;
