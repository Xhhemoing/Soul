//! Keeps the fixture corpora honest.
//!
//! Fixtures are the only thing standing behind several acceptance criteria, so
//! a truncated file or a JSONL line that silently stopped parsing would hollow
//! out a test without failing it. Everything under `fixtures/` is loaded here,
//! and the corpora that carry a specific promise are checked against it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;
use soul_testkit::fixtures;

fn files_under(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {dir:?}: {e}")) {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_json_fixture_parses() {
    let files = files_under(&fixtures::fixtures_dir(), "json");
    assert!(
        files.len() >= 25,
        "found only {} json fixtures",
        files.len()
    );
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read");
        serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));
    }
}

#[test]
fn every_jsonl_line_parses() {
    let files = files_under(&fixtures::fixtures_dir(), "jsonl");
    assert!(!files.is_empty(), "the import corpora went missing");
    for path in &files {
        let text = std::fs::read_to_string(path).expect("read");
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 4,
            "{} looks truncated: {} lines",
            path.display(),
            lines.len(),
        );
        for (index, line) in lines.iter().enumerate() {
            serde_json::from_str::<Value>(line).unwrap_or_else(|e| {
                panic!("{} line {} is not JSON: {e}", path.display(), index + 1)
            });
        }
    }
}

#[test]
fn the_valid_import_corpus_validates_against_the_contract() {
    let set = SchemaSet::load().expect("contracts compile");
    let lines = fixtures::read_jsonl("import/soul-import-v1/valid_basic.jsonl").expect("load");

    assert_eq!(lines[0]["type"], "header", "a header comes first");
    for (index, line) in lines.iter().enumerate() {
        if let Err(failure) = set.validate(SchemaId::SoulImportV1, line) {
            panic!(
                "valid_basic.jsonl line {} was rejected:\n{failure}",
                index + 1
            );
        }
    }
    assert!(lines.len() >= 5);
}

#[test]
fn the_malformed_import_corpus_is_rejected_line_by_line() {
    let set = SchemaSet::load().expect("contracts compile");
    let lines =
        fixtures::read_jsonl("import/soul-import-v1/invalid_missing_field.jsonl").expect("load");

    let rejected: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| set.validate(SchemaId::SoulImportV1, line).is_err())
        .map(|(index, _)| index + 1)
        .collect();

    assert_eq!(
        rejected.len(),
        lines.len() - 2,
        "exactly the deliberately broken lines should fail; rejected {rejected:?} of {} lines",
        lines.len(),
    );
    assert!(
        !rejected.contains(&1) && !rejected.contains(&2),
        "the header and the one good line must still pass",
    );
}

/// Hostile text is well-formed data. That is the whole point: it arrives
/// through the data channel and must never become an instruction.
#[test]
fn the_injection_corpus_is_structurally_valid_but_hostile() {
    let set = SchemaSet::load().expect("contracts compile");
    let lines = fixtures::read_jsonl("import/soul-import-v1/injection_lines.jsonl").expect("load");

    for (index, line) in lines.iter().enumerate() {
        if let Err(failure) = set.validate(SchemaId::SoulImportV1, line) {
            panic!(
                "injection_lines.jsonl line {} must be a well-formed import line:\n{failure}",
                index + 1,
            );
        }
    }

    let joined = serde_json::to_string(&lines).expect("re-encode");
    assert!(
        joined.contains("忽略之前指令"),
        "the Chinese instruction-override string must be present",
    );
    assert!(
        joined.contains("evil.example"),
        "the corpus must carry a URL no code path is allowed to contact",
    );
}

