//! Repository guardrails as a command line.
//!
//! `just e0`, `just denylist` and `just schema` are thin wrappers around this.

use anyhow::{bail, Result};

use xtask::{denylist, egress, repo_root, schema_freeze};

const USAGE: &str = "\
xtask — Soul repository guardrails

USAGE:
    cargo run -p xtask -- <command>

COMMANDS:
    e0-audit          No HTTP client in the shipped normal dependency graph and
                      no URL literal outside the loopback/soul.local allowlist.
    denylist-audit    No clinical vocabulary or numeric rating in crate sources.
    schema-freeze     Pin docs/schemas by digest.
        --check       Fail if the documents drifted from the lock (CI default).
        --write       Rewrite the lock. Needs approval to change the contracts.
    all               Run e0-audit, denylist-audit and schema-freeze --check.
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();

    match args.first().map(String::as_str) {
        Some("e0-audit") => run_e0(&root),
        Some("denylist-audit") => run_denylist(&root),
        Some("schema-freeze") => run_schema_freeze(&root, args.get(1).map(String::as_str)),
        Some("all") => {
            run_e0(&root)?;
            run_denylist(&root)?;
            run_schema_freeze(&root, Some("--check"))
        }
        Some("--help") | Some("-h") | None => {
            print!("{USAGE}");
            Ok(())
        }
        Some(other) => {
            eprint!("{USAGE}");
            bail!("unknown command `{other}`");
        }
    }
}

fn run_e0(root: &std::path::Path) -> Result<()> {
    let report = egress::audit(root)?;
    println!("{report}");
    if !report.is_clean() {
        bail!(
            "e0-audit failed: {} banned dependency path(s), {} url literal(s)",
            report.banned_dependencies.len(),
            report.url_hits.len(),
        );
    }
    Ok(())
}

fn run_denylist(root: &std::path::Path) -> Result<()> {
    let report = denylist::audit(root)?;
    println!("{report}");
    if !report.is_clean() {
        bail!("denylist-audit failed: {} hit(s)", report.hits.len());
    }
    Ok(())
}

fn run_schema_freeze(root: &std::path::Path, mode: Option<&str>) -> Result<()> {
    match mode {
        Some("--write") => {
            let path = schema_freeze::write_lock(root)?;
            println!("schema-freeze: rewrote {}", path.display());
            Ok(())
        }
        None | Some("--check") => {
            let drift = schema_freeze::check_lock(root)?;
            if drift.is_empty() {
                println!("schema-freeze: docs/schemas matches the lock");
                return Ok(());
            }
            for entry in &drift {
                eprintln!("  schema drift: {entry}");
            }
            bail!(
                "schema-freeze failed: {} document(s) drifted from docs/schemas/schemas.lock.json. \
                 If the change is approved, rerun with --write and say so in the commit.",
                drift.len(),
            );
        }
        Some(other) => {
            eprint!("{USAGE}");
            bail!("schema-freeze takes --check or --write, not `{other}`");
        }
    }
}
