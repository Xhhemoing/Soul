//! AC-09 and AC-10 against a desktop somebody is sitting at.
//!
//! Every other collection test drives [`FakeForegroundSource`]: it has to,
//! because a CI runner has no one switching between applications. But the two
//! criteria are written about a real machine — *switch applications ten times
//! with collection off and see zero foreground events; switch them with it on
//! and see at least one; turn it off and see nothing new within a second* —
//! and the part a fake cannot stand in for is `GetForegroundWindow` returning
//! whatever the author is actually looking at.
//!
//! So this is the instrument for that. It is not a product surface: no shell
//! command reaches it, it is reachable only through `soul-headless
//! collect-probe`, and everything it writes goes to a scratch store that is
//! deleted when it returns. What it collects, it collects because whoever ran
//! it passed a flag that says so, and it turns the consent back off before it
//! prints.
//!
//! Two phases, both timed by the operator switching windows:
//!
//! 1. **Consent off.** The gate refuses `start` outright, so nothing samples
//!    the desktop at all. The events in the store after the phase have to be
//!    the same as the events before it. That is AC-09.
//! 2. **Consent on.** The real source runs. At least one event has to appear,
//!    or the phase proves nothing — a probe that reported "0 events, all
//!    good" on a machine where collection was broken would be worse than no
//!    probe. Then consent is revoked *without* stopping the collector, and the
//!    count is read twice a second apart: the collector has to notice on its
//!    own, and the second reading has to equal the first. That is AC-10.
//!
//! [`crate::netwatch`] watches the whole thing, because a collector that
//! reported an application name to somebody would do it here and nowhere a
//! Linux test could see.
//!
//! [`FakeForegroundSource`]: soul_collect::FakeForegroundSource

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use soul_policy::consent::ConsentState;
use soul_store::SqlCipherStore;
use soul_store_api::types::EventFilter;
use soul_store_api::{AuditLog, EventStore, SoulStore};

use crate::commands::{collect as collect_commands, store as store_commands};
use crate::headless::{EgressFindings, HeadlessError};
use crate::netwatch;

/// Key material for the probe's own store. The user's library is never opened.
const SCRATCH_SEED: &str = "soul collect probe, scratch store";

/// How long after a revocation the collector is allowed to still be writing.
/// PRODUCT_LOCK says one second, and `soul_collect::STOP_BUDGET` is that
/// second; the reading is taken after it has elapsed.
const SETTLE: Duration = Duration::from_millis(1_200);

/// How long the operator gets to switch applications, per phase.
pub const DEFAULT_PHASE: Duration = Duration::from_secs(20);

/// What one phase saw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseReport {
    pub name: String,
    pub consent: String,
    pub seconds: u64,
    /// Events in the store when the phase ended, minus the events in it when
    /// the phase began. Never a name, and never a sample: what is being
    /// counted is rows.
    pub events_written: usize,
    pub verdict: String,
}

/// What one probe run found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeReport {
    pub ok: bool,
    /// The source this machine has, by its fixed label. `None` on a platform
    /// with no collector, which is every platform but Windows.
    pub source: Option<String>,
    pub phases: Vec<PhaseReport>,
    /// Read one second after consent was revoked, and again a second later.
    /// AC-10 is that these are equal.
    pub events_after_revocation: usize,
    pub events_a_second_later: usize,
    pub consent_left_granted: bool,
    pub egress: EgressFindings,
    pub audit_entries: usize,
    pub audit_chain_verified: bool,
}

