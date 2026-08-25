//! Soul's permission surface.
//!
//! Every promise in `docs/PRODUCT_LOCK.md` that is about *not* doing something
//! is implemented here, and each one is a type rather than a check a caller
//! could forget:
//!
//! | Promise | Type |
//! |---|---|
//! | E0 has no code path; E1 is one exact origin | [`net_guard::NetGuard`], the only source of [`net_guard::EgressPermit`] |
//! | Third-party prose is placeheld by default | [`redactor::RedactedBody`], which only [`redactor::Redactor`] can produce |
//! | A single exemption is one-shot and not remembered | [`redactor::OneShotExemption`], consumed by value, with no state behind it |
//! | External content is never instruction | [`injection::UntrustedText`] and the two-slot body in [`e1`] |
//! | Unknown action, changed plan, replayed token are refused | [`hitl::check_action`] |
//! | The audit chain carries no prose | [`audit::AuditContent`] and [`audit::check`] |
//! | Collection is off until consented | [`consent::ConsentLedger`], whose `Default` grants nothing |
//! | Soul makes no medical claims | [`clinical::assert_non_clinical`] |
//!
//! This crate has no HTTP client, directly or transitively. The crate that
//! does — `soul-egress` — depends on this one, so the permission types sit
//! upstream of the socket and cannot be routed around.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod audit;
pub mod clinical;
pub mod clock;
pub mod consent;
pub mod e1;
pub mod hitl;
pub mod injection;
pub mod net_guard;
pub mod redactor;

pub use audit::{AuditContent, AuditContentError, ReasonCode};
pub use clinical::{assert_non_clinical, NonClinicalViolation};
pub use consent::{ConsentLedger, ConsentState, ConsentTopic};
pub use e1::E1RequestPlan;
pub use hitl::{
    check_action, ActionKind, ActionRequest, ApprovedAction, CapabilityScope, CapabilityToken,
    HitlDenial, PlanHash, RequestOrigin, TokenError, TokenIssuer,
};
pub use injection::{ExternalChannel, InjectionSignal, UntrustedText};
pub use net_guard::{
    EgressClass, EgressConfig, EgressDenied, EgressPermit, NetGuard, Origin, OriginError,
};
pub use redactor::{
    ExemptionRequest, KnownIdentifiers, OneShotExemption, RedactedBody, Redactor, Turn,
};
