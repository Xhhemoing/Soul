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
            .or_else(|| COMMANDS_RS.find(&format!("pub async fn {name}(")))
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
            let declaration = rest.lines().find(|line| line.starts_with("pub "))?;
            let prefix = if declaration.starts_with("pub async fn ") {
                "pub async fn "
            } else {
                "pub fn "
            };
            let start = rest.find(prefix)? + prefix.len();
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

/// `Session::grant_collect_consent_with_source` exists so AC-09 and AC-10 can
/// be proven on a host with no desktop. It is a way to hand the collector a
/// foreground source of the caller's choosing, and the shipped shell must not
/// have one: a build that could substitute the source could collect from
/// somewhere the user was never told about.
#[test]
fn the_shell_never_hands_the_collector_a_source_of_its_own() {
    for (name, source) in [("commands.rs", COMMANDS_RS), ("lib.rs", LIB_RS)] {
        assert!(
            !source.contains("_with_source"),
            "{name} injects a foreground source; only tests may do that",
        );
        assert!(
            !source.contains("ForegroundSource"),
            "{name} names a foreground source, which is the core's business",
        );
    }
}

/// The address the user types is parsed in one place, and it is not here.
///
/// `soul-policy`'s `Origin::parse` is what the whole egress promise rests on:
/// it decides what counts as the configured endpoint, and the redirect check
/// inside `soul-egress` has to mean the same thing by it. A shell that
/// normalized, defaulted or pre-validated an address before forwarding it would
/// be a second parser, and the two would agree right up until they did not —
/// at which point the origin the user approved and the origin that is contacted
/// would be different strings.
#[test]
fn the_shell_never_reads_an_endpoint_address_itself() {
    for (name, source) in [("commands.rs", COMMANDS_RS), ("lib.rs", LIB_RS)] {
        for parser in ["Origin", "EgressConfig", "NetGuard", "with_user_endpoint"] {
            assert!(
                !source.contains(parser),
                "{name} names {parser}; deciding what an address means is the core's business",
            );
        }
    }
}

/// WP09's brief: the shell forwards, it does not decide. A command body long
/// enough to hold a decision is the signal that something moved into the UI
/// layer that belongs in `soulcore`.
#[test]
fn the_command_layer_stays_thin() {
    for name in COMMAND_NAMES {
        if matches!(
            *name,
            "generate_draft"
                | "person_summary"
                | "preview_plan"
                | "prepare_import"
                | "confirm_import"
        ) {
            // Scheduling is the exception. Policy/body construction
            // must still be in core, and no Session guard may cross execute.
            let start = COMMANDS_RS.find(&format!("pub async fn {name}(")).unwrap();
            let body = COMMANDS_RS[start..]
                .split("#[tauri::command]")
                .next()
                .unwrap();
            assert!(body.contains("tauri::async_runtime::spawn_blocking(move ||"));
            let (begin, complete) = if *name == "generate_draft" {
                ("begin_draft(&approval)", "complete_draft(completed)")
            } else if *name == "person_summary" {
                (
                    "begin_person_summary(&contact_id)",
                    "complete_person_summary(completed)",
                )
            } else if *name == "preview_plan" {
                ("begin_file_plan(&path)", "complete_file_plan(completed)")
            } else if *name == "prepare_import" {
                (
                    "begin_import_prepare_request(request)",
                    "complete_import_prepare(completed)",
                )
            } else {
                (
                    "begin_import(&operation_id, &preview_id)",
                    "complete_import(completed)",
                )
            };
            assert!(body.contains(&format!("let work = session.held().{begin}?;")));
            assert!(body.contains("let completed = work.execute();"));
            assert!(body.contains(&format!("session.held().{complete}")));
            for forbidden in [
                "issue_token",
                "authorize_e1",
                "RedactedBody",
                "let mut guard",
                "let guard",
            ] {
                assert!(
                    !body.contains(forbidden),
                    "{name} scheduling grew policy or a held guard"
                );
            }
            continue;
        }
        let position = COMMANDS_RS
            .find(&format!("pub fn {name}("))
            .expect("the command is defined");
        let body = &COMMANDS_RS[position..];
        let open = body.find('{').expect("a body");
        let close = body[open..].find('}').expect("the body ends") + open;
        let expression = body[open + 1..close]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("//"))
            .collect::<String>()
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let forwarded = if *name == "draft_notices" {
            expression == "DraftNotices::of_this_build()"
        } else if *name == "retry_forget_cleanup" {
            expression == "soulcore::commands::memory::retry_cleanup_for_session(&session.held())"
        } else {
            expression.starts_with("session.held().")
                && expression.ends_with(')')
                && expression.matches("session.held()").count() == 1
        };
        assert!(
            forwarded && !expression.contains([';', '{', '}']),
            "{name} must be one forwarding expression, regardless of rustfmt line breaks",
        );
    }
}
