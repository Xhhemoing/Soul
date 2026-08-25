//! WP07 leftover 8 and WP09 leftover 8: one process, one store handle.
//!
//! Two connections to the same SQLCipher database are two write-ahead logs.
//! Nothing crashes when a second one is opened — the two just stop agreeing
//! about what is committed, in a way that shows up much later as rows that
//! were written and are not there. So the rule is not "be careful", it is
//! "there is one place that opens a database", and this file is what keeps
//! that true as the shell grows routes.
//!
//! The check is a read of this crate's own source. That is a blunt instrument
//! and it is the right one here: a runtime test can only prove that the
//! handles it happened to ask for were the same handle, whereas the thing
//! being ruled out is a *second* call appearing somewhere a test never looks.

use std::sync::Arc;

use soulcore::commands::session::Session;

const COMMANDS_RS: &str = include_str!("../src/commands.rs");
const LIB_RS: &str = include_str!("../src/lib.rs");
const MAIN_RS: &str = include_str!("../src/main.rs");
const TRAY_RS: &str = include_str!("../src/tray.rs");

/// Every source file in the shell, with the file it came from.
fn shell_sources() -> [(&'static str, &'static str); 4] {
    [
        ("src/commands.rs", COMMANDS_RS),
        ("src/lib.rs", LIB_RS),
        ("src/main.rs", MAIN_RS),
        ("src/tray.rs", TRAY_RS),
    ]
}

/// A directory no installation uses.
fn scratch(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("soul-one-store-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// The shell builds a session; it does not open a database.
///
/// `Session::open` is the only entry point in the product that calls
/// `store::open_store`, and the shell reaches it exactly once, in `run`.
#[test]
fn the_shell_opens_a_session_once_and_a_database_never() {
    let openings = LIB_RS.matches("Session::open").count();
    assert_eq!(
        openings, 1,
        "lib.rs reaches for a session {openings} times; the process gets one",
    );

    for (file, source) in shell_sources() {
        for forbidden in [
            "open_store",
            "open_test_store",
            "SqlCipherStore::open",
            "Connection::open",
        ] {
            assert!(
                !source.contains(forbidden),
                "{file} calls `{forbidden}`; the store is opened in soulcore, once",
            );
        }
    }
}

/// The state the commands share is a session, not a store.
///
/// A command that took a `State<'_, SqlCipherStore>` would be one that could
/// be handed a different store than the session's, which is the same failure
/// wearing a type signature.
#[test]
fn every_command_borrows_the_one_session() {
    assert!(
        COMMANDS_RS.contains("pub struct SessionState(Mutex<Session>)"),
        "the shared state is no longer a single session",
    );
    assert!(
        !COMMANDS_RS.contains("State<'_, SqlCipherStore>"),
        "a command takes a store directly rather than through the session",
    );

    for name in soul_desktop::commands::COMMAND_NAMES {
        // `draft_notices` is two constants and a `false`; it needs nothing.
        if *name == "draft_notices" {
            continue;
        }
        let position = COMMANDS_RS
            .find(&format!("pub fn {name}("))
            .expect("the command is defined");
        let signature_end = COMMANDS_RS[position..]
            .find('{')
            .expect("the command has a body")
            + position;
        assert!(
            COMMANDS_RS[position..signature_end].contains("State<'_, SessionState>"),
            "{name} does not take the shared session",
        );
    }
}

/// The runtime half: every route that reads rows reads the same rows.
///
/// `Session::store` hands out an `Arc` clone, so "the same store" is a pointer
/// comparison rather than a claim about behaviour.
#[test]
fn one_session_hands_out_one_store() {
    let directory = scratch("handles");
    let session = Session::open(&directory);

    match (session.store(), session.store()) {
        (Some(first), Some(second)) => assert!(
            Arc::ptr_eq(&first, &second),
            "two calls, two stores; the write-ahead logs would diverge from here",
        ),
        // A machine whose key provider could not produce a key has no handle
        // to compare. Since DPAPI landed that should not happen on Windows —
        // an account with no loaded user profile has no master key, and that
        // is the case this arm is left for. It has to be a session that says
        // the store is closed, not one that quietly opened something else.
        (None, None) => assert!(!session.status().store_opened),
        _ => panic!("the session opened a store between two calls"),
    }

    drop(session);
    let _ = std::fs::remove_dir_all(&directory);
}
