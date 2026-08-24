//! Proves the frozen contracts are wired the way PRODUCT_LOCK requires.
//!
//! The load-bearing claim is that the nine entity schemas draw `privacy`,
//! `sealedText`, `uuid7`, `sha256`, `evidenceBand` and `timestamp` from
//! `_defs.schema.json` across a document boundary. String matching would not
//! prove that, so the checks here observe what the `$ref` resolver is actually
//! asked for and confirm that withholding `_defs` breaks compilation.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use jsonschema::{Draft, Retrieve, Uri};
use serde_json::Value;
use soul_schema::validate::{LocalRetriever, SchemaId, SCHEMA_BASE};
use soul_schema::SchemaSet;

const DEFS_URI: &str = "https://soul.local/schemas/_defs.schema.json";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/soul-schema sits two levels below the repository root")
        .to_path_buf()
}

/// `fixtures/schemas/{valid,invalid}/<stem>/` for one schema.
fn fixture_dir(kind: &str, id: SchemaId) -> PathBuf {
    let stem = id
        .file_name()
        .strip_suffix(".schema.json")
        .expect("every schema file name ends in .schema.json");
    repo_root().join("fixtures").join("schemas").join(kind).join(stem)
}

fn load_fixtures(kind: &str, id: SchemaId) -> Vec<(String, Value)> {
    let dir = fixture_dir(kind, id);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.expect("readable dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    entries.sort();
    for path in entries {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let value: Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));
        out.push((path.display().to_string(), value));
    }
    out
}

#[test]
fn every_schema_compiles() {
    let set = SchemaSet::load().expect("all eleven contracts compile as draft 2020-12");
    assert_eq!(SchemaId::ALL.len(), 11, "the frozen set is eleven documents");
    for &id in SchemaId::ALL {
        let _ = set.validator(id);
    }
}

#[test]
fn every_schema_accepts_at_least_one_valid_fixture() {
    let set = SchemaSet::load().expect("contracts compile");
    for &id in SchemaId::ALL {
        let fixtures = load_fixtures("valid", id);
        assert!(
            !fixtures.is_empty(),
            "{id} has no valid fixture under fixtures/schemas/valid/",
        );
        for (name, value) in fixtures {
            if let Err(failure) = set.validate(id, &value) {
                panic!("{name} should validate against {id}:\n{failure}");
            }
        }
    }
}

