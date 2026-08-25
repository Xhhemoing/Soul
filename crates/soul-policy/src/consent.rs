//! What the user has actually agreed to. Everything starts off.
//!
//! PRODUCT_LOCK: "采集默认关。向导不弹一堆权限". AC-02 and AC-09 both rest on
//! that being a property of the type rather than something a first-run wizard
//! remembers to write, so [`ConsentLedger::default`] grants nothing and there
//! is no constructor that grants anything.
//!
//! Consent is a closed enum, not a string, so a capability cannot be invented
//! at a call site and silently default to allowed. Every change is meant to
//! leave an `consent.grant` audit entry; [`ConsentLedger::grant`] returns the
//! [`AuditContent`] for it so the caller cannot forget what to write, only
//! where to write it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use soul_schema::audit::{AuditAction, AuditDecision};

use crate::audit::{AuditContent, ReasonCode};

/// The capabilities a user can turn on. One per thing that observes, stores or
/// transmits something on their behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentTopic {
    /// Foreground application duration. The only collector in v0.1.
    CollectForegroundApp,
    /// Reading an export file the user chose.
    ImportSocialExport,
    /// Sending a generation request to the user's own endpoint.
    UseUserEndpoint,
    /// Including one message's original prose in one request, after a second
    /// confirmation. Standing consent for the *ability*; each use still needs
    /// its own two-step confirmation in the redactor.
    ThirdPartyBodyExemption,
    /// Letting the research track read derived and aggregate data.
    ResearchPreview,
}

impl ConsentTopic {
    pub const ALL: &'static [ConsentTopic] = &[
        ConsentTopic::CollectForegroundApp,
        ConsentTopic::ImportSocialExport,
        ConsentTopic::UseUserEndpoint,
        ConsentTopic::ThirdPartyBodyExemption,
        ConsentTopic::ResearchPreview,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            ConsentTopic::CollectForegroundApp => "collect.foreground_app",
            ConsentTopic::ImportSocialExport => "import.social_export",
            ConsentTopic::UseUserEndpoint => "egress.user_endpoint",
            ConsentTopic::ThirdPartyBodyExemption => "egress.third_party_body_exemption",
            ConsentTopic::ResearchPreview => "research.preview",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentState {
    /// Never asked, or asked and refused. Indistinguishable on purpose: both
    /// mean "do not do this".
    #[default]
    Withheld,
    Granted,
}

/// One record, so the UI can say when a permission was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub state: ConsentState,
    /// Seconds since the Unix epoch when the state last changed.
    pub changed_at_unix_seconds: i64,
}

/// The whole of what the user has agreed to.
///
/// Serializable so WP09 can persist it next to the configuration; there is no
/// deserialization path that turns an absent entry into a grant.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentLedger {
    #[serde(default)]
    granted: BTreeMap<String, ConsentRecord>,
}

impl ConsentLedger {
    /// Nothing granted. This is also `Default`, and the two must stay the same
    /// thing; `tests/consent.rs` asserts it.
    pub fn closed() -> ConsentLedger {
        ConsentLedger::default()
    }

    pub fn state(&self, topic: ConsentTopic) -> ConsentState {
        self.granted
            .get(topic.as_str())
            .map(|record| record.state)
            .unwrap_or(ConsentState::Withheld)
    }

    pub fn is_granted(&self, topic: ConsentTopic) -> bool {
        self.state(topic) == ConsentState::Granted
    }

    pub fn record(&self, topic: ConsentTopic) -> Option<ConsentRecord> {
        self.granted.get(topic.as_str()).copied()
    }

    /// Turn a capability on, and describe the audit entry that says so.
    pub fn grant(&mut self, topic: ConsentTopic, at_unix_seconds: i64) -> AuditContent {
        self.set(topic, ConsentState::Granted, at_unix_seconds);
        AuditContent::new(AuditAction::ConsentGrant, AuditDecision::Allowed)
            .because(ReasonCode::ConsentGranted)
    }

    /// Turn it off again. Recorded as a `consent.grant` action with a
    /// `denied` decision: the frozen action vocabulary has no revoke verb, and
    /// inventing one would change a frozen contract for a state transition the
    /// decision field already expresses.
    pub fn revoke(&mut self, topic: ConsentTopic, at_unix_seconds: i64) -> AuditContent {
        self.set(topic, ConsentState::Withheld, at_unix_seconds);
        AuditContent::new(AuditAction::ConsentGrant, AuditDecision::Denied)
            .because(ReasonCode::ConsentRevoked)
    }

    fn set(&mut self, topic: ConsentTopic, state: ConsentState, at_unix_seconds: i64) {
        self.granted.insert(
            topic.as_str().to_owned(),
            ConsentRecord {
                state,
                changed_at_unix_seconds: at_unix_seconds,
            },
        );
    }

    /// Everything currently on. Empty on a fresh install.
    pub fn granted_topics(&self) -> Vec<ConsentTopic> {
        ConsentTopic::ALL
            .iter()
            .copied()
            .filter(|topic| self.is_granted(*topic))
            .collect()
    }

    /// Guard for a caller that is about to do something consent-gated.
    pub fn require(&self, topic: ConsentTopic) -> Result<(), ConsentMissing> {
        match self.is_granted(topic) {
            true => Ok(()),
            false => Err(ConsentMissing { topic }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{} has not been consented to", topic.as_str())]
pub struct ConsentMissing {
    pub topic: ConsentTopic,
}

impl ConsentMissing {
    pub fn reason_code(&self) -> ReasonCode {
        ReasonCode::ConsentMissing
    }
}
