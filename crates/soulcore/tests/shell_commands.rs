//! WP09, core side: the three things the desktop shell may ask for.
//!
//! The shell's own tests are in `apps/desktop` and run under vitest against a
//! stubbed IPC. These are the ones that hold even if the WebView is rewritten:
//! AC-02 (a finished wizard leaves everything off) and AC-22 (the cloud switch
//! answers with a sentence and changes nothing).

use std::path::PathBuf;

use soulcore::commands::shell::{
    authorize_root, authorized_roots, cloud_toggle, complete_wizard, config_snapshot, wall_clock,
    CloudNotice, ConfigSnapshot, KeyProtection, RootRefusedReason, Session, WizardAnswers,
    WizardRefused, CLOUD_NOT_YET_AVAILABLE_LABEL, DESKTOP_BINARY_NAME,
    KEY_FILE_NOT_PROTECTED_EXPLANATION, NO_STORE_OPENED_EXPLANATION,
};
use soulcore::commands::store::{
    open_store_choosing_keys, open_store_for_session, KeyError, KeyProvider, KeyResult, SecretKey,
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

    let refusal = WizardRefused::CapabilityLeftOpen {
        open: opened
            .open_capabilities()
            .into_iter()
            .map(str::to_owned)
            .collect(),
    };
    assert!(
        refusal.to_string().contains("collect_enabled"),
        "a refusal has to name what was left on: {refusal}",
    );
}

