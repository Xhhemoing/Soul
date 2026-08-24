//! Proves the guardrails are not no-ops.
//!
//! An audit that never fails is worse than no audit, because it reads as a
//! green tick. Each check here builds a small tree that should trip the
//! auditor and asserts that it does, then a clean counterpart and asserts that
//! it does not.

use std::path::{Path, PathBuf};

use xtask::denylist::{self, HitContext};
use xtask::egress;
use xtask::schema_freeze;

fn workspace_metadata() -> cargo_metadata::Metadata {
    cargo_metadata::MetadataCommand::new()
        .manifest_path(xtask::repo_root().join("Cargo.toml"))
        .exec()
        .expect("cargo metadata")
}

fn write(root: &Path, relative: &str, contents: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("mkdir");
    std::fs::write(&path, contents).expect("write");
    path
}

// ------------------------------------------------------------ e0-audit ---

#[test]
fn the_url_scanner_fails_on_a_vendor_domain() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(
        dir.path(),
        "crates/pretend/src/lib.rs",
        "pub const TELEMETRY: &str = \"https://evil.example/collect\";\n",
    );

    let scan = egress::scan_tree_for_urls(dir.path()).expect("scan");
    assert_eq!(
        scan.hits.len(),
        1,
        "a vendor domain in a source file must be reported; got {:#?}",
        scan.hits,
    );
    assert_eq!(scan.hits[0].url, "https://evil.example/collect");
    assert_eq!(scan.hits[0].line, 1);
}

#[test]
fn the_url_scanner_accepts_the_allowlist() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(
        dir.path(),
        "crates/pretend/src/lib.rs",
        concat!(
            "const SCHEMA: &str = \"https://soul.local/schemas/event.schema.json\";\n",
            "const CORE: &str = \"http://127.0.0.1:7331/rpc\";\n",
            "const UI: &str = \"http://localhost:1420\";\n",
            "fn base(port: u16) -> String { format!(\"http://127.0.0.1:{port}\") }\n",
        ),
    );

    let scan = egress::scan_tree_for_urls(dir.path()).expect("scan");
    assert!(
        scan.hits.is_empty(),
        "the allowlist should have covered these: {:#?}",
        scan.hits,
    );
    assert_eq!(scan.files_scanned, 1);
}

#[test]
fn the_url_scanner_skips_fixtures_and_tests() {
    let dir = tempfile::tempdir().expect("temp dir");
    let offending = "const BAD: &str = \"https://evil.example/x\";\n";
    write(dir.path(), "fixtures/injection/paste.txt", offending);
    write(dir.path(), "crates/pretend/tests/red_line.rs", offending);
    write(dir.path(), "crates/pretend/src/clean.rs", "pub fn f() {}\n");

    let scan = egress::scan_tree_for_urls(dir.path()).expect("scan");
    assert!(
        scan.hits.is_empty(),
        "corpora and tests are where the attack strings are supposed to live: {:#?}",
        scan.hits,
    );
}

/// Build output is skipped by the manifest beside it, not by its name. A
/// hand-written `src/gen/` has no `tauri.conf.json` next to it and stays in
/// the audit; without this test the exemption would be a way to hide source
/// from the scanner by choosing a directory name.
#[test]
fn the_url_scanner_skips_generated_output_but_not_a_directory_that_shares_its_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let offending = "const BAD: &str = \"https://evil.example/x\";\n";

    write(dir.path(), "apps/desktop/package.json", "{}\n");
    write(dir.path(), "apps/desktop/dist/assets/bundle.js", offending);
    write(dir.path(), "apps/desktop/src-tauri/tauri.conf.json", "{}\n");
    write(
        dir.path(),
        "apps/desktop/src-tauri/gen/schemas/x.json",
        offending,
    );

    write(dir.path(), "crates/pretend/src/gen/table.rs", offending);
    write(dir.path(), "crates/pretend/dist/notes.rs", offending);

    let scan = egress::scan_tree_for_urls(dir.path()).expect("scan");
    let flagged: Vec<String> = scan
        .hits
        .iter()
        .map(|hit| hit.file.display().to_string().replace('\\', "/"))
        .collect();

    assert_eq!(
        flagged.len(),
        2,
        "expected only the source copies: {flagged:#?}"
    );
    assert!(
        flagged
            .iter()
            .any(|f| f.ends_with("crates/pretend/src/gen/table.rs")),
        "a hand-written gen/ has no generator manifest beside it: {flagged:#?}",
    );
    assert!(
        flagged
            .iter()
            .any(|f| f.ends_with("crates/pretend/dist/notes.rs")),
        "a dist/ with no package.json beside it is not bundler output: {flagged:#?}",
    );
}

