//! The one crate that owns an HTTP client.
//!
//! Everything about this crate is an exception being kept narrow:
//!
//! * `deny.toml` bans `reqwest` and `hyper` workspace-wide and readmits them
//!   only when `soul-egress` is the wrapper, so a second crate that adds an
//!   HTTP client fails `cargo deny`;
//! * `xtask e0-audit` treats this crate as the single gateway in the shipped
//!   dependency graph, and fails if any other shipped crate reaches an HTTP
//!   client — or if this one stops holding it, which would mean the exception
//!   has gone stale;
//! * [`send`] takes an [`E1RequestPlan`], which carries an
//!   [`EgressPermit`](soul_policy::EgressPermit) that only `soul-policy`'s net
//!   guard can mint and a [`RedactedBody`](soul_policy::RedactedBody) that only
//!   its redactor can produce. There is no other public entry point, no
//!   exposed client, and no URL-taking overload.
//!
//! The redirect policy is the part worth reading twice. `docs/SECURITY.md`
//! says a cross-origin redirect is refused, and a client that follows
//! redirects by default would turn a `302` from the user's endpoint into an
//! outbound request to wherever the response pointed — an E0 connection that
//! no code in the workspace asked for. So the policy compares each hop's
//! origin against the permit and refuses anything else.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

use std::time::Duration;

use soul_policy::e1::E1RequestPlan;
use soul_policy::net_guard::{EgressClass, Origin};
use soul_policy::ReasonCode;

/// Hops allowed within the permitted origin. Enough for a `/v1` to `/v1/`
/// tidy-up, not enough to be a chain.
const MAX_SAME_ORIGIN_REDIRECTS: usize = 3;

/// How long a generation request may take before it is abandoned.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// How long the connection itself may take to establish.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum EgressError {
    #[error("the request was redirected off {permitted}, which is not allowed")]
    CrossOriginRedirect { permitted: String },

    #[error("the endpoint answered {status}")]
    HttpStatus { status: u16, body_len: usize },

    #[error("the HTTP client could not be built: {0}")]
    ClientSetup(String),

    #[error("the request to the configured endpoint failed: {0}")]
    Transport(String),
}

impl EgressError {
    /// The audit `reason_code`. Never any prose from the wire.
    pub fn reason_code(&self) -> Option<ReasonCode> {
        match self {
            EgressError::CrossOriginRedirect { .. } => Some(ReasonCode::E1CrossOriginRedirect),
            _ => None,
        }
    }
}

/// What came back. The caller decides what to do with the text; this crate
/// does not parse it, and never treats it as instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1Response {
    pub status: u16,
    pub body: String,
}

/// Send one request to the origin the permit names.
///
/// The signature is the enforcement. There is no variant that takes a URL, no
/// exposed `Client`, and `E1RequestPlan` cannot be built without a permit and
/// a redacted body, so a caller who wants to reach an unapproved host has to
/// change `soul-policy` first — and that is where the review attention is.
pub fn send(plan: &E1RequestPlan) -> Result<E1Response, EgressError> {
    let permitted = plan.permit().origin().clone();
    let client = build_client(&permitted)?;

    let response = client
        .post(plan.url())
        .header("content-type", "application/json")
        .body(plan.encoded_body())
        .send()
        .map_err(|error| classify(error, &permitted))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| EgressError::Transport(redact_error(&error.to_string(), &permitted)))?;

    Ok(E1Response { status, body })
}

/// Which egress class the request will be recorded under.
pub fn class_of(plan: &E1RequestPlan) -> EgressClass {
    plan.permit().class()
}

fn build_client(permitted: &Origin) -> Result<reqwest::blocking::Client, EgressError> {
    let allowed = permitted.clone();
    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        let same_origin = Origin::parse(attempt.url().as_str())
            .map(|origin| origin == allowed)
            .unwrap_or(false);
        if !same_origin {
            // `attempt.error` surfaces as a redirect failure rather than a
            // silent stop, so a cross-origin `302` is loud.
            return attempt.error(CrossOrigin {
                permitted: allowed.to_string(),
            });
        }
        if attempt.previous().len() >= MAX_SAME_ORIGIN_REDIRECTS {
            return attempt.stop();
        }
        attempt.follow()
    });

    reqwest::blocking::Client::builder()
        .redirect(policy)
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        // No proxy: a system proxy would send the request to a host the guard
        // never approved.
        .no_proxy()
        .user_agent("soul/0.1")
        .build()
        .map_err(|error| EgressError::ClientSetup(error.to_string()))
}

/// The error a refused redirect carries, so the reason survives reqwest's
/// error chain and can be recognised again in [`classify`].
#[derive(Debug)]
struct CrossOrigin {
    permitted: String,
}

impl std::fmt::Display for CrossOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "refused a redirect away from the permitted origin {}",
            self.permitted,
        )
    }
}

impl std::error::Error for CrossOrigin {}

fn classify(error: reqwest::Error, permitted: &Origin) -> EgressError {
    if error.is_redirect() || chain_contains_cross_origin(&error) {
        return EgressError::CrossOriginRedirect {
            permitted: permitted.to_string(),
        };
    }
    EgressError::Transport(redact_error(&error.to_string(), permitted))
}

fn chain_contains_cross_origin(error: &reqwest::Error) -> bool {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = source {
        if current.downcast_ref::<CrossOrigin>().is_some() {
            return true;
        }
        source = current.source();
    }
    false
}

/// Keep whatever a failing endpoint said out of Soul's own error text.
///
/// A hostile endpoint controls the error message it produces, and error
/// strings end up in logs. Only the permitted origin and the error's own
/// classification survive.
fn redact_error(message: &str, permitted: &Origin) -> String {
    let permitted = permitted.to_string();
    let kind = if message.contains("timed out") || message.contains("timeout") {
        "timed out"
    } else if message.contains("connect") || message.contains("Connection") {
        "could not connect"
    } else {
        "failed"
    };
    format!("request to {permitted} {kind}")
}