#[test]
fn an_unacknowledged_wizard_does_not_finish() {
    let answers = WizardAnswers::default();
    assert!(!answers.acknowledged_defaults_are_off);
    assert_eq!(
        complete_wizard(&answers),
        Err(WizardRefused::NotAcknowledged),
    );
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
    assert_eq!(value["kek_protected"], serde_json::json!(false));
    assert_eq!(
        value["key_protection"],
        serde_json::json!(NO_STORE_OPENED_EXPLANATION),
        "a snapshot taken without a store must say so, not describe one",
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

// ------------------------------------------------------------- key material

/// The flag is computed, not written down. Both directions are asserted, so a
/// `kek_protected: false` that is really a hard-coded literal fails here.
#[test]
fn the_protection_flag_follows_the_provider_that_answered() {
    assert!(!KeyProtection::NoStoreOpened.kek_protected());
    assert!(!KeyProtection::UnprotectedKeyFile.kek_protected());
    assert!(
        KeyProtection::PlatformKeyStore.kek_protected(),
        "if this is false too, the field is a constant and proves nothing",
    );
}

/// No configuration, in any combination, can make the shell claim the key is
/// held by the platform. The exhaustive form matters: the value must not be
/// reachable from anything a user or a settings file can set.
#[test]
fn no_configuration_can_claim_the_key_is_protected() {
    for collect_enabled in [false, true] {
        for cloud_enabled in [false, true] {
            for llm_endpoint in [None, Some("http://127.0.0.1:11434/v1".to_owned())] {
                for authorized_roots in [Vec::new(), vec![PathBuf::from("/tmp")]] {
                    let config = Config {
                        collect_enabled,
                        cloud_enabled,
                        llm_endpoint: llm_endpoint.clone(),
                        authorized_roots,
                        ..Config::default()
                    };
                    for keys in [
                        KeyProtection::NoStoreOpened,
                        KeyProtection::UnprotectedKeyFile,
                    ] {
                        let snapshot = ConfigSnapshot::of_session(&config, keys);
                        assert!(
                            !snapshot.kek_protected,
                            "{config:?} with {keys:?} claimed the key is protected",
                        );
                        assert_eq!(snapshot.key_protection, keys.explanation());
                    }
                    assert!(!ConfigSnapshot::of(&config).kek_protected);
                }
            }
        }
    }
}

/// The shell opens one store, the platform provider refuses, and the fallback
/// is a key file the sentence can honestly describe.
#[test]
fn the_session_store_falls_back_to_a_key_file_and_says_so() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session = open_store_for_session(dir.path()).expect("the store opens");

    assert_eq!(session.key_protection(), KeyProtection::UnprotectedKeyFile);
    assert!(!session.key_protection().kek_protected());
    assert!(
        dir.path().join("soul.db").exists(),
        "the session is supposed to have opened a real database",
    );
    assert!(
        dir.path().join("soul-test-keys.bin").exists(),
        "the sentence names this file, so it had better be the one that appeared",
    );

    assert!(
        std::sync::Arc::ptr_eq(&session.handle(), &session.handle()),
        "every caller has to get the same handle; a second one is a second WAL",
    );

    // Reopening the same directory has to find the same key, or the fallback
    // would lock the user out of their own database on the second launch.
    drop(session);
    let again = open_store_for_session(dir.path()).expect("the store reopens");
    assert_eq!(again.key_protection(), KeyProtection::UnprotectedKeyFile);
}

/// A platform error that is not "this platform cannot" must not mint a
/// plaintext key file beside a database we never opened.
#[test]
fn a_platform_key_error_that_is_not_unsupported_does_not_fall_back() {
    #[derive(Debug)]
    struct BrokenPlatform(KeyError);

    impl KeyProvider for BrokenPlatform {
        fn database_key(&self) -> KeyResult<SecretKey> {
            Err(clone_key_error(&self.0))
        }

        fn key_encryption_key(&self) -> KeyResult<SecretKey> {
            Err(clone_key_error(&self.0))
        }

        fn describe(&self) -> String {
            "broken".to_owned()
        }
    }

    fn clone_key_error(error: &KeyError) -> KeyError {
        match error {
            KeyError::Unavailable(text) => KeyError::Unavailable(text.clone()),
            KeyError::Unsupported(text) => KeyError::Unsupported(text.clone()),
            KeyError::Io(text) => KeyError::Io(text.clone()),
            KeyError::Malformed {
                path,
                found,
                expected,
            } => KeyError::Malformed {
                path: path.clone(),
                found: *found,
                expected: *expected,
            },
        }
    }

    for error in [
        KeyError::Io("the blob could not be read".to_owned()),
        KeyError::Unavailable("the vault is locked".to_owned()),
        KeyError::Malformed {
            path: "keys.dpapi".to_owned(),
            found: 3,
            expected: 32,
        },
    ] {
        let dir = tempfile::tempdir().expect("temp dir");
        open_store_choosing_keys(dir.path(), &BrokenPlatform(error))
            .expect_err("a real key error is not a fallback");
        assert!(
            !dir.path().join("soul.db").exists(),
            "a failed open must not leave a database behind",
        );
        assert!(
            !dir.path().join("soul-test-keys.bin").exists(),
            "a failed open must not mint a plaintext seed",
        );
    }
}

/// `docs/SECURITY.md`: nothing may say the Windows KEK is protected until
/// DPAPI is wired up. This is that rule, applied to the words themselves.
#[test]
fn the_key_sentence_never_claims_a_protection_that_does_not_exist() {
    for claim in [
        "已受 DPAPI 保护",
        "已受保护",
        "已加密保护",
        "受到系统保护",
        "由 Windows 保护",
        "安全保管",
    ] {
        assert!(
            !KEY_FILE_NOT_PROTECTED_EXPLANATION.contains(claim),
            "the fallback sentence claims 「{claim}」",
        );
    }
    assert!(KEY_FILE_NOT_PROTECTED_EXPLANATION.contains("DPAPI"));
    assert!(KEY_FILE_NOT_PROTECTED_EXPLANATION.contains("明文"));
    assert!(
        KEY_FILE_NOT_PROTECTED_EXPLANATION.contains("soul-test-keys.bin"),
        "naming the file is what makes the warning actionable",
    );
}

#[test]
fn the_session_reports_only_the_protection_it_was_told_about() {
    let session = Session::new();
    assert_eq!(
        session.snapshot().key_protection,
        NO_STORE_OPENED_EXPLANATION,
    );

    session.opened_store_with(KeyProtection::UnprotectedKeyFile);
    let snapshot = session.snapshot();
    assert_eq!(snapshot.key_protection, KEY_FILE_NOT_PROTECTED_EXPLANATION);
    assert!(!snapshot.kek_protected);
}

// --------------------------------------------------------- authorised roots

#[test]
fn authorizing_a_directory_shows_up_in_the_snapshot() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session = Session::new();
    assert_eq!(session.snapshot().authorized_root_count, 0);
    assert!(session.authorized_roots().is_empty());

    let requested = dir.path().to_str().expect("a utf-8 temporary path");
    let snapshot = session
        .authorize_root(requested)
        .expect("the path is a real directory");

    assert_eq!(snapshot.authorized_root_count, 1);
    assert!(!snapshot.fully_closed);
    assert!(
        snapshot
            .open_capabilities
            .contains(&"authorized_roots".to_owned()),
        "an authorised directory is an open capability: {:?}",
        snapshot.open_capabilities,
    );

    let canonical = dir.path().canonicalize().expect("canonical form");
    assert_eq!(
        session.authorized_roots(),
        vec![canonical.display().to_string()],
        "the list reads back the resolved path, which is what WP11 will compare against",
    );
}

