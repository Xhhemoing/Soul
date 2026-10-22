use super::*;
use soulcore::commands::collect::{CollectorConfig, FakeForegroundSource};
use std::time::{Duration, Instant};

#[test]
fn research_save_uses_real_handler_and_synthetic_collection_only() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path().join("database"));
    let source = FakeForegroundSource::showing("synthetic-first.exe").unwrap();
    session
        .grant_collect_consent_with_source(
            source.clone(),
            CollectorConfig::every(Duration::from_millis(5)),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut n = 0;
    while session.collect_status().events_collected == Some(0) && Instant::now() < deadline {
        source
            .switch_to(if n % 2 == 0 {
                "synthetic-second.exe"
            } else {
                "synthetic-first.exe"
            })
            .unwrap();
        n += 1;
        std::thread::sleep(Duration::from_millis(10));
    }
    session.revoke_collect_consent().unwrap();
    assert!(session.collect_status().events_collected.unwrap() > 0);
    let shell = Shell::with_session(directory.path().join("database"), session);
    let target = directory.path().join("research.zip");
    let request = json!({"request": {"scope": {"from_day": "2000-01-01", "to_day": "9999-12-31"}, "destination": target.to_str().unwrap()}});
    assert!(shell
        .invoke_from(
            "prepare_research_save",
            request.clone(),
            Some("https://untrusted.invalid")
        )
        .is_err());
    let preview = shell
        .invoke("prepare_research_save", request.clone())
        .unwrap();
    assert!(!target.exists());
    assert!(
        preview["dataset"]["counts"]["included_events"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(shell
        .invoke(
            "confirm_research_save",
            json!({"request_id": preview["request_id"]})
        )
        .is_err());
    let cancel = shell
        .invoke(
            "cancel_research_save",
            json!({"requestId": preview["request_id"]}),
        )
        .unwrap();
    assert_eq!(cancel["cancelled"], true);
    assert!(shell
        .invoke(
            "confirm_research_save",
            json!({"requestId": preview["request_id"]})
        )
        .is_err());
    let preview = shell.invoke("prepare_research_save", request).unwrap();
    let receipt = shell.invoke("confirm_research_save", json!({"requestId": preview["request_id"], "destination": directory.path().join("tampered.zip")})).unwrap();
    assert_eq!(receipt["written_to_disk"], true);
    assert!(target.exists());
    assert!(!directory.path().join("tampered.zip").exists());
    assert!(shell
        .invoke(
            "confirm_research_save",
            json!({"requestId": preview["request_id"]})
        )
        .is_err());
    let bytes = std::fs::read(&target).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-first.exe"));
    assert_eq!(
        shell.invoke("research_preview", json!({})).unwrap()["written_to_disk"],
        false
    );
    #[allow(deprecated)]
    drop(
        shell
            .webview
            .unmanage::<soul_desktop::commands::SessionState>(),
    );
}
