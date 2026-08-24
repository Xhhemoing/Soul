//! Sample, gate, seal, append. In that order, every time.
//!
//! This is the whole pipeline WP07 owns, and it is written so that there is no
//! second way through it. [`Collector::poll_once`] is the only thing that reads
//! the source, [`Collector::record`] is the only thing that writes an event,
//! and the consent check happens in both — once before the sample is taken, and
//! again in the instant before the write. The second check is not redundant:
//! the user can revoke consent from another thread between the two, and the
//! promise in PRODUCT_LOCK is about what reaches the store, not about what the
//! collector intended when it woke up.
//!
//! Consent withheld also *discards* the session in flight rather than flushing
//! it. Turning collection off is not a request to write one more row.
//!
//! The audit entries are `collect.start` and `collect.stop`, one per run, with
//! a count of the events written. One entry per event would put a per-minute
//! trace of the user's day in a log that deliberately cannot be forgotten,
//! which is the opposite of what an audit chain without prose is for.

use std::sync::{Arc, Mutex};

use uuid::Uuid;

use soul_policy::audit::{append, AuditContent, ReasonCode};
use soul_schema::audit::{AuditAction, AuditCounts, AuditDecision};
use soul_schema::common::{
    ActorSubject, Derivation, EgressPolicy, Privacy, Purpose, Retention, SchemaVersion,
    SealedSubject, Subject, Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_store_api::types::SealRequest;
use soul_store_api::{AuditLog, BlobStore, EventStore};

use crate::consent::ConsentHandle;
use crate::error::{CollectError, CollectResult};
use crate::lock::lock;
use crate::session::{ForegroundSession, OpenSession, BODY_FIELD};
use crate::source::ForegroundSource;

/// Everything a collector needs from the store.
pub trait CollectSink: EventStore + BlobStore + AuditLog {}

impl<T: EventStore + BlobStore + AuditLog> CollectSink for T {}

/// What one poll did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    /// Collection is not consented to. Nothing was sampled, nothing was
    /// written, and any session in flight was dropped.
    ConsentWithheld,
    /// Nothing has focus: a locked session, or a process this build declines
    /// to identify.
    NothingInForeground,
    /// The same application as last time. The session keeps running.
    SameApp,
    /// The first application after a quiet stretch. Nothing to write yet.
    SessionStarted,
    /// A session ended and became an event.
    SessionRecorded { event_id: Uuid, duration_ms: u64 },
}

/// Counts a caller can show without opening the store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub events_written: u64,
    /// Sessions dropped unwritten, which is what a revocation does to the
    /// session in flight.
    pub sessions_discarded: u64,
    /// Polls that found consent withheld.
    pub consent_refusals: u64,
    pub source_errors: u64,
}

/// The foreground collector: one source, one gate, one store.
#[derive(Debug)]
pub struct Collector<F, S> {
    source: F,
    sink: Arc<Mutex<S>>,
    consent: ConsentHandle,
    content_key_id: Uuid,
    open: Option<OpenSession>,
    tally: Tally,
}

impl<F: ForegroundSource, S: CollectSink> Collector<F, S> {
    /// A collector with its own content key.
    ///
    /// One key per run, so "forget what Soul collected while it was on" is a
    /// single [`soul_store_api::forget::ForgetUnit::ContentKey`]. It is the
    /// coarsest unit that is still under the user's control; a finer one, such
    /// as a key per day, needs the key for a day to be found again after a
    /// restart, and that is a store-side lookup rather than a collector-side
    /// decision.
    pub fn new(source: F, sink: Arc<Mutex<S>>, consent: ConsentHandle) -> Collector<F, S> {
        Collector::with_content_key(source, sink, consent, Uuid::now_v7())
    }

    /// As [`Collector::new`], continuing an existing forget unit.
    pub fn with_content_key(
        source: F,
        sink: Arc<Mutex<S>>,
        consent: ConsentHandle,
        content_key_id: Uuid,
    ) -> Collector<F, S> {
        Collector {
            source,
            sink,
            consent,
            content_key_id,
            open: None,
            tally: Tally::default(),
        }
    }

    pub fn content_key_id(&self) -> Uuid {
        self.content_key_id
    }

    pub fn tally(&self) -> Tally {
        self.tally
    }

