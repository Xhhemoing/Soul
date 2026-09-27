//! The only place an [`EgressPermit`] can come from.
//!
//! PRODUCT_LOCK grades every outbound attempt as E0 (vendor business traffic),
//! E1 (the endpoint the user typed in) or L (loopback), and DECISIONS D12 says
//! a runtime switch is not an acceptable way to close E0. So the guard is
//! built the other way round: there is no argument, no configuration value and
//! no feature flag that produces an E0 permit, because [`EgressClass`] as
//! decided here has only two inhabitants and neither of them is E0. A vendor
//! URL does not take a disabled branch — it falls out of [`NetGuard::authorize`]
//! as [`EgressDenied::NoCodePath`].
//!
//! [`EgressPermit`]'s fields are private to this module and it has no public
//! constructor, so `soul-egress` — a different crate — cannot fabricate one.
//! Holding a permit is therefore evidence that this function ran and said yes.

use std::fmt;

use crate::audit::ReasonCode;

/// Scheme plus host plus port, compared exactly.
///
/// "Exact origin" in PRODUCT_LOCK means precisely this: a permit for
/// `http://127.0.0.1:8080` does not authorize `http://127.0.0.1:8081`, and a
/// redirect that changes any of the three is a different origin.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origin {
    scheme: String,
    host: String,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("`{0}` has no scheme; an origin needs one")]
    NoScheme(String),
    #[error("scheme `{0}` is not http or https")]
    UnsupportedScheme(String),
    #[error("`{0}` has no host")]
    NoHost(String),
    #[error("`{0}` does not contain a port number")]
    BadPort(String),
    #[error("`{0}` carries credentials; an origin must not")]
    HasCredentials(String),
    /// An authority like `::1:11434`, whose colons cannot be split into host
    /// and port. Rejected rather than guessed at: `rsplit` would read it as
    /// host `::1` port `11434`, someone else's parser may read it as host
    /// `::1:11434`, and an origin two parsers disagree about is not "exact".
    #[error("`{0}` has an IPv6 host without brackets; write the address as [address] instead")]
    UnbracketedIpv6(String),
}

impl Origin {
    /// Parse the origin out of an absolute URL.
    ///
    /// Deliberately hand-written rather than delegated to a URL crate: this is
    /// the comparison the whole egress promise rests on, and it must mean the
    /// same thing here and in the redirect check inside `soul-egress`, which
    /// sees a already-parsed URL. Anything unusual is rejected instead of
    /// normalized.
    pub fn parse(url: &str) -> Result<Origin, OriginError> {
        let trimmed = url.trim();
        let (scheme, rest) = trimmed
            .split_once("://")
            .ok_or_else(|| OriginError::NoScheme(trimmed.to_owned()))?;
        let scheme = scheme.to_ascii_lowercase();
        let default_port = match scheme.as_str() {
            "http" => 80u16,
            "https" => 443u16,
            _ => return Err(OriginError::UnsupportedScheme(scheme)),
        };

        let authority = rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .to_owned();
        if authority.contains('@') {
            return Err(OriginError::HasCredentials(trimmed.to_owned()));
        }
        // Only brackets make a second colon unambiguous. Without this, an
        // IPv6 address typed bare would be split at its last colon and
        // silently become a host-and-port nobody intended.
        if !authority.starts_with('[') && authority.matches(':').count() > 1 {
            return Err(OriginError::UnbracketedIpv6(trimmed.to_owned()));
        }

        let (host, port) = split_host_port(&authority, default_port)
            .ok_or_else(|| OriginError::BadPort(trimmed.to_owned()))?;
        if host.is_empty() {
            return Err(OriginError::NoHost(trimmed.to_owned()));
        }

        Ok(Origin {
            scheme,
            host: host.to_ascii_lowercase(),
            port,
        })
    }