/// A bare scheme from a `format!` has no domain to object to.
#[test]
fn the_url_scanner_ignores_a_scheme_with_no_host() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(
        dir.path(),
        "crates/pretend/src/lib.rs",
        "fn url(addr: &str) -> String { format!(\"http://{addr}/v1\") }\n",
    );
    let scan = egress::scan_tree_for_urls(dir.path()).expect("scan");
    assert!(scan.hits.is_empty(), "{:#?}", scan.hits);
}

/// The dependency walker must be able to see an HTTP stack when one is
/// reachable. `soul-testkit` really does depend on axum and therefore hyper,
/// so treating it as a shipped root has to produce a finding. Without this,
/// a clean report would be indistinguishable from a walker that never looks.
#[test]
fn the_dependency_walker_finds_a_real_http_stack_when_one_is_in_scope() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(xtask::repo_root().join("Cargo.toml"))
        .exec()
        .expect("cargo metadata");

    let (clean, shipped_roots) = egress::audit_dependencies(&metadata);
    assert!(clean.is_empty(), "{clean:#?}");
    assert!(!shipped_roots.iter().any(|r| r == "soul-testkit"));

    let (findings, all_roots) = egress::audit_dependencies_from_roots(&metadata, &[]);
    assert!(all_roots.iter().any(|r| r == "soul-testkit"));
    assert!(
        findings
            .iter()
            .any(|f| f.root == "soul-testkit" && f.banned == "hyper"),
        "expected the walker to reach hyper from soul-testkit; got {findings:#?}",
    );
}

/// The one shipped exception has to be exactly one crate wide, and it has to
/// still be in use. Both halves matter: a second holder is a hole, and a
/// gateway that no longer owns an HTTP client is an exemption sheltering
/// whatever ends up under it next.
#[test]
fn the_http_client_exception_covers_soul_egress_and_nothing_else() {
    let metadata = workspace_metadata();
    assert!(
        egress::audit_gateway(&metadata).is_empty(),
        "{:#?}",
        egress::audit_gateway(&metadata),
    );

    // Computed independently of `audit_gateway`, so a bug that made the check
    // return nothing would not also make this agree.
    let holders: Vec<&str> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .filter(|p| !egress::TEST_TOOLING.contains(&p.name.as_str()))
        .filter(|p| {
            p.dependencies.iter().any(|d| {
                d.kind == cargo_metadata::DependencyKind::Normal
                    && egress::BANNED_HTTP_CLIENTS.contains(&d.name.as_str())
            })
        })
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(holders, vec![egress::EGRESS_GATEWAY]);
}

/// The negative control for the exception: give a second crate the same
/// dependency `soul-egress` has and the audit must object. Without this, a
/// check that returned an empty list unconditionally would look identical.
#[test]
fn a_second_crate_holding_an_http_client_is_reported() {
    let metadata = workspace_metadata();
    let mut tampered = metadata.clone();

    let http_client = tampered
        .packages
        .iter()
        .find(|p| p.name == egress::EGRESS_GATEWAY)
        .expect("the gateway is a workspace member")
        .dependencies
        .iter()
        .find(|d| egress::BANNED_HTTP_CLIENTS.contains(&d.name.as_str()))
        .expect("the gateway holds an HTTP client")
        .clone();

    let victim = tampered
        .packages
        .iter_mut()
        .find(|p| p.name == "soul-policy")
        .expect("soul-policy is a workspace member");
    victim.dependencies.push(http_client);

    let findings = egress::audit_gateway(&tampered);
    assert!(
        findings.iter().any(
            |f| matches!(f, egress::GatewayFinding::SecondHolder { crate_name, .. }
                if crate_name == "soul-policy")
        ),
        "expected soul-policy to be reported; got {findings:#?}",
    );
}

/// And the other half: a gateway that stopped holding an HTTP client.
#[test]
fn an_unused_exception_is_reported() {
    let mut tampered = workspace_metadata();
    let gateway = tampered
        .packages
        .iter_mut()
        .find(|p| p.name == egress::EGRESS_GATEWAY)
        .expect("the gateway is a workspace member");
    gateway
        .dependencies
        .retain(|d| !egress::BANNED_HTTP_CLIENTS.contains(&d.name.as_str()));

    assert_eq!(
        egress::audit_gateway(&tampered),
        vec![egress::GatewayFinding::ExceptionUnused],
    );
}

