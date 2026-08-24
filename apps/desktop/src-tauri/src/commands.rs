//! The IPC surface, and nothing else.
//!
//! Every function here is one line long by design: it takes what the WebView
//! sent, hands it to `soulcore::commands`, and returns what came back. No
//! decision is made in this file, so there is nothing in it for a test to
//! catch — which is the point. The decisions have tests, in `soulcore`.
//!
//! One absence is deliberate. WP10's endpoint path is two calls —
//! `DraftSession::prepare` describes a request and `generate` runs it, with a
//! person in between — and this shell has no screen for that person yet. So
//! it binds the local half only: [`draft_reply`] takes the path that builds
//! no request body at all. When WP13 gives the shell a configuration to read
//! and a dialog to show, the other two calls get bound; until then there is
//! no way through this file to reach an endpoint, which is a stronger thing
//! to be able to say than "the button is hidden".

use std::sync::{Mutex, MutexGuard};

use soulcore::commands::draft::{self, DraftRefusalView, DraftSession, DraftValue};
use soulcore::commands::policy::PolicySession;
use soulcore::commands::shell::{self, CloudNotice, ConfigSnapshot, WizardAnswers, WizardRefused};
use soulcore::Config;
use tauri::State;

/// The configuration this session is running under.
///
/// `Config::default()` for now. Where a real installation reads it from — and
/// therefore what happens on the second launch — is WP13's question; keeping
/// it in managed state means answering it does not touch this file.
#[derive(Debug, Default)]
pub struct SessionConfig(pub Config);

#[tauri::command]
pub fn config_snapshot(config: State<'_, SessionConfig>) -> ConfigSnapshot {
    ConfigSnapshot::of(&config.0)
}

#[tauri::command]
pub fn complete_wizard(answers: WizardAnswers) -> Result<ConfigSnapshot, WizardRefused> {
    shell::complete_wizard(&answers)
}

#[tauri::command]
pub fn cloud_toggle(config: State<'_, SessionConfig>, requested_on: bool) -> CloudNotice {
    shell::cloud_toggle(&config.0, requested_on)
}

/// The drafting state this window is using.
///
/// Both halves are behind one lock because they have to agree: the two
/// sessions each hold a redactor, and a redactor that knew about a different
/// set of names would placehold different things. `draft::closed_session`
/// builds the pair, so the agreement is `soulcore`'s to keep rather than a
/// thing this file could get wrong.
#[derive(Debug)]
pub struct Drafting {
    draft: DraftSession,
    policy: PolicySession,
}

impl Default for Drafting {
    fn default() -> Drafting {
        let (draft, policy) = draft::closed_session();
        Drafting { draft, policy }
    }
}

impl Drafting {
    /// One paste in, one draft out. Nothing here decides anything: the origin,
    /// the clock and the reading of a paste as somebody else's words are all
    /// `soulcore`'s, and `tests/draft_commands.rs` is where they are checked.
    fn reply_to(&mut self, pasted: &str) -> Result<DraftValue, DraftRefusalView> {
        draft::draft_pasted(&self.draft, &mut self.policy, pasted)
            .map_err(|refusal| DraftRefusalView::of(&refusal))
    }
}

#[derive(Debug, Default)]
pub struct DraftingState(pub Mutex<Drafting>);

impl DraftingState {
    /// The pair, with a poisoned lock recovered rather than propagated.
    ///
    /// A panic in one draft must not take drafting away for the rest of the
    /// session: nothing in [`Drafting`] can be left half-written by one, since
    /// the only mutable thing in it is the prepared body this shell never
    /// builds. The honest recovery is to carry on.
    fn held(&self) -> MutexGuard<'_, Drafting> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Draft a reply to something the user pasted. Never sends it.
#[tauri::command]
pub fn draft_reply(
    drafting: State<'_, DraftingState>,
    pasted: String,
) -> Result<DraftValue, DraftRefusalView> {
    drafting.held().reply_to(&pasted)
}

/// What the drafting screen says before there is a draft on it.
///
/// Read over the IPC rather than written in TypeScript, for the reason WP09
/// gave the cloud notice the same treatment: it is a promise about what this
/// build does, and a promise kept in the interface is one the Rust tests
/// cannot check.
#[tauri::command]
pub fn draft_notices() -> DraftNotices {
    DraftNotices::of_this_build()
}

/// The sentences the drafting screen renders verbatim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftNotices {
    /// `soulcore`'s constant, not a paraphrase of it.
    pub not_sent: String,
    /// Always false. There is no command on this surface that sends anything,
    /// and `tests/command_surface.rs` is what keeps that list short.
    pub can_send: bool,
}

impl DraftNotices {
    fn of_this_build() -> DraftNotices {
        DraftNotices {
            not_sent: draft::NOT_SENT_NOTICE.to_owned(),
            can_send: false,
        }
    }
}

/// The command names the WebView is allowed to call.
///
/// Spelled out so `tests/command_surface.rs` can compare this list against
/// `apps/desktop/src/core.ts`, which is the only place the other side names
/// them. A command added to one and not the other is a failing test.
pub const COMMAND_NAMES: &[&str] = &[
    "config_snapshot",
    "complete_wizard",
    "cloud_toggle",
    "draft_reply",
    "draft_notices",
];
