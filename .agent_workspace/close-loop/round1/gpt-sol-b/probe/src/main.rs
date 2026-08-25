use serde_json::json;
use soul_graph::TieStrength;
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;

fn main() {
    // This is the eight-field shape written before the frozen tie rule landed.
    // Deserialization uses TieStrength's serde defaults, exactly as read_strength does.
    let legacy = json!({
        "band": "moderate",
        "interaction_count": 4,
        "outgoing_count": 2,
        "incoming_count": 2,
        "conversation_count": 1,
        "active_day_count": 2,
        "first_contact_utc": "2026-08-01T00:00:00Z",
        "last_contact_utc": "2026-08-02T00:00:00Z"
    });
    let strength: TieStrength =
        serde_json::from_value(legacy).expect("legacy TieStrength must deserialize");
    assert_eq!(strength.algorithm_id, "");

    // correct_tie/release_tie call serde_json::to_value on this same model.
    let serialized = serde_json::to_value(&strength).expect("TieStrength must serialize");
    assert_eq!(serialized["algorithm_id"], "");

    let relationship = json!({
        "schema_version": "1.0.0",
        "relationship_id": "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a70",
        "from_contact_id": "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a60",
        "to_contact_id": "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a63",
        "tie_strength": serialized,
        "evidence_ids": ["0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a10"]
    });

    let schemas = SchemaSet::load().expect("frozen schemas must compile");
    let failure = schemas
        .validate(SchemaId::Relationship, &relationship)
        .expect_err("empty algorithm_id must fail relationship.schema.json");
    assert!(
        failure
            .errors
            .iter()
            .any(|error| error.contains("T4D")
                && error.contains("T4")
                && error.contains("/tie_strength/algorithm_id")),
        "expected the algorithm_id enum rejection, got: {failure}"
    );

    println!(
        "serialized TieStrength:\n{}",
        serde_json::to_string_pretty(&relationship["tie_strength"]).unwrap()
    );
    println!("relationship schema valid: false");
    println!("{failure}");
}