/// `soul-policy` decides whether a request may leave; it must not be able to
/// make one. The dependency direction is what keeps the permit meaningful.
#[test]
fn the_policy_crate_cannot_reach_an_http_client() {
    let metadata = workspace_metadata();
    let (findings, _) = egress::audit_dependencies_from_roots(&metadata, egress::TEST_TOOLING);
    assert!(findings.is_empty(), "{findings:#?}");

    let policy = metadata
        .packages
        .iter()
        .find(|p| p.name == "soul-policy")
        .expect("soul-policy is a workspace member");
    assert!(
        !policy
            .dependencies
            .iter()
            .any(|d| d.name == egress::EGRESS_GATEWAY),
        "soul-policy must not depend on the crate it issues permits for",
    );
}

#[test]
fn this_repository_passes_the_e0_audit() {
    let report = egress::audit(&xtask::repo_root()).expect("audit runs");
    assert!(report.is_clean(), "{report}");
    assert!(
        !report.roots.is_empty(),
        "the dependency walk must actually start somewhere",
    );
    assert!(
        !report.roots.iter().any(|r| r == "soul-testkit"),
        "soul-testkit owns an HTTP stack and must never be a shipped root",
    );
    assert!(
        report.files_scanned > 0,
        "the source scan must actually read files",
    );
}

// ------------------------------------------------------ denylist-audit ---

#[test]
fn the_denylist_scanner_fails_on_a_forbidden_identifier() {
    let terms = denylist::parse_terms("score\npercentile\n抑郁\n");
    let hits = denylist::scan_source(
        Path::new("crates/pretend/src/lib.rs"),
        "pub struct TraitAxis { pub score: u8 }\n",
        &terms,
    );
    assert!(
        hits.iter()
            .any(|h| h.term == "score" && h.context == HitContext::Identifier),
        "a field called `score` must be reported; got {hits:#?}",
    );
}

#[test]
fn the_denylist_scanner_fails_on_a_forbidden_string_literal() {
    let terms = denylist::parse_terms("抑郁\nsymptom\n");
    let hits = denylist::scan_source(
        Path::new("crates/pretend/src/lib.rs"),
        "const SUMMARY: &str = \"该用户有抑郁倾向\";\n",
        &terms,
    );
    assert!(
        hits.iter()
            .any(|h| h.term == "抑郁" && h.context == HitContext::StringLiteral),
        "a diagnostic term in a user-visible string must be reported; got {hits:#?}",
    );
}

#[test]
fn the_denylist_scanner_respects_word_boundaries() {
    let terms = denylist::parse_terms("score\ntherapy\n");
    let clean = concat!(
        "pub fn underscore_separated() {}\n",
        "pub struct Psychotherapy;\n",
        "const NOTE: &str = \"underscore\";\n",
    );
    let hits = denylist::scan_source(Path::new("crates/pretend/src/lib.rs"), clean, &terms);
    assert!(
        hits.is_empty(),
        "`underscore` is not `score` and `Psychotherapy` is not `therapy`: {hits:#?}",
    );
}

#[test]
fn the_denylist_scanner_ignores_comments() {
    let terms = denylist::parse_terms("symptom\n");
    let source = concat!(
        "// symptom: this word is fine in a note to a maintainer\n",
        "/* symptom */\n",
        "pub fn f() {}\n",
    );
    let hits = denylist::scan_source(Path::new("crates/pretend/src/lib.rs"), source, &terms);
    assert!(hits.is_empty(), "{hits:#?}");
}

#[test]
fn the_denylist_scanner_reads_raw_strings() {
    let terms = denylist::parse_terms("symptom\n");
    let source = "const Q: &str = r#\"a symptom of something\"#;\n";
    let hits = denylist::scan_source(Path::new("crates/pretend/src/lib.rs"), source, &terms);
    assert_eq!(hits.len(), 1, "{hits:#?}");
    assert_eq!(hits[0].context, HitContext::StringLiteral);
}

#[test]
fn multi_word_terms_match_across_identifier_words() {
    let terms = denylist::parse_terms("personality disorder\n");
    let hits = denylist::scan_source(
        Path::new("crates/pretend/src/lib.rs"),
        "pub enum PersonalityDisorder { A }\n",
        &terms,
    );
    assert_eq!(hits.len(), 1, "{hits:#?}");
}