/// Resolution happens before comparison, so the same directory spelled another
/// way is a duplicate. WP11 refuses everything outside this list; a list with
/// four spellings of one directory in it is a list nobody can reason about.
#[test]
fn a_second_spelling_of_the_same_directory_is_not_a_second_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir
        .path()
        .to_str()
        .expect("a utf-8 temporary path")
        .to_owned();
    let leaf = dir
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a leaf name")
        .to_owned();

    let mut config = Config::default();
    authorize_root(&mut config, &path).expect("the first spelling is authorised");

    for spelling in [
        format!("{path}/"),
        format!("{path}/."),
        format!("{path}/../{leaf}"),
        format!("  {path}  "),
    ] {
        let refusal = authorize_root(&mut config, &spelling)
            .expect_err("the same directory must not be authorised twice");
        assert_eq!(
            refusal.reason,
            RootRefusedReason::AlreadyAuthorized,
            "for {spelling:?}",
        );
    }

    assert_eq!(config.authorized_roots.len(), 1);
}

#[test]
fn a_path_that_is_not_a_readable_directory_is_refused_with_a_sentence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("notes.txt");
    std::fs::write(&file, b"not a directory").expect("write the file");
    let missing = dir.path().join("no-such-directory");

    let cases = [
        (String::new(), RootRefusedReason::Empty),
        ("   ".to_owned(), RootRefusedReason::Empty),
        (missing.display().to_string(), RootRefusedReason::NotFound),
        (file.display().to_string(), RootRefusedReason::NotADirectory),
    ];

    let mut config = Config::default();
    for (requested, reason) in cases {
        let refusal = match authorize_root(&mut config, &requested) {
            Ok(added) => panic!("{requested:?} was authorised as {}", added.display()),
            Err(refusal) => refusal,
        };
        assert_eq!(refusal.reason, reason, "for {requested:?}");
        assert!(
            !refusal.message.trim().is_empty(),
            "a refusal the user reads has to be a sentence: {refusal:?}",
        );
        assert_eq!(
            refusal.to_string(),
            refusal.message,
            "the Display text and the text that crosses the IPC must be one string",
        );
    }

    assert!(
        config.authorized_roots.is_empty(),
        "a refused path must not be authorised anyway",
    );
}

/// The refusal crosses an IPC boundary, so its JSON shape is part of the
/// contract with `apps/desktop/src/core.ts`.
#[test]
fn a_refusal_serializes_to_a_reason_and_a_sentence() {
    let mut config = Config::default();
    let refusal = authorize_root(&mut config, "/definitely/not/here")
        .expect_err("a path that does not exist is refused");

    let value = serde_json::to_value(&refusal).expect("serialize");
    assert_eq!(value["reason"], serde_json::json!("not_found"));
    assert_eq!(
        value["message"],
        serde_json::json!(refusal.message),
        "the WebView renders this string; it must not have to build one",
    );
    assert_eq!(value.as_object().expect("an object").len(), 2);
}

