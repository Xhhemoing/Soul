//! Configuration, with every capability off until the user turns it on.
//!
//! AC-02 is about this type: after the first-run wizard, collection is off,
//! cloud is off, and there is no model endpoint. PRODUCT_LOCK also says the
//! wizard must not open with a wall of permission prompts, so "off" has to be
//! what [`Config::default`] means, not what a wizard remembers to write.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What the cloud toggle can be in v0.1.
///
/// There is no `Enabled` variant on purpose. v0.1 has no non-loopback code
/// path at all, so an enum that could represent one would be a lie the UI
/// could accidentally tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudState {
    /// Shown in the UI as "尚未启用". Toggling it changes no behaviour.
    #[default]
    NotYetAvailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Foreground application duration collection. Off until consented.
    pub collect_enabled: bool,

    /// Deep cloud analysis. There is no code path behind this in v0.1.
    pub cloud_enabled: bool,
    pub cloud_state: CloudState,

    /// The user's own OpenAI-compatible endpoint (egress class E1).
    /// `None` means no generation request can be made at all.
    pub llm_endpoint: Option<String>,

    /// Directories the user has authorised for read-only scanning. Anything
    /// outside this list is refused; v0.1 never writes inside it either.
    pub authorized_roots: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            collect_enabled: false,
            cloud_enabled: false,
            cloud_state: CloudState::NotYetAvailable,
            llm_endpoint: None,
            authorized_roots: Vec::new(),
        }
    }
}

impl Config {
    /// True when nothing has been switched on. A fresh install must satisfy
    /// this, and so must the state right after the first-run wizard.
    pub fn is_fully_closed(&self) -> bool {
        !self.collect_enabled
            && !self.cloud_enabled
            && self.cloud_state == CloudState::NotYetAvailable
            && self.llm_endpoint.is_none()
            && self.authorized_roots.is_empty()
    }

    /// Why the configuration is not closed, for a readable assertion failure.
    pub fn open_capabilities(&self) -> Vec<&'static str> {
        let mut open = Vec::new();
        if self.collect_enabled {
            open.push("collect_enabled");
        }
        if self.cloud_enabled {
            open.push("cloud_enabled");
        }
        if self.llm_endpoint.is_some() {
            open.push("llm_endpoint");
        }
        if !self.authorized_roots.is_empty() {
            open.push("authorized_roots");
        }
        open
    }
}
