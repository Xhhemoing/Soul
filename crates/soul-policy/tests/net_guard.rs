//! The egress guard: E0 has no code path, E1 is one exact origin, L is
//! loopback.
//!
//! These cases live in `tests/` rather than beside the code because they have
//! to name hosts, and `xtask e0-audit` scans `crates/**/src` for URL literals.
//! Keeping them here means the guard's own source stays free of the thing it
//! exists to refuse, which is a property worth having on its own.

use soul_policy::audit::ReasonCode;
use soul_policy::net_guard::{
    EgressClass, EgressConfig, EgressDenied, NetGuard, Origin, OriginError,
};

/// Hosts a vendor build would talk to. `.invalid` is reserved by RFC 2606 and
/// can never resolve, so a mistake here cannot become a real connection.
const VENDOR_HOSTS: &[&str] = &[
    "https://telemetry.example.invalid/collect",
    "https://update.example.invalid/latest",
    "https://license.example.invalid/check",
    "https://api.example.invalid/v1/chat/completions",
    "http://analytics.example.invalid/beacon",
];

fn configured() -> NetGuard {
    NetGuard::new(EgressConfig::with_user_endpoint("http://127.0.0.1:8080/v1").expect("config"))
}

// ------------------------------------------------------------- origins ---

#[test]
fn an_origin_is_scheme_host_and_port() {
    let origin = Origin::parse("http://127.0.0.1:8080/v1/chat/completions").expect("parse");
    assert_eq!(origin.scheme(), "http");
    assert_eq!(origin.host(), "127.0.0.1");
    assert_eq!(origin.port(), 8080);
    assert_eq!(origin.to_string(), "http://127.0.0.1:8080");
}

#[test]
fn the_default_port_comes_from_the_scheme_and_a_scheme_change_is_an_origin_change() {
    assert_eq!(
        Origin::parse("http://example.invalid")
            .expect("http")
            .port(),
        80,
    );
    assert_eq!(
        Origin::parse("https://example.invalid")
            .expect("https")
            .port(),
        443,
    );
    assert_ne!(
        Origin::parse("http://example.invalid").expect("http"),
        Origin::parse("https://example.invalid").expect("https"),
    );
    assert_ne!(
        Origin::parse("http://127.0.0.1:1").expect("one"),
        Origin::parse("http://127.0.0.1:2").expect("two"),
    );
    assert_eq!(
        Origin::parse("http://127.0.0.1:80").expect("explicit"),
        Origin::parse("http://127.0.0.1").expect("implicit"),
    );
}

#[test]
fn unusable_targets_are_rejected_rather_than_normalized() {
    assert!(matches!(
        Origin::parse("http://user:pass@127.0.0.1:1/x"),
        Err(OriginError::HasCredentials(_)),
    ));
    assert!(matches!(
        Origin::parse("ftp://127.0.0.1"),
        Err(OriginError::UnsupportedScheme(_)),
    ));
    assert!(matches!(
        Origin::parse("127.0.0.1:1"),
        Err(OriginError::NoScheme(_)),
    ));
    assert!(matches!(
        Origin::parse("http://127.0.0.1:not-a-port"),
        Err(OriginError::BadPort(_)),
    ));
    assert!(matches!(
        Origin::parse("http:///v1"),
        Err(OriginError::NoHost(_)),
    ));
}

/// The Wave 1 IPv6 regression. `Display` used to drop the brackets, so the
/// exact string every request URL is built from — `format!("{origin}/v1/…")`
/// in `E1RequestPlan::url` and in `PolicySession::e1_generate_for` — was
/// unparsable for any IPv6 endpoint: `http://[::1]:11434` came back out as
/// `http://::1:11434`. An origin's `Display` must reparse as itself.
#[test]
fn an_ipv6_origin_survives_its_own_display() {
    let explicit = Origin::parse("http://[::1]:11434/v1").expect("bracketed IPv6 parses");
    assert_eq!(explicit.host(), "::1");
    assert_eq!(explicit.port(), 11434);
    assert_eq!(explicit.to_string(), "http://[::1]:11434");
    assert_eq!(Origin::parse(&explicit.to_string()).expect("its display reparses"), explicit);

    let implicit = Origin::parse("https://[::1]/v1").expect("default-port IPv6 parses");
    assert_eq!(implicit.port(), 443);
    assert_eq!(implicit.to_string(), "https://[::1]");
    assert_eq!(Origin::parse(&implicit.to_string()).expect("its display reparses"), implicit);
}

/// The product path behind the display fix: the guard is asked about the URL
/// that was built by appending the fixed path to the configured origin's own
/// `Display`, and for an IPv6 endpoint that question must come back E1 rather
/// than unparsable.
#[test]
fn the_request_url_built_from_an_ipv6_endpoint_is_authorized() {
    let guard = NetGuard::new(
        EgressConfig::with_user_endpoint("http://[::1]:11434/v1").expect("IPv6 configures"),
    );
    let endpoint = guard.config().e1_endpoint().expect("configured").clone();
    let url = format!("{endpoint}/v1/chat/completions");
    let permit = guard.authorize_e1(&url).expect("the rebuilt URL is the same origin");
    assert_eq!(permit.origin(), &endpoint);
    assert_eq!(permit.class(), EgressClass::E1);
}

/// A bare IPv6 authority is ambiguous — its own colons read as port splits —
/// so it is refused with a sentence that says how to write it, rather than
/// silently split at the last colon into a host and port nobody meant.
#[test]
fn an_ipv6_authority_without_brackets_is_rejected_rather_than_misread() {
    for url in ["http://::1:11434/v1", "http://::1", "https://fe80::1:2/v1"] {
        assert!(matches!(Origin::parse(url), Err(OriginError::UnbracketedIpv6(_))), "{url}");
    }
}