/// Run both phases, and clean up after them.
pub fn run(phase: Duration) -> Result<ProbeReport, HeadlessError> {
    let scratch = crate::headless::scratch_dir_named("soul-collect-probe")?;
    let outcome = run_in(&scratch, phase);
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// The probe, against a directory the caller owns.
pub fn run_in(scratch: &Path, phase: Duration) -> Result<ProbeReport, HeadlessError> {
    let watch = netwatch::Watch::start();
    let outcome = probe(scratch, phase);
    let observed = watch.stop();

    let passed = outcome?;
    Ok(ProbeReport {
        ok: true,
        source: passed.source,
        phases: passed.phases,
        events_after_revocation: passed.events_after_revocation,
        events_a_second_later: passed.events_a_second_later,
        consent_left_granted: passed.consent_left_granted,
        egress: crate::headless::egress_findings(observed)?,
        audit_entries: passed.audit_entries,
        audit_chain_verified: true,
    })
}

/// What the phases proved, before the network observation is folded in.
struct ProbeOutcome {
    source: Option<String>,
    phases: Vec<PhaseReport>,
    events_after_revocation: usize,
    events_a_second_later: usize,
    consent_left_granted: bool,
    audit_entries: usize,
}

fn fail(detail: impl Into<String>) -> HeadlessError {
    HeadlessError {
        step: "collect-probe",
        detail: detail.into(),
    }
}

fn at<T, E: std::fmt::Display>(result: Result<T, E>) -> Result<T, HeadlessError> {
    result.map_err(|error| fail(error.to_string()))
}

fn count(store: &Arc<Mutex<SqlCipherStore>>) -> Result<usize, HeadlessError> {
    let guard = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(at(guard.list_events(&EventFilter::all()))?.len())
}

fn probe(scratch: &Path, phase: Duration) -> Result<ProbeOutcome, HeadlessError> {
    let store = at(store_commands::open_test_store(scratch, SCRATCH_SEED))?;
    let shared = collect_commands::share(store);
    let consent = collect_commands::ConsentHandle::closed();
    let mut phases = Vec::new();

    // Report the source before either phase: on a machine with no collector
    // there is nothing to measure, and saying so is the answer.
    let source_label = match collect_commands::platform_source() {
        Ok(source) => Some(source.describe().to_owned()),
        Err(error) => {
            return Err(fail(format!(
                "this machine has no foreground source ({error}); \
                 AC-09 and AC-10 are Windows criteria and this is where they are checked",
            )))
        }
    };

    // --- AC-09: nothing is collected while the gate is shut -----------------
    let before = count(&shared)?;
    eprintln!(
        "collect-probe: consent is OFF. Switch between applications for {} seconds.",
        phase.as_secs(),
    );
    std::thread::sleep(phase);

    let refused = collect_commands::start(
        at(collect_commands::platform_source())?,
        Arc::clone(&shared),
        &consent,
        Default::default(),
    );
    if refused.is_ok() {
        return Err(fail("collection started without consent"));
    }
    let after_closed = count(&shared)?;
    if after_closed != before {
        return Err(fail(format!(
            "AC-09: {} event(s) appeared with consent off",
            after_closed.saturating_sub(before),
        )));
    }
    phases.push(PhaseReport {
        name: "consent off".to_owned(),
        consent: format!("{:?}", consent.state(collect_commands::COLLECTION_TOPIC)),
        seconds: phase.as_secs(),
        events_written: 0,
        verdict: "AC-09: start refused, no event written".to_owned(),
    });

    // --- AC-10: it collects while granted, and stops when revoked -----------
    let granted_at = crate::headless::now_unix_seconds();
    let entry = consent.grant(collect_commands::COLLECTION_TOPIC, granted_at);
    let handle = at(collect_commands::start(
        at(collect_commands::platform_source())?,
        Arc::clone(&shared),
        &consent,
        Default::default(),
    ))?;
    eprintln!(
        "collect-probe: consent is ON. Switch between applications for {} seconds.",
        phase.as_secs(),
    );
    std::thread::sleep(phase);

    let while_granted = count(&shared)?;
    if while_granted <= after_closed {
        return Err(fail(
            "AC-10: consent was granted and the collector wrote nothing. \
             Either no application was switched to during the phase, or the \
             foreground source is not seeing this desktop",
        ));
    }

    // Revoke, and do not stop the collector: whether it notices on its own is
    // the thing being measured.
    consent.revoke(
        collect_commands::COLLECTION_TOPIC,
        crate::headless::now_unix_seconds(),
    );
    std::thread::sleep(SETTLE);
    let after_revocation = count(&shared)?;
    std::thread::sleep(SETTLE);
    let a_second_later = count(&shared)?;
    if a_second_later != after_revocation {
        return Err(fail(format!(
            "AC-10: {} event(s) were written more than a second after consent was revoked",
            a_second_later.saturating_sub(after_revocation),
        )));
    }
    let stopped = at(collect_commands::stop(handle))?;
    if stopped.stopped_because != collect_commands::StopReason::ConsentWithdrawn {
        return Err(fail(format!(
            "the collector stopped for the wrong reason: {:?}",
            stopped.stopped_because,
        )));
    }
    phases.push(PhaseReport {
        name: "consent on".to_owned(),
        consent: "Granted, then revoked".to_owned(),
        seconds: phase.as_secs(),
        events_written: while_granted - after_closed,
        verdict: format!(
            "AC-10: {} event(s) while on, none after; the collector stopped itself ({:?})",
            while_granted - after_closed,
            stopped.stopped_because,
        ),
    });

    let mut store =
        at(Arc::try_unwrap(shared)
            .map_err(|_| "the collector kept a handle to the store".to_owned()))?
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // The grant is an audit entry the ledger hands back for whoever holds the
    // store to write. Nobody else is going to, and a chain missing the line
    // that says collection was switched on is not the chain AC-23 wants.
    at(soul_policy::audit::append(&mut store, entry, granted_at))?;
    at(store.verify_audit_chain())?;
    let audit = at(store.list_audit())?;
    at(store.flush())?;
    at(store.close())?;

    Ok(ProbeOutcome {
        source: source_label,
        phases,
        events_after_revocation: after_revocation,
        events_a_second_later: a_second_later,
        consent_left_granted: consent.state(collect_commands::COLLECTION_TOPIC)
            == ConsentState::Granted,
        audit_entries: audit.len(),
    })
}
