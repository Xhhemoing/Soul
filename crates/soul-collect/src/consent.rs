//! The gate, shared between the user interface and the collector thread.
//!
//! `soul-policy` owns what consent *is*; this is the handle that lets a running
//! collector see a revocation the moment it happens. Without it the collector
//! would be holding a copy of the ledger taken at start, and turning collection
//! off would only take effect the next time somebody restarted it.
//!
//! [`ConsentHandle::closed`] grants nothing, and so does `Default`. There is no
//! constructor that grants anything: the only way to a granted state is
//! [`ConsentHandle::grant`], which hands back the audit entry that says so.

use std::sync::{Arc, Mutex};

use soul_policy::audit::AuditContent;
use soul_policy::consent::{ConsentLedger, ConsentMissing, ConsentState, ConsentTopic};

use crate::lock::lock;

/// The one collector v0.1 has, and the topic that gates it.
pub const COLLECTION_TOPIC: ConsentTopic = ConsentTopic::CollectForegroundApp;

/// A consent ledger that several threads can read and one can change.
#[derive(Debug, Clone, Default)]
pub struct ConsentHandle {
    ledger: Arc<Mutex<ConsentLedger>>,
}

impl ConsentHandle {
    /// Nothing granted.
    pub fn closed() -> ConsentHandle {
        ConsentHandle::default()
    }

    /// Wrap a ledger that was loaded from configuration.
    pub fn from_ledger(ledger: ConsentLedger) -> ConsentHandle {
        ConsentHandle {
            ledger: Arc::new(Mutex::new(ledger)),
        }
    }

    pub fn is_granted(&self, topic: ConsentTopic) -> bool {
        lock(&self.ledger).is_granted(topic)
    }

    pub fn state(&self, topic: ConsentTopic) -> ConsentState {
        lock(&self.ledger).state(topic)
    }

    /// Turn a capability on, and return the audit entry the caller owes the
    /// chain. Whoever holds the open store writes it.
    pub fn grant(&self, topic: ConsentTopic, at_unix_seconds: i64) -> AuditContent {
        lock(&self.ledger).grant(topic, at_unix_seconds)
    }

    /// Turn it off again. A collector polling this handle stops on its next
    /// tick; see [`crate::runner`].
    pub fn revoke(&self, topic: ConsentTopic, at_unix_seconds: i64) -> AuditContent {
        lock(&self.ledger).revoke(topic, at_unix_seconds)
    }

    /// The guard the collector calls immediately before every write.
    pub fn require_collection(&self) -> Result<(), ConsentMissing> {
        lock(&self.ledger).require(COLLECTION_TOPIC)
    }

    /// A copy, for a caller that wants to persist or display the ledger.
    pub fn snapshot(&self) -> ConsentLedger {
        lock(&self.ledger).clone()
    }
}
