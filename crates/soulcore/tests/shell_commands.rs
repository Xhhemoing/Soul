//! WP09, core side: the three things the desktop shell may ask for.
//!
//! The shell's own tests are in `apps/desktop` and run under vitest against a
//! stubbed IPC. These are the ones that hold even if the WebView is rewritten:
//! AC-02 (a finished wizard leaves everything off) and AC-22 (the cloud switch
//! answers with a sentence and changes nothing).

use soul_policy::ReasonCode;
use soulcore::commands::session::SessionRefusal;
use soulcore::commands::shell::{
    cloud_toggle, complete_wizard, config_snapshot, CloudNotice, ConfigSnapshot, WizardAnswers,
    WizardRefused, CLOUD_NOT_YET_AVAILABLE_LABEL, DESKTOP_BINARY_NAME,
    WIZARD_NOT_ACKNOWLEDGED_NOTICE,
};
use soulcore::{CloudState, Config};

/// AC-02. The wizard's whole output is a configuration with nothing on.
#[test]
fn a_finished_wizard_leaves_every_capability_off() {
    let answers = WizardAnswers {
        acknowledged_defaults_are_off: true,
    };
    let snapshot = complete_wizard(&answers).expect("the wizard finishes");

    assert!(!snapshot.collect_enabled, "collection must be off");
    assert!(!snapshot.cloud.enabled, "the cloud must be off");
    assert_eq!(snapshot.cloud.state, CloudState::NotYetAvailable);
    assert!(
        !snapshot.llm_endpoint_configured,
        "the wizard must not configure a model endpoint",
    );
    assert_eq!(snapshot.authorized_root_count, 0);
    assert!(snapshot.fully_closed);
    assert!(
        snapshot.open_capabilities.is_empty(),
        "these were left on: {:?}",
        snapshot.open_capabilities,
    );
}

/// The wizard is not a formality that returns a constant: it reads the
/// configuration it is about to hand over and refuses an open one.
///
/// `complete_wizard` builds `Config::default()` itself, so the only way to
/// exercise the refusal is to run the same check the function runs against a
/// configuration that is not closed. If that check ever stops being able to
/// fail, this test says so.
#[test]
fn the_wizard_would_refuse_a_configuration_with_a_switch_on() {
    let opened = Config {
        collect_enabled: true,
        ..Config::default()
    };
    assert!(!opened.is_fully_closed());

    let refusal = WizardRefused::capability_left_open(&opened.open_capabilities());
    assert_eq!(refusal.reason_code, ReasonCode::ConsentMissing.as_str());
    assert!(
        refusal.explanation.contains("collect_enabled"),
        "a refusal has to name what was left on: {refusal}",
    );
}

#[test]
fn an_unacknowledged_wizard_does_not_finish() {
    let answers = WizardAnswers::default();
    assert!(!answers.acknowledged_defaults_are_off);
    assert_eq!(
        complete_wizard(&answers),
        Err(WizardRefused::not_acknowledged())
    );
}

/// The wizard refuses in the same shape as every other command, because the
/// shell has one renderer for a refusal and it reads two fields off it.
///
/// Deserializing into [`SessionRefusal`] is what makes this a shape assertion
/// rather than two spellings: that type is `deny_unknown_fields`, so a third
/// field here — or a tag, which is what this refusal used to cross the IPC as
/// — fails rather than arriving at a screen that renders `unavailable` and the
/// word `[object Object]`.
#[test]
fn a_refused_wizard_crosses_the_ipc_as_a_code_and_a_sentence() {
    let refusal = complete_wizard(&WizardAnswers::default()).expect_err("the wizard refuses");
    let value = serde_json::to_value(&refusal).expect("serialize");

    assert_eq!(
        value.as_object().map(|fields| fields.len()),
        Some(2),
        "the shell reads exactly two fields: {value}",
    );
    assert_eq!(value["reason_code"], serde_json::json!("ROUTINE"));
    assert_eq!(
        value["explanation"],
        serde_json::json!(WIZARD_NOT_ACKNOWLEDGED_NOTICE),
    );

    let read_as_any_refusal: SessionRefusal =
        serde_json::from_value(value).expect("the shared refusal shape reads it");
    assert_eq!(read_as_any_refusal.reason_code, refusal.reason_code);
    assert_eq!(read_as_any_refusal.explanation, refusal.explanation);
}