    pub fn describe_source(&self) -> &'static str {
        self.source.describe()
    }

    /// Take one sample and act on it.
    pub fn poll_once(&mut self, now_unix_millis: u64) -> CollectResult<Poll> {
        if !self.consent.is_granted(crate::consent::COLLECTION_TOPIC) {
            self.tally.consent_refusals += 1;
            self.discard_open();
            return Ok(Poll::ConsentWithheld);
        }

        let sampled = match self.source.sample() {
            Ok(sampled) => sampled,
            Err(error) => {
                self.tally.source_errors += 1;
                return Err(CollectError::Source(error));
            }
        };

        match (self.open.take(), sampled) {
            (None, None) => Ok(Poll::NothingInForeground),
            (None, Some(app)) => {
                self.open = Some(OpenSession::started(app, now_unix_millis));
                Ok(Poll::SessionStarted)
            }
            (Some(open), Some(app)) if open.app == app => {
                self.open = Some(open);
                Ok(Poll::SameApp)
            }
            (Some(open), next) => {
                let session = open.ended_at(now_unix_millis);
                let event_id = self.record(&session)?;
                if let Some(app) = next {
                    self.open = Some(OpenSession::started(app, now_unix_millis));
                }
                Ok(Poll::SessionRecorded {
                    event_id,
                    duration_ms: session.duration_ms(),
                })
            }
        }
    }

    /// End the session in flight and write it. What an orderly stop does.
    ///
    /// If consent went away while that session was running it is discarded
    /// instead, and quietly: a caller stopping a collector the user has already
    /// switched off has done nothing wrong, and there is nothing left to write.
    pub fn finish(&mut self, now_unix_millis: u64) -> CollectResult<Option<Uuid>> {
        if self.consent.require_collection().is_err() {
            self.discard_open();
            return Ok(None);
        }
        let Some(open) = self.open.take() else {
            return Ok(None);
        };
        let session = open.ended_at(now_unix_millis);
        self.record(&session).map(Some)
    }

    /// Throw the session in flight away. What a revocation does.
    pub fn discard_open(&mut self) {
        if self.open.take().is_some() {
            self.tally.sessions_discarded += 1;
        }
    }

    /// `collect.start`, written before the first sample is taken.
    pub fn audit_start(&mut self, at_unix_seconds: i64) -> CollectResult<Uuid> {
        self.consent.require_collection()?;
        let content = AuditContent::allowed(AuditAction::CollectStart, ReasonCode::ConsentGranted)
            .about(&[self.content_key_id]);
        self.append_audit(content, at_unix_seconds)
    }

    /// `collect.stop`, with the number of events the run produced.
    ///
    /// Written after collection has already stopped, and with a reason code
    /// rather than a sentence: the chain records that a run ended and how much
    /// it wrote, never what the user was doing.
    pub fn audit_stop(&mut self, reason: ReasonCode, at_unix_seconds: i64) -> CollectResult<Uuid> {
        let content = AuditContent::new(AuditAction::CollectStop, AuditDecision::Allowed)
            .because(reason)
            .about(&[self.content_key_id])
            .counting(AuditCounts {
                items: Some(self.tally.events_written),
                bytes: None,
            });
        self.append_audit(content, at_unix_seconds)
    }

    // ----------------------------------------------------------- internals ---

    /// Seal a session and append the event that points at it.
    ///
    /// The consent check on the first line is the gate AC-09 is about. It is
    /// here, rather than only at the top of the poll, because this is the last
    /// instruction before the store is touched.
    fn record(&mut self, session: &ForegroundSession) -> CollectResult<Uuid> {
        if let Err(missing) = self.consent.require_collection() {
            self.tally.consent_refusals += 1;
            self.tally.sessions_discarded += 1;
            return Err(CollectError::Consent(missing));
        }

        let event_id = Uuid::now_v7();
        let body = session.sealed_body().to_json_bytes()?;
        let mut sink = lock(&self.sink);

        let sealed = sink.seal(SealRequest::new(
            self.content_key_id,
            event_id,
            BODY_FIELD,
            SealedSubject::Owner,
            body,
        ))?;

        sink.append_event(SoulEvent {
            schema_version: SchemaVersion,
            event_id,
            ts: Timestamp::new(soul_policy::clock::rfc3339_utc(
                session.started_at_unix_seconds(),
            )),
            source: EventSource::CollectorForegroundApp,
            kind: EventKind::AppForeground,
            actor_subject: ActorSubject::Owner,
            // The ledger records a state and when it changed, not an
            // identifier, so there is nothing honest to put here yet.
            consent_id: None,
            privacy: collected_privacy(),
            body_ref: Some(sealed),
        })?;

        drop(sink);
        self.tally.events_written += 1;
        Ok(event_id)
    }

    fn append_audit(&mut self, content: AuditContent, at_unix_seconds: i64) -> CollectResult<Uuid> {
        let mut sink = lock(&self.sink);
        Ok(append(&mut *sink, content, at_unix_seconds)?)
    }
}

/// Privacy for a collected event.
///
/// The user's own data, raw, kept until they forget it, and going nowhere: E0
/// has no code path, E1 has no reason to see which applications someone used,
/// and v0.1 research is a local preview rather than an export. `purposes` names
/// research all the same, because the hourly rollup in `soul-store` reads these
/// rows and the purpose field is where that has to be declared.
fn collected_privacy() -> Privacy {
    Privacy {
        subject: Subject::Owner,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::SoulProfile, Purpose::Research],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy::default(),
    }
}