    pub fn new(scheme: &str, host: &str, port: u16) -> Origin {
        // Stored the way `parse` stores it: an IPv6 literal keeps its colons
        // and loses its brackets, so `[::1]` and `::1` are one origin here
        // and `Display` is the only place brackets are written.
        let host = host.trim_start_matches('[').trim_end_matches(']');
        Origin {
            scheme: scheme.to_ascii_lowercase(),
            host: host.to_ascii_lowercase(),
            port,
        }
    }

    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// True for `127.0.0.0/8`, `::1` and the `localhost` name.
    ///
    /// A name that merely ends in `localhost` (`evil.localhost`) is not
    /// loopback here: it resolves wherever its DNS says.
    pub fn is_loopback(&self) -> bool {
        let host = self.host.trim_start_matches('[').trim_end_matches(']');
        if host == "localhost" {
            return true;
        }
        if host == "::1" || host == "0:0:0:0:0:0:0:1" {
            return true;
        }
        match host.parse::<std::net::IpAddr>() {
            Ok(address) => address.is_loopback(),
            Err(_) => false,
        }
    }
}

impl fmt::Display for Origin {
    /// The origin as a URL prefix, byte-for-byte reparsable by [`Origin::parse`].
    ///
    /// This string is load-bearing: `E1RequestPlan::url` appends a path to it
    /// and hands the result to the HTTP client, and `PolicySession` re-parses
    /// that URL before authorizing it. An IPv6 host therefore goes back
    /// between the brackets it was parsed out of — written bare, its own
    /// colons read as a port split, and an `::1` host with a port came back
    /// out as a bare `::1:11434` authority no URL parser accepts.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let implicit = match self.scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => 0,
        };
        if self.host.contains(':') {
            write!(f, "{}://[{}]", self.scheme, self.host)?;
        } else {
            write!(f, "{}://{}", self.scheme, self.host)?;
        }
        if self.port != implicit {
            write!(f, ":{}", self.port)?;
        }
        Ok(())
    }
}

fn split_host_port(authority: &str, default_port: u16) -> Option<(&str, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        // IPv6 literal: the colons inside the brackets are part of the host.
        let (host, tail) = rest.split_once(']')?;
        return match tail {
            "" => Some((host, default_port)),
            _ => {
                let port = tail.strip_prefix(':')?;
                Some((host, port.parse().ok()?))
            }
        };
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host, port.parse().ok()?)),
        None => Some((authority, default_port)),
    }
}

/// The classes a permit can be issued for.
///
/// E0 is absent by construction, which is the whole point: `docs/SECURITY.md`
/// asks for "no code path", and a variant that no branch produces would still
/// be a shape the rest of the program could pattern-match on and grow into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgressClass {
    /// The OpenAI-compatible endpoint the user configured, and only that one.
    E1,
    /// Loopback. UI-to-core and local models.
    L,
}

impl EgressClass {
    /// The value `docs/schemas/audit.schema.json` records.
    pub fn as_audit_class(self) -> soul_schema::audit::EgressClass {
        match self {
            EgressClass::E1 => soul_schema::audit::EgressClass::E1,
            EgressClass::L => soul_schema::audit::EgressClass::L,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EgressDenied {
    /// Not loopback and not the configured endpoint: business egress, which
    /// v0.1 has no implementation for.
    #[error("{origin} is neither loopback nor the configured endpoint; E0 has no code path")]
    NoCodePath { origin: Origin },

    /// The user never entered an endpoint, so nothing may leave the machine.
    #[error("no endpoint is configured, so {origin} cannot be reached")]
    EndpointNotConfigured { origin: Origin },

    /// An endpoint exists but this is a different host, port or scheme.
    #[error("{origin} is not the configured endpoint {configured}")]
    WrongOrigin { origin: Origin, configured: Origin },

    #[error("the request target is unusable: {0}")]
    BadTarget(#[from] OriginError),
}

impl EgressDenied {
    /// The audit `reason_code` for this refusal. Never any prose.
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            EgressDenied::NoCodePath { .. } => ReasonCode::E0NoCodePath,
            EgressDenied::EndpointNotConfigured { .. } => ReasonCode::E1NotConfigured,
            EgressDenied::WrongOrigin { .. } => ReasonCode::E1OriginMismatch,
            EgressDenied::BadTarget(_) => ReasonCode::EgressTargetUnparsable,
        }
    }
}

/// Proof that [`NetGuard::authorize`] approved one target origin.
///
/// Not `Clone`, not `Serialize`, no public constructor, and every field
/// private to this module. `soul-egress` can read where it is allowed to
/// connect and nothing else; it cannot mint one, and neither can any other
/// crate in the workspace.
#[derive(Debug)]
pub struct EgressPermit {
    origin: Origin,
    class: EgressClass,
}

impl EgressPermit {
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    pub fn class(&self) -> EgressClass {
        self.class
    }

    /// True when `url` stays on the origin this permit was issued for. The
    /// redirect check in `soul-egress` is exactly this question.
    pub fn allows(&self, url: &str) -> bool {
        Origin::parse(url).is_ok_and(|origin| origin == self.origin)
    }
}

impl fmt::Display for EgressPermit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} permit for {}", self.class, self.origin)
    }
}