#[test]
fn loopback_is_the_address_not_the_suffix() {
    for loopback in [
        "http://127.0.0.1:1",
        "http://127.9.9.9:1",
        "http://localhost:1",
        "http://[::1]:1",
    ] {
        assert!(
            Origin::parse(loopback).expect(loopback).is_loopback(),
            "{loopback}",
        );
    }
    for elsewhere in [
        "http://evil.localhost:1",
        "http://localhost.example.invalid:1",
        "http://10.0.0.1:1",
        "http://0.0.0.0:1",
    ] {
        assert!(
            !Origin::parse(elsewhere).expect(elsewhere).is_loopback(),
            "{elsewhere} is not loopback",
        );
    }
}

// --------------------------------------------------------------- E0 ---

/// The point of D12: with nothing configured, business egress is refused, and
/// with an endpoint configured it is still refused. There is no third state
/// and no switch that changes the answer.
#[test]
fn business_egress_is_refused_in_every_configuration() {
    for guard in [NetGuard::closed(), configured()] {
        for url in VENDOR_HOSTS {
            let denied = guard
                .authorize(url)
                .err()
                .unwrap_or_else(|| panic!("{url} must never be reachable"));
            assert!(
                matches!(
                    denied.reason_code(),
                    ReasonCode::E0NoCodePath | ReasonCode::E1NotConfigured,
                ),
                "{url}: {denied:?}",
            );
        }
    }
}

/// The class enum has no E0 variant, so no downstream `match` can grow a
/// branch that performs one.
#[test]
fn the_class_a_permit_can_carry_has_no_e0() {
    let guard = configured();
    let permit = guard.authorize("http://127.0.0.1:8080/v1").expect("permit");
    match permit.class() {
        EgressClass::E1 => {}
        EgressClass::L => panic!("the configured origin must be classed E1"),
    }
    assert_eq!(
        permit.class().as_audit_class(),
        soul_schema::audit::EgressClass::E1,
    );
}

// --------------------------------------------------------------- E1 ---

#[test]
fn a_closed_guard_reaches_nothing_but_loopback() {
    let guard = NetGuard::closed();
    assert!(guard.config().e1_endpoint().is_none());

    let denied = guard
        .authorize("https://api.example.invalid/v1/chat/completions")
        .expect_err("nothing is configured");
    assert!(matches!(denied, EgressDenied::EndpointNotConfigured { .. }));
    assert_eq!(denied.reason_code(), ReasonCode::E1NotConfigured);

    assert_eq!(
        guard
            .authorize("http://127.0.0.1:9/x")
            .expect("loopback")
            .class(),
        EgressClass::L,
    );
    assert!(
        guard.authorize_e1("http://127.0.0.1:9/x").is_err(),
        "loopback is not the configured endpoint, so it is not an E1 permit",
    );
}

#[test]
fn the_configured_endpoint_is_matched_exactly() {
    let guard = configured();
    assert_eq!(
        guard
            .authorize("http://127.0.0.1:8080/v1/chat/completions")
            .expect("the configured origin")
            .class(),
        EgressClass::E1,
    );

    for near_miss in [
        "http://127.0.0.1:8081/v1/chat/completions",
        "https://127.0.0.1:8080/v1/chat/completions",
        "http://localhost:8080/v1/chat/completions",
    ] {
        assert!(
            guard.authorize_e1(near_miss).is_err(),
            "{near_miss} is a different origin",
        );
    }
}

/// A permit covers its own origin and nothing else. This is the comparison the
/// redirect refusal in `soul-egress` performs on every hop.
#[test]
fn a_permit_only_covers_its_own_origin() {
    let permit = configured()
        .authorize("http://127.0.0.1:8080/v1")
        .expect("permit");
    assert!(permit.allows("http://127.0.0.1:8080/v1/other"));
    assert!(!permit.allows("http://127.0.0.1:8081/v1"));
    assert!(!permit.allows("https://evil.example.invalid/v1"));
    assert!(!permit.allows("not a url"));
}

/// The net guard is the only source of a permit. `EgressPermit` has private
/// fields and no public constructor, so this is enforced by the compiler; the
/// assertion here is that nobody added a back door.
#[test]
fn the_guard_is_the_only_place_a_permit_is_built() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/net_guard.rs"),
    )
    .expect("read the net guard source");

    // Two struct literals, both inside `authorize`: one per class.
    let constructions = source.matches("Ok(EgressPermit {").count();
    assert_eq!(
        constructions, 2,
        "a permit should be built once per egress class; found {constructions}",
    );

    for back_door in [
        "pub fn new(origin",
        "impl Default for EgressPermit",
        "pub origin:",
        "pub class:",
    ] {
        assert!(
            !source.contains(back_door),
            "`{back_door}` would let another crate mint a permit",
        );
    }

    // And no other file in the crate constructs one.
    for entry in std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .expect("read the source directory")
    {
        let path = entry.expect("entry").path();
        if path.file_name().and_then(|n| n.to_str()) == Some("net_guard.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read");
        // A struct literal, not a return type: `-> &EgressPermit {` is an
        // accessor, and other modules are supposed to have those.
        let literals: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| line.contains("EgressPermit {") && !line.contains("->"))
            .collect();
        assert!(
            literals.is_empty(),
            "{} constructs an EgressPermit: {literals:#?}",
            path.display(),
        );
    }
}