/// The sentence is Soul's own, and it is in the language the rest of the
/// wizard is written in.
#[test]
fn the_wizard_refuses_in_the_words_the_screen_speaks() {
    for refusal in [
        WizardRefused::not_acknowledged(),
        WizardRefused::capability_left_open(&["collect_enabled"]),
    ] {
        assert!(
            refusal
                .explanation
                .chars()
                .any(|point| ('\u{4e00}'..='\u{9fff}').contains(&point)),
            "the user reads this sentence: {refusal}",
        );
        assert_eq!(refusal.to_string(), refusal.explanation);
    }
}

/// AC-22. Pressing the switch returns the same notice either way.
#[test]
fn the_cloud_switch_says_the_same_thing_whichever_way_it_is_pressed() {
    let config = Config::default();
    let untouched = config_snapshot().cloud;
    let pressed_on = cloud_toggle(&config, true);
    let pressed_off = cloud_toggle(&config, false);

    assert_eq!(pressed_on, pressed_off);
    assert_eq!(pressed_on, untouched);
    assert_eq!(pressed_on.label, CLOUD_NOT_YET_AVAILABLE_LABEL);
    assert!(!pressed_on.enabled);
    assert!(
        !pressed_on.performs_network_request,
        "AC-22: pressing it must not put anything on the wire",
    );
    assert!(
        !pressed_on.explanation.is_empty(),
        "the switch has to explain itself, not just refuse",
    );
}

/// The label is the promise. Keep it in one place and keep it exact: the
/// desktop test asserts the same string from the other side of the IPC.
#[test]
fn the_label_is_the_words_the_lock_document_uses() {
    assert_eq!(CLOUD_NOT_YET_AVAILABLE_LABEL, "尚未启用");
}

/// There is no `Enabled` variant to deserialize into, so a configuration file
/// that claims the cloud is on cannot be read as one.
#[test]
fn no_configuration_can_claim_the_cloud_is_available() {
    let json = r#"{
        "collect_enabled": false,
        "cloud_enabled": false,
        "cloud_state": "enabled",
        "llm_endpoint": null,
        "authorized_roots": []
    }"#;
    assert!(serde_json::from_str::<Config>(json).is_err());
}

/// The snapshot crosses an IPC boundary, so its JSON shape is part of the
/// contract with `apps/desktop/src/core.ts`.
#[test]
fn the_snapshot_serializes_to_the_shape_the_webview_expects() {
    let snapshot = config_snapshot();
    let value = serde_json::to_value(&snapshot).expect("serialize");

    assert_eq!(value["collect_enabled"], serde_json::json!(false));
    assert_eq!(value["llm_endpoint_configured"], serde_json::json!(false));
    assert_eq!(value["authorized_root_count"], serde_json::json!(0));
    assert_eq!(value["fully_closed"], serde_json::json!(true));
    assert_eq!(value["open_capabilities"], serde_json::json!([]));
    assert_eq!(
        value["cloud"]["state"],
        serde_json::json!("not_yet_available")
    );
    assert_eq!(value["cloud"]["enabled"], serde_json::json!(false));
    assert_eq!(value["cloud"]["label"], serde_json::json!("尚未启用"));
    assert_eq!(
        value["cloud"]["performs_network_request"],
        serde_json::json!(false),
    );

    let decoded: ConfigSnapshot = serde_json::from_value(value).expect("round trip");
    assert_eq!(decoded, snapshot);
}

/// The endpoint the user typed does not travel to the WebView. Only the fact
/// that there is one.
#[test]
fn the_snapshot_carries_no_endpoint_string() {
    let configured = Config {
        llm_endpoint: Some("http://127.0.0.1:11434/v1".into()),
        ..Config::default()
    };
    let snapshot = ConfigSnapshot::of(&configured);
    assert!(snapshot.llm_endpoint_configured);
    assert!(!snapshot.fully_closed);

    let json = serde_json::to_string(&snapshot).expect("serialize");
    assert!(
        !json.contains("11434"),
        "the snapshot leaked the endpoint: {json}",
    );
}

/// PRODUCT_LOCK names the process `soul.exe`. The Tauri crate asserts its own
/// binary name against this constant; this is the other half of that pair.
#[test]
fn the_binary_is_named_the_way_the_lock_document_names_it() {
    assert_eq!(DESKTOP_BINARY_NAME, "soul");
}

/// `CloudNotice` has to be constructible only through the config, so no caller
/// can assemble one that claims something the build cannot do.
#[test]
fn a_notice_always_comes_from_a_configuration() {
    let notice: CloudNotice = cloud_toggle(&Config::default(), true);
    let json = serde_json::to_string(&notice).expect("serialize");
    assert!(json.contains("not_yet_available"));
}
