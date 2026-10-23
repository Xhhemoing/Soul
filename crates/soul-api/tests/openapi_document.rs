#[test]
fn checked_in_openapi_is_generated_from_the_current_rust_dtos() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/architecture/local-api.openapi.json");
    let actual = std::fs::read_to_string(path).expect("run the offline export_openapi example");
    assert_eq!(
        actual.replace("\r\n", "\n"),
        soul_api::openapi::json().unwrap()
    );
    let document = soul_api::openapi::document();
    assert_eq!(document.paths.paths.len(), 16);
    assert!(document.paths.paths.contains_key("/v1/status"));
    assert!(document.paths.paths.contains_key("/v1/people"));
    assert!(document
        .paths
        .paths
        .contains_key("/v1/memories/{memory_id}"));
    for path in [
        "/v1/imports/prepare",
        "/v1/imports/{operation_id}/confirm",
        "/v1/imports/{operation_id}/discard",
        "/v1/imports/{operation_id}",
        "/v1/scoped/draft",
        "/v1/scoped/propose",
        "/v1/scoped/confirm",
        "/v1/scoped/{proposal_id}/discard",
        "/v1/scoped/status",
        "/v1/memories/{memory_id}/forget-preview",
        "/v1/memories/{memory_id}/forget-confirm",
        "/v1/memories/{memory_id}/state",
        "/v1/events/snapshot",
    ] {
        assert!(
            document.paths.paths.contains_key(path),
            "every allowlisted operation must be documented"
        );
    }
}
