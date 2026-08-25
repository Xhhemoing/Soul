//! AC-02, core side: after the first-run wizard the configuration must have
//! collection off, cloud off, and no model endpoint.
//!
//! The wizard itself is WP09. What is pinned here is the thing the wizard is
//! not allowed to change: the default.

use std::path::PathBuf;

use soulcore::{CloudState, Config};

#[test]
fn defaults_are_all_off() {
    let config = Config::default();

    assert!(!config.collect_enabled, "collection must be off by default");
    assert!(!config.cloud_enabled, "the cloud must be off by default");
    assert_eq!(config.cloud_state, CloudState::NotYetAvailable);
    assert_eq!(
        config.llm_endpoint, None,
        "no endpoint means no generation request is possible at all",
    );
    assert!(
        config.authorized_roots.is_empty(),
        "no directory is authorised until the user picks one",
    );
    assert!(config.is_fully_closed());
    assert!(config.open_capabilities().is_empty());
}

#[test]
fn the_defaults_survive_a_json_round_trip() {
    let config = Config::default();
    let json = serde_json::to_string_pretty(&config).expect("serialize");
    let decoded: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, config);

    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(value["collect_enabled"], serde_json::json!(false));
    assert_eq!(value["cloud_enabled"], serde_json::json!(false));
    assert_eq!(value["llm_endpoint"], serde_json::Value::Null);
    assert_eq!(value["authorized_roots"], serde_json::json!([]));
    assert_eq!(value["cloud_state"], serde_json::json!("not_yet_available"));
}

/// An unrecognised key is a configuration mistake, not something to ignore.
#[test]
fn an_unknown_key_is_rejected() {
    let json = r#"{
        "collect_enabled": false,
        "cloud_enabled": false,
        "cloud_state": "not_yet_available",
        "llm_endpoint": null,
        "authorized_roots": [],
        "telemetry_enabled": true
    }"#;
    assert!(
        serde_json::from_str::<Config>(json).is_err(),
        "a key the core does not know about must not be silently accepted",
    );
}

/// `open_capabilities` has to name what is on, or the headless check is blind.
#[test]
fn open_capabilities_names_each_switch() {
    let config = Config {
        collect_enabled: true,
        cloud_enabled: true,
        llm_endpoint: Some("http://127.0.0.1:11434/v1".into()),
        authorized_roots: vec![PathBuf::from("/tmp/authorised")],
        ..Config::default()
    };
    assert!(!config.is_fully_closed());
    assert_eq!(
        config.open_capabilities(),
        vec![
            "collect_enabled",
            "cloud_enabled",
            "llm_endpoint",
            "authorized_roots",
        ],
    );
}

#[test]
fn the_headless_binary_prints_closed_defaults_and_exits_zero() {
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_soul-headless"));
    let output = std::process::Command::new(binary)
        .output()
        .expect("run soul-headless");

    assert!(
        output.status.success(),
        "soul-headless exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8 output");
    let printed: Config = serde_json::from_str(&stdout).expect("stdout is a Config");
    assert_eq!(printed, Config::default());
    assert!(printed.is_fully_closed());
}