#[test]
fn xtask_and_fixtures_are_exempt_from_the_denylist() {
    assert!(denylist::is_exempt(Path::new(
        "crates/xtask/src/denylist.rs"
    )));
    assert!(denylist::is_exempt(Path::new(
        "crates/soul-schema/tests/roundtrip.rs"
    )));
    assert!(denylist::is_exempt(Path::new(
        "fixtures/denylist/diagnostic_terms.txt"
    )));
    assert!(!denylist::is_exempt(Path::new(
        "crates/soul-schema/src/profile.rs"
    )));
}

#[test]
fn this_repository_passes_the_denylist_audit() {
    let report = denylist::audit(&xtask::repo_root()).expect("audit runs");
    assert!(report.is_clean(), "{report}");
    assert!(
        report.terms_loaded > 40,
        "the denylist should be substantial, found {} terms",
        report.terms_loaded,
    );
    assert!(report.files_scanned > 0);
}

// -------------------------------------------------------- schema-freeze ---

#[test]
fn the_schema_lock_matches_this_checkout() {
    let drift = schema_freeze::check_lock(&xtask::repo_root()).expect("check runs");
    assert!(
        drift.is_empty(),
        "docs/schemas drifted from the lock: {drift:#?}",
    );
}

#[test]
fn the_lock_covers_every_frozen_document() {
    let root = xtask::repo_root();
    let lock = schema_freeze::read_lock(&root).expect("lock parses");
    let actual = schema_freeze::digest_schema_dir(&root).expect("digest");

    assert_eq!(lock.algorithm, "sha256");
    assert_eq!(lock.schemas.len(), schema_freeze::EXPECTED_SCHEMA_COUNT);
    assert_eq!(
        lock.schemas.keys().collect::<Vec<_>>(),
        actual.keys().collect::<Vec<_>>(),
    );
    for digest in lock.schemas.values() {
        assert_eq!(digest.len(), 64, "sha256 renders as 64 hex characters");
        assert!(
            digest
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "digests are lowercase hex: {digest}",
        );
    }
}

/// The freeze must notice a byte-level edit, not just a missing file.
#[test]
fn the_schema_freeze_notices_an_edited_document() {
    let source = xtask::repo_root();
    let dir = tempfile::tempdir().expect("temp dir");
    let staging = dir.path();

    std::fs::create_dir_all(staging.join("docs/schemas")).expect("mkdir");
    for entry in std::fs::read_dir(source.join("docs/schemas")).expect("read source schemas") {
        let entry = entry.expect("entry");
        std::fs::copy(
            entry.path(),
            staging.join("docs/schemas").join(entry.file_name()),
        )
        .expect("copy");
    }

    assert!(
        schema_freeze::check_lock(staging)
            .expect("check the copy")
            .is_empty(),
        "an untouched copy must still match the lock",
    );

    let victim = staging.join("docs/schemas/audit.schema.json");
    let mut text = std::fs::read_to_string(&victim).expect("read");
    text.push('\n');
    std::fs::write(&victim, text).expect("write");

    let drift = schema_freeze::check_lock(staging).expect("check the edited copy");
    assert_eq!(
        drift.len(),
        1,
        "one edited document should produce exactly one drift entry: {drift:#?}",
    );
    assert!(matches!(
        &drift[0],
        schema_freeze::LockDrift::Changed { file, .. } if file == "audit.schema.json",
    ));
}

/// Git for Windows may check out LF files as CRLF. That must not fail the freeze.
#[test]
fn a_crlf_checkout_still_matches_the_lock() {
    let source = xtask::repo_root();
    let dir = tempfile::tempdir().expect("temp dir");
    let staging = dir.path();

    std::fs::create_dir_all(staging.join("docs/schemas")).expect("mkdir");
    for entry in std::fs::read_dir(source.join("docs/schemas")).expect("read source schemas") {
        let entry = entry.expect("entry");
        let dest = staging.join("docs/schemas").join(entry.file_name());
        std::fs::copy(entry.path(), &dest).expect("copy");
        if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if entry.file_name() == std::ffi::OsStr::new("schemas.lock.json") {
            continue;
        }
        let lf = std::fs::read_to_string(&dest).expect("read");
        let crlf = lf.replace('\n', "\r\n");
        std::fs::write(&dest, crlf).expect("write crlf");
    }

    assert!(
        schema_freeze::check_lock(staging)
            .expect("check the crlf copy")
            .is_empty(),
        "CRLF checkouts must still match the lock",
    );
}
