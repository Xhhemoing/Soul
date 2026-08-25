//! The E1 endpoint only ever comes from the user, and naming it is not the
//! same as contacting it.
//!
//! This test exists partly to keep `soul-testkit` a genuine dev-dependency of
//! `soulcore`: `xtask e0-audit` must stay clean even though the dev graph here
//! reaches an HTTP stack.

use soul_testkit::mock_llm::MockLlm;
use soulcore::Config;

#[test]
fn configuring_an_endpoint_does_not_contact_it() {
    let endpoint = MockLlm::start().expect("start the mock endpoint");

    let mut config = Config::default();
    assert_eq!(
        config.llm_endpoint, None,
        "a fresh install knows of no endpoint",
    );

    config.llm_endpoint = Some(endpoint.base_url());
    assert_eq!(
        endpoint.request_count(),
        0,
        "writing an endpoint into the configuration must not make a request",
    );

    let stored = config.llm_endpoint.as_deref().expect("just set");
    assert!(
        stored.starts_with("http://127.0.0.1:"),
        "the test endpoint must be loopback, got {stored}",
    );
    assert!(
        !config.is_fully_closed(),
        "once an endpoint is configured the install is no longer fully closed",
    );
}