/// What the user configured, and nothing else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EgressConfig {
    /// Origin of the user's own OpenAI-compatible endpoint. `None` until they
    /// type one in, which is the shipped default.
    e1_endpoint: Option<Origin>,
}

impl EgressConfig {
    /// Nothing configured. Every non-loopback target is refused.
    pub fn closed() -> EgressConfig {
        EgressConfig { e1_endpoint: None }
    }

    /// Point E1 at the endpoint URL the user supplied.
    pub fn with_user_endpoint(url: &str) -> Result<EgressConfig, OriginError> {
        Ok(EgressConfig {
            e1_endpoint: Some(Origin::parse(url)?),
        })
    }

    pub fn e1_endpoint(&self) -> Option<&Origin> {
        self.e1_endpoint.as_ref()
    }
}

/// The single gate between Soul and the network.
#[derive(Debug, Clone)]
pub struct NetGuard {
    config: EgressConfig,
}

impl NetGuard {
    pub fn new(config: EgressConfig) -> NetGuard {
        NetGuard { config }
    }

    /// A guard with nothing configured: only loopback is reachable.
    pub fn closed() -> NetGuard {
        NetGuard::new(EgressConfig::closed())
    }

    pub fn config(&self) -> &EgressConfig {
        &self.config
    }

    /// Decide whether `url` may be contacted, and under which class.
    ///
    /// The configured endpoint is checked before loopback so that a user who
    /// runs a local model still gets an E1 permit for it, and the audit trail
    /// says E1 rather than L. Everything that is neither is refused; there is
    /// no third branch to disable.
    pub fn authorize(&self, url: &str) -> Result<EgressPermit, EgressDenied> {
        let origin = Origin::parse(url)?;

        if let Some(configured) = self.config.e1_endpoint.as_ref() {
            if &origin == configured {
                return Ok(EgressPermit {
                    origin,
                    class: EgressClass::E1,
                });
            }
        }

        if origin.is_loopback() {
            return Ok(EgressPermit {
                origin,
                class: EgressClass::L,
            });
        }

        Err(match self.config.e1_endpoint.as_ref() {
            None => EgressDenied::EndpointNotConfigured { origin },
            Some(configured) if configured.is_loopback() => EgressDenied::NoCodePath { origin },
            Some(configured) => EgressDenied::WrongOrigin {
                origin,
                configured: configured.clone(),
            },
        })
    }

    /// Authorize and require the E1 class, for callers that must not silently
    /// fall back to talking to something on loopback.
    pub fn authorize_e1(&self, url: &str) -> Result<EgressPermit, EgressDenied> {
        let permit = self.authorize(url)?;
        match permit.class {
            EgressClass::E1 => Ok(permit),
            EgressClass::L => Err(match self.config.e1_endpoint.as_ref() {
                None => EgressDenied::EndpointNotConfigured {
                    origin: permit.origin,
                },
                Some(configured) => EgressDenied::WrongOrigin {
                    origin: permit.origin,
                    configured: configured.clone(),
                },
            }),
        }
    }
}