/// AC-02 does not become a formality just because the settings page can write.
#[test]
fn a_finished_wizard_clears_a_directory_someone_authorised_first() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session = Session::new();
    session
        .authorize_root(dir.path().to_str().expect("a utf-8 temporary path"))
        .expect("the path is a real directory");
    assert_eq!(session.snapshot().authorized_root_count, 1);

    let snapshot = session
        .complete_wizard(&WizardAnswers {
            acknowledged_defaults_are_off: true,
        })
        .expect("the wizard finishes");

    assert!(snapshot.fully_closed);
    assert_eq!(snapshot.authorized_root_count, 0);
    assert!(snapshot.open_capabilities.is_empty());
    assert!(
        session.authorized_roots().is_empty(),
        "the snapshot said nothing is authorised, so nothing may be",
    );
}

/// Authorising is reachable from one function and the wizard is not it.
#[test]
fn the_wizard_has_no_way_to_authorise_anything() {
    let answers = WizardAnswers {
        acknowledged_defaults_are_off: true,
    };
    assert_eq!(
        serde_json::to_value(answers).expect("serialize"),
        serde_json::json!({ "acknowledged_defaults_are_off": true }),
        "one field, and it is not a path",
    );
    assert_eq!(
        complete_wizard(&answers)
            .expect("the wizard finishes")
            .authorized_root_count,
        0,
    );
}

#[test]
fn the_authorised_list_is_empty_until_someone_authorises_something() {
    assert!(authorized_roots(&Config::default()).is_empty());
    assert_eq!(config_snapshot().authorized_root_count, 0);
}

// ------------------------------------------------------ what the views read

/// The views need the configuration itself, not the snapshot: WP11 compares
/// against resolved paths, and a count of them is not something to compare.
#[test]
fn the_session_hands_the_views_the_directory_it_just_authorised() {
    let dir = tempfile::tempdir().expect("temp dir");
    let session = Session::new();
    assert!(session.config().authorized_roots.is_empty());

    session
        .authorize_root(dir.path().to_str().expect("a utf-8 temporary path"))
        .expect("the path is a real directory");

    let canonical = dir.path().canonicalize().expect("canonical form");
    assert_eq!(session.config().authorized_roots, vec![canonical]);
    assert!(
        session.config().llm_endpoint.is_none(),
        "nothing in the shell can write an endpoint, so the views never see one",
    );
}

/// The profile the draft view writes in the voice of is minted once and kept.
///
/// Both halves matter. Stable within a session is what makes a voice the user
/// sets apply to the next draft; different between sessions is what keeps it
/// from being a constant somebody wrote down, which would make every
/// installation draft as the same person.
#[test]
fn a_session_drafts_under_one_profile_and_the_next_one_under_another() {
    let session = Session::new();
    assert_eq!(session.draft_profile_id(), session.draft_profile_id());

    let another = Session::new();
    assert_ne!(session.draft_profile_id(), another.draft_profile_id());

    // Finishing the wizard replaces the configuration, not the identity: a
    // draft after the wizard has to read the same voice as one before it.
    let before = session.draft_profile_id();
    session
        .complete_wizard(&WizardAnswers {
            acknowledged_defaults_are_off: true,
        })
        .expect("the wizard finishes");
    assert_eq!(session.draft_profile_id(), before);
}

/// One reading, two units. The audit entry and the action check have to be
/// describing the same moment, so they come from the same call.
#[test]
fn the_wall_clock_answers_in_both_units_at_once() {
    let (now_ms, at_unix_seconds) = wall_clock();

    // 2020-01-01, which every machine that can run the test suite is past.
    assert!(
        at_unix_seconds > 1_577_836_800,
        "the clock reads {at_unix_seconds}"
    );
    assert_eq!(
        now_ms / 1_000,
        u64::try_from(at_unix_seconds).expect("a time after the epoch"),
        "the two units came from two readings",
    );
}
