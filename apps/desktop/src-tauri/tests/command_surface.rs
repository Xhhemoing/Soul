//! The IPC surface, from both ends.
//!
//! `apps/desktop/src/core.ts` is the only place the WebView names a command,
//! and `src/commands.rs` is the only place the shell answers one. Neither can
//! check the other at compile time, so the check is here: the two lists have
//! to be the same list.

use soul_desktop::commands::COMMAND_NAMES;

const COMMANDS_RS: &str = include_str!("../src/commands.rs");
const CORE_TS: &str = include_str!("../../src/core.ts");
const LIB_RS: &str = include_str!("../src/lib.rs");

/// Command names as `core.ts` spells them, read out of the `COMMANDS` object.
fn names_the_webview_uses() -> Vec<String> {
    let start = CORE_TS
        .find("export const COMMANDS")
        .expect("core.ts declares COMMANDS");
    let body = &CORE_TS[start..];
    let end = body.find("} as const;").expect("the object is closed");
    body[..end]
        .lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once(": \"")?;
            let (name, _) = rest.split_once('"')?;
            Some(name.to_owned())
        })
        .collect()
}

#[test]
fn both_sides_name_the_same_commands() {
    let mut webview = names_the_webview_uses();
    webview.sort();
    assert!(!webview.is_empty(), "core.ts names no commands at all");

    let mut shell: Vec<String> = COMMAND_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    shell.sort();

    assert_eq!(webview, shell);
}

/// `COMMAND_NAMES` is a list a human maintains, so it has to be checked
/// against the attributes rather than trusted.
#[test]
fn every_named_command_is_actually_a_command() {
    for name in COMMAND_NAMES {
        let declaration = format!("pub fn {name}(");
        let position = COMMANDS_RS
            .find(&declaration)
            .unwrap_or_else(|| panic!("{name} is listed but not defined"));
        let before = &COMMANDS_RS[..position];
        assert!(
            before.trim_end().ends_with("#[tauri::command]"),
            "{name} is defined but not exposed over the IPC",
        );
        assert!(
            LIB_RS.contains(&format!("commands::{name}")),
            "{name} is defined but not registered with the builder",
        );
    }
}

/// The other direction: a command that exists but is not on the list would be
/// callable from the WebView without appearing in either inventory.
#[test]
fn no_command_exists_outside_the_list() {
    let declared: Vec<&str> = COMMANDS_RS
        .match_indices("#[tauri::command]")
        .filter_map(|(at, _)| {
            let rest = &COMMANDS_RS[at..];
            let start = rest.find("pub fn ")? + "pub fn ".len();
            let end = rest[start..].find('(')? + start;
            Some(rest[start..end].trim())
        })
        .collect();

    for name in &declared {
        assert!(
            COMMAND_NAMES.contains(name),
            "{name} is a command but is not in COMMAND_NAMES",
        );
    }
    assert_eq!(declared.len(), COMMAND_NAMES.len());
}

/// WP09's brief: the shell forwards, it does not decide. A command body long
/// enough to hold a decision is the signal that something moved into the UI
/// layer that belongs in `soulcore`.
#[test]
fn the_command_layer_stays_thin() {
    for name in COMMAND_NAMES {
        let position = COMMANDS_RS
            .find(&format!("pub fn {name}("))
            .expect("the command is defined");
        let body = &COMMANDS_RS[position..];
        let open = body.find('{').expect("a body");
        let close = body[open..].find('}').expect("the body ends") + open;
        let statements = body[open + 1..close]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .count();
        assert!(
            statements <= 1,
            "{name} has {statements} statements; a command that does more than forward \
             is business logic in the shell",
        );
    }
}