#[test]
fn the_telegram_fixtures_keep_the_desktop_export_shape() {
    let basic: Value = fixtures::read_json("import/telegram/result_basic.json").expect("load");

    let chats = basic["chats"]["list"]
        .as_array()
        .expect("chats.list is an array, as Telegram Desktop writes it");
    assert!(chats.len() >= 2);
    for chat in chats {
        assert!(chat["id"].is_number(), "each chat carries a numeric id");
        assert!(chat["type"].is_string());
        let messages = chat["messages"].as_array().expect("chats.list[].messages");
        assert!(!messages.is_empty());
    }

    let first = &chats[0]["messages"][0];
    for field in ["id", "type", "date", "date_unixtime", "from", "from_id"] {
        assert!(
            !first[field].is_null(),
            "a Telegram message carries `{field}`",
        );
    }
    assert!(
        basic["contacts"]["list"].is_array(),
        "the export includes the contact list",
    );
    assert!(
        chats[0]["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .any(|m| m["type"] == "service"),
        "service messages exist in real exports and the adapter must survive them",
    );

    // The hostile export is the same shape, or it proves nothing about the
    // adapter: it has to reach `telegram::parse` before it can be counted.
    let hostile: Value =
        fixtures::read_json("import/telegram/result_injection.json").expect("load");
    assert!(
        hostile["personal_information"]["user_id"].is_number(),
        "a Telegram export names its owner by user id",
    );
    let hostile_messages = hostile["chats"]["list"][0]["messages"]
        .as_array()
        .expect("chats.list[].messages");
    for field in ["id", "type", "date", "date_unixtime", "from_id"] {
        assert!(
            !hostile_messages[0][field].is_null(),
            "a Telegram message carries `{field}`",
        );
    }

    // Telegram cuts `text` into runs wherever an entity begins, and the whole
    // point of this fixture is that the override phrase is only there once the
    // runs are joined. A fixture edited into a single run would leave
    // `soul-import`'s flattening untested while its tests stayed green.
    let split = hostile_messages
        .iter()
        .filter_map(|message| message["text"].as_array())
        .find(|runs| {
            runs.iter()
                .filter_map(|run| run.as_str().or_else(|| run["text"].as_str()))
                .collect::<String>()
                .contains("忽略之前指令")
        })
        .expect("one message spells the override phrase out across its runs");
    assert!(
        split
            .iter()
            .filter_map(|run| run.as_str().or_else(|| run["text"].as_str()))
            .all(|run| !run.contains("忽略之前指令")),
        "no single run may carry the whole phrase: {split:?}",
    );
    assert!(
        serde_json::to_string(&hostile)
            .expect("re-encode")
            .contains("evil.example"),
        "the export must carry a URL no code path is allowed to contact",
    );

    let broken: Value =
        fixtures::read_json("import/telegram/result_missing_fields.json").expect("load");
    let broken_chats = broken["chats"]["list"].as_array().expect("list");
    assert!(
        broken_chats.iter().any(|c| c["messages"].is_null()),
        "one chat must be missing its messages array",
    );
    assert!(
        broken_chats.iter().any(|c| c["id"].is_null()),
        "one chat must be missing its id",
    );
}

#[test]
fn the_denylist_is_a_clean_single_source_of_truth() {
    let terms = fixtures::denylist_terms().expect("load the denylist");
    assert!(terms.len() >= 60, "found only {} terms", terms.len());

    let unique: BTreeSet<&String> = terms.iter().collect();
    assert_eq!(
        unique.len(),
        terms.len(),
        "the denylist has duplicate lines"
    );

    for required in ["score", "percentile", "量表", "得分"] {
        assert!(
            terms.iter().any(|t| t == required),
            "the denylist must contain `{required}`",
        );
    }
    for forbidden in ["clinical", "diagnostics"] {
        assert!(
            !terms.iter().any(|t| t == forbidden),
            "`{forbidden}` is a guard field or a legitimate privacy purpose and must not be denied",
        );
    }
    for term in &terms {
        assert_eq!(term.trim(), term, "`{term}` has stray whitespace");
        assert!(
            !term.starts_with('#'),
            "comments should already be stripped"
        );
    }
}

#[test]
fn the_injection_corpora_are_present_and_pointed() {
    let paste = fixtures::read_lines("injection/paste_injection.txt").expect("load");
    let payloads: Vec<&String> = paste.iter().filter(|l| !l.starts_with('#')).collect();
    assert!(
        payloads.len() >= 10,
        "found {} paste payloads",
        payloads.len()
    );
    assert!(payloads.iter().any(|l| l.contains("忽略之前指令")));
    assert!(payloads
        .iter()
        .any(|l| l.contains("Ignore previous instructions")));
    assert!(payloads.iter().any(|l| l.contains("evil.example")));

    let names = fixtures::read_lines("injection/filenames.txt").expect("load");
    let entries: Vec<&String> = names.iter().filter(|l| !l.starts_with('#')).collect();
    assert!(entries.len() >= 20, "found {} filenames", entries.len());
    assert!(entries.iter().any(|l| l.contains("忽略之前指令")));
    assert!(
        entries.iter().any(|l| l.contains("..")),
        "a traversal attempt must be in the corpus",
    );
    assert!(
        entries.iter().any(|l| l.eq_ignore_ascii_case("NUL")),
        "reserved Windows device names must be in the corpus",
    );
}

#[test]
fn the_leakage_fixture_covers_the_named_edge_cases() {
    let fixture = fixtures::leakage_fixture().expect("load");

    let bodies: Vec<&str> = fixture
        .third_party_bodies
        .iter()
        .map(|b| b.text.as_str())
        .collect();
    let identifiers: Vec<&str> = fixture
        .known_identifiers
        .iter()
        .map(|i| i.text.as_str())
        .collect();

    assert!(identifiers.contains(&"李雷"), "the two-scalar name");
    assert!(identifiers.contains(&"@wang_xiao2"), "the handle");
    assert!(bodies.contains(&"好的没问题"), "the five-scalar reply");
    assert!(
        bodies.iter().any(|b| b.contains('\u{200D}')),
        "a zero-width joiner sequence",
    );
    assert!(
        bodies.iter().any(|b| b.contains('\u{00E9}')),
        "a composed café",
    );
    assert!(
        bodies.iter().any(|b| b.contains('\u{0301}')),
        "a decomposed café",
    );

    // The registered number, written the ways a keyboard writes it. NFC folds
    // none of these into the ASCII spelling the corpus registers, so without
    // them the harness could report clean on the number itself going out.
    let leaking: Vec<&str> = fixture
        .cases
        .iter()
        .filter(|c| c.leaks)
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        leaking.iter().any(|t| t.contains('\u{FF10}')),
        "a number written in fullwidth digits",
    );
    assert!(
        leaking.iter().any(|t| t.contains('\u{2013}')),
        "a number grouped by an en-dash",
    );
    assert!(
        leaking.iter().any(|t| t.contains('\u{FF0D}')),
        "a number grouped by a fullwidth hyphen",
    );

    let hits = fixture.cases.iter().filter(|c| c.leaks).count();
    let misses = fixture.cases.len() - hits;
    assert!(hits >= 6 && misses >= 3, "{hits} leaking, {misses} clean");
}
