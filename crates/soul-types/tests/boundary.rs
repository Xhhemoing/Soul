use serde_json::json;
use soul_types::{MemoryIdRequest, PeoplePageRequest, ReadErrorCode};

#[test]
fn requests_have_defaults_and_reject_unrecognized_fields_and_codes() {
    assert_eq!(
        serde_json::from_value::<PeoplePageRequest>(json!({})).unwrap(),
        PeoplePageRequest {
            limit: 50,
            offset: 0
        }
    );
    assert!(serde_json::from_value::<PeoplePageRequest>(json!({"limit": -1})).is_err());
    assert!(serde_json::from_value::<PeoplePageRequest>(json!({"root": "synthetic"})).is_err());
    assert!(serde_json::from_value::<MemoryIdRequest>(
        json!({"memory_id": "id", "root": "synthetic"})
    )
    .is_err());
    assert!(serde_json::from_value::<ReadErrorCode>(json!("provider_exception")).is_err());
    assert_eq!(
        serde_json::to_value(ReadErrorCode::ReadFailed).unwrap(),
        json!("read_failed")
    );
}

#[test]
fn committed_typescript_matches_the_rust_dtos() {
    let actual = include_str!("../../../apps/desktop/src/generated/runtime.ts");
    assert_eq!(actual.replace("\r\n", "\n"), soul_types::typescript());
}
