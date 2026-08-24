//! One store, opened in one place, by rules that are not written here.
//!
//! WP07's leftover 8 and WP09's trade-off 8 say the same thing: a process gets
//! one `SqlCipherStore` handle, because two connections write two write-ahead
//! logs and the second one is always the one somebody added "just for this
//! command". The runtime version of that is hard to observe from a shell with
//! no store command yet, so what is asserted here is the shape of the source —
//! the same trick `command_surface.rs` uses on the command layer, and the same
//! one `crates/soul-store/tests/research_preview.rs` uses on the research
//! module.
//!
//! The second rule is about who chooses the key. Picking between
//! `DpapiKeyProvider` and `TestKeyProvider` is a security decision; if it lived
//! in this crate then changing the key policy would be a UI pull request, and
//! UI pull requests are not reviewed as data-plane changes. So the shell must
//! not name a key provider at all.

use std::fs;
use std::path::{Path, PathBuf};

/// Every Rust file under `src/`, read at run time rather than named in an
/// `include_str!` list: a new module must not be able to hide from this.
fn shell_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    collect(&root, &root, &mut found);
    found.sort();
    assert!(
        found.len() >= 4,
        "the shell has more source files than this scan found: {found:?}",
    );
    found
}

fn collect(root: &Path, directory: &Path, found: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(directory).expect("read the shell's source directory") {
        let entry = entry.expect("a directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .strip_prefix(root)
                .expect("a path under src")
                .to_string_lossy()
                .into_owned();
            found.push((name, fs::read_to_string(&path).expect("read a source file")));
        }
    }
}

fn source(name: &str) -> String {
    shell_sources()
        .into_iter()
        .find(|(found, _)| found == name)
        .map(|(_, text)| text)
        .unwrap_or_else(|| panic!("{name} is not in the shell's sources"))
}

/// The body of a top-level function, from its signature to the line that
/// closes it. Enough to ask what a specific function does and no more.
fn function_body(text: &str, header: &str) -> String {
    let start = text
        .find(header)
        .unwrap_or_else(|| panic!("{header} is not in the source"));
    let rest = &text[start..];
    let end = rest.find("\n}").expect("the function ends");
    rest[..end].to_owned()
}

fn occurrences(needle: &str) -> Vec<String> {
    shell_sources()
        .into_iter()
        .flat_map(|(name, text)| {
            std::iter::repeat_n(name, text.matches(needle).count())
        })
        .collect()
}

/// S-01. The database is opened once, in the whole shell.
#[test]
fn the_store_is_opened_in_exactly_one_place() {
    assert_eq!(
        occurrences("open_store"),
        vec!["lib.rs".to_owned()],
        "opening the store belongs in setup, once; every other call site is a second connection",
    );
    assert!(occurrences("open_test_store").is_empty());
    assert!(occurrences("SqlCipherStore::open").is_empty());
    assert!(
        occurrences("Mutex::new").is_empty() && occurrences("Arc::new").is_empty(),
        "the shared handle comes from soulcore::commands::collect::share; a second wrapper here \
         is a second handle waiting to happen",
    );

    // Guard against all of the above passing because the file moved.
    let lib = source("lib.rs");
    assert!(lib.contains("open_store_for_session"));
    assert!(lib.contains("fn install_store"));
}

/// The IPC layer never touches the database directly, so no command can open
/// "just a read-only one" on the side.
#[test]
fn no_command_opens_a_store_of_its_own() {
    let commands = source("commands.rs");
    for needle in ["open_store", "SqlCipherStore", "Mutex", "Arc"] {
        assert!(
            !commands.contains(needle),
            "commands.rs mentions `{needle}`; the command layer forwards and holds nothing",
        );
    }
}

/// S-02. The shell may not know that key providers exist, let alone pick one.
#[test]
fn the_shell_does_not_choose_the_key_provider() {
    for needle in [
        "KeyProvider",
        "TestKeyProvider",
        "DpapiKeyProvider",
        "key_encryption_key",
        "database_key",
        "cfg!(windows)",
    ] {
        assert!(
            occurrences(needle).is_empty(),
            "the shell names `{needle}`; choosing key material is soulcore's decision",
        );
    }
}

/// The scanner has to be able to fail. Without this, a rename that emptied
/// `shell_sources()` would make every assertion above pass silently.
#[test]
fn the_scanner_notices_the_things_it_is_looking_for() {
    let synthetic = "let keys = TestKeyProvider::in_dir(dir);\nlet s = open_store(dir, &keys);";
    assert_eq!(synthetic.matches("open_store").count(), 1);
    assert!(synthetic.contains("TestKeyProvider"));

    let real: String = shell_sources()
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        real.contains("fn run") && real.contains("fn configure"),
        "the scan read the shell's own sources, not an empty list",
    );
}

/// `configure` is what the mock runtime builds, and `tests/ipc_roundtrip.rs`
/// runs it on a host with no data directory at all. Opening the database there
/// would make every IPC test depend on a disk.
#[test]
fn configure_opens_nothing_and_setup_does() {
    let lib = source("lib.rs");
    let configure = function_body(&lib, "pub fn configure<R: tauri::Runtime>");
    assert!(
        !configure.contains("open_store") && !configure.contains("install_store"),
        "configure must stay buildable without a data directory: {configure}",
    );

    let run = function_body(&lib, "pub fn run()");
    assert!(
        run.contains(".setup(") && run.contains("install_store(app)"),
        "the store has to be opened from setup: {run}",
    );
}

/// The commands the WebView may call are the ones the two inventories agree
/// on, and authorising a directory is now one of them. `command_surface.rs`
/// checks the agreement; this checks that the pair that writes and the pair
/// that reads both arrived.
#[test]
fn the_authorisation_commands_exist_on_both_sides() {
    let commands = source("commands.rs");
    for name in ["authorize_root", "authorized_roots"] {
        assert!(commands.contains(&format!("pub fn {name}(")));
    }

    let core_ts = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("src")
            .join("core.ts"),
    )
    .expect("read core.ts");
    assert!(core_ts.contains("\"authorize_root\""));
    assert!(core_ts.contains("\"authorized_roots\""));
}