#[test]
fn every_invalid_fixture_is_rejected() {
    let set = SchemaSet::load().expect("contracts compile");
    let mut checked = 0usize;
    for &id in SchemaId::ALL {
        for (name, value) in load_fixtures("invalid", id) {
            assert!(
                set.validate(id, &value).is_err(),
                "{name} must be rejected by {id} but was accepted",
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 4,
        "the negative corpus is the only thing standing between a typo and a silent contract loosening",
    );
}

/// The four rejections the work order names explicitly.
#[test]
fn named_negative_cases_are_present_and_rejected() {
    let set = SchemaSet::load().expect("contracts compile");
    let cases: [(SchemaId, &str); 4] = [
        (SchemaId::Event, "privacy_e0_not_deny.json"),
        (
            SchemaId::ExportManifest,
            "research_preview_written_to_disk.json",
        ),
        (SchemaId::Inference, "missing_evidence_ids.json"),
        (SchemaId::Audit, "extra_body_field.json"),
    ];
    for (id, file) in cases {
        let path = fixture_dir("invalid", id).join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing required negative fixture {}: {e}", path.display()));
        let value: Value = serde_json::from_str(&text).expect("fixture parses");
        assert!(
            set.validate(id, &value).is_err(),
            "{} must be rejected by {id}",
            path.display()
        );
    }
}

/// Records every URI the `$ref` resolver is asked to fetch.
#[derive(Debug, Clone, Default)]
struct RecordingRetriever {
    seen: Arc<Mutex<BTreeSet<String>>>,
}

impl Retrieve for RecordingRetriever {
    fn retrieve(&self, uri: &Uri<&str>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.seen
            .lock()
            .expect("no panics while holding the lock")
            .insert(uri.as_str().to_owned());
        LocalRetriever.retrieve(uri)
    }
}

/// Refuses `_defs` so that any document genuinely depending on it fails.
#[derive(Debug, Clone, Copy)]
struct DefsWithholdingRetriever;

impl Retrieve for DefsWithholdingRetriever {
    fn retrieve(&self, uri: &Uri<&str>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if uri.as_str() == DEFS_URI {
            return Err("_defs deliberately withheld".into());
        }
        LocalRetriever.retrieve(uri)
    }
}

#[test]
fn the_nine_resolve_shared_definitions_from_defs_across_documents() {
    assert_eq!(SchemaId::REFERENCING_DEFS.len(), 9);
    for &id in SchemaId::REFERENCING_DEFS {
        let document: Value = serde_json::from_str(id.source()).expect("schema parses");

        let recorder = RecordingRetriever::default();
        jsonschema::options()
            .with_draft(Draft::Draft202012)
            .with_retriever(recorder.clone())
            .should_validate_formats(true)
            .build(&document)
            .unwrap_or_else(|e| panic!("{id} failed to compile: {e}"));

        let seen = recorder.seen.lock().expect("lock").clone();
        assert!(
            seen.contains(DEFS_URI),
            "{id} compiled without ever fetching {DEFS_URI}; its shared definitions are still inlined (resolver saw {seen:?})",
        );
    }
}

#[test]
fn withholding_defs_breaks_every_one_of_the_nine() {
    for &id in SchemaId::REFERENCING_DEFS {
        let document: Value = serde_json::from_str(id.source()).expect("schema parses");
        let result = jsonschema::options()
            .with_draft(Draft::Draft202012)
            .with_retriever(DefsWithholdingRetriever)
            .should_validate_formats(true)
            .build(&document);
        assert!(
            result.is_err(),
            "{id} still compiles when _defs is unavailable, so it is not really referencing it",
        );
    }
}

/// `_defs` itself must stay a pure vocabulary document.
#[test]
fn defs_declares_the_expected_vocabulary() {
    let defs: Value = serde_json::from_str(SchemaId::Defs.source()).expect("parses");
    let names = defs
        .get("$defs")
        .and_then(Value::as_object)
        .expect("_defs.schema.json has a $defs object");
    for expected in [
        "uuid7",
        "timestamp",
        "sha256",
        "evidenceBand",
        "sealedText",
        "privacy",
    ] {
        assert!(names.contains_key(expected), "_defs lost $defs/{expected}");
    }
}

/// A private copy of a shared definition would silently drift. Forbid it.
#[test]
fn the_nine_keep_no_private_copy_of_shared_definitions() {
    let shared = [
        "uuid7",
        "timestamp",
        "sha256",
        "evidenceBand",
        "sealedText",
        "privacy",
    ];
    for &id in SchemaId::REFERENCING_DEFS {
        let document: Value = serde_json::from_str(id.source()).expect("schema parses");
        if let Some(local) = document.get("$defs").and_then(Value::as_object) {
            for name in shared {
                assert!(
                    !local.contains_key(name),
                    "{id} still carries a local $defs/{name}",
                );
            }
        }
    }
}

fn walk(value: &Value, path: &str, visit: &mut impl FnMut(&str, &str, &Value)) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}/{key}");
                visit(&child_path, key, child);
                walk(child, &child_path, visit);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let child_path = format!("{path}/{index}");
                walk(child, &child_path, visit);
            }
        }
        _ => {}
    }
}

/// Returns every `$ref` string reachable from a subschema, including through
/// `oneOf` / `allOf` / `items`.
fn refs_within(subschema: &Value) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(direct) = subschema.get("$ref").and_then(Value::as_str) {
        found.push(direct.to_owned());
    }
    walk(subschema, "", &mut |_, key, child| {
        if key == "$ref" {
            if let Some(text) = child.as_str() {
                found.push(text.to_owned());
            }
        }
    });
    found
}

/// No entity identifier may stay a bare `{"type": "string"}`.
#[test]
fn identifier_properties_reference_uuid7() {
    let uuid7_ref = format!("{SCHEMA_BASE}_defs.schema.json#/$defs/uuid7");
    for &id in SchemaId::REFERENCING_DEFS {
        let document: Value = serde_json::from_str(id.source()).expect("schema parses");
        walk(&document, "", &mut |path, key, child| {
            // Only look at `properties/<name>` positions.
            if !path.contains("/properties/") {
                return;
            }
            let is_singular = key.ends_with("_id") && key != "schema_version";
            let is_plural = key.ends_with("_ids");
            if !(is_singular || is_plural) {
                return;
            }
            let target = if is_plural {
                child.get("items").unwrap_or(child)
            } else {
                child
            };
            let refs = refs_within(target);
            assert!(
                refs.iter().any(|r| *r == uuid7_ref),
                "{id} property `{key}` at {path} does not reach {uuid7_ref}",
            );
        });
    }
}

/// Every reference in the frozen set stays inside the Soul naming authority.
#[test]
fn no_schema_reaches_outside_the_local_naming_authority() {
    for &id in SchemaId::ALL {
        let document: Value = serde_json::from_str(id.source()).expect("schema parses");
        walk(&document, "", &mut |path, key, child| {
            if key != "$ref" {
                return;
            }
            let target = child.as_str().unwrap_or_default();
            assert!(
                target.starts_with('#') || target.starts_with(SCHEMA_BASE),
                "{id} references {target} at {path}, which is outside {SCHEMA_BASE}",
            );
        });
    }
}
