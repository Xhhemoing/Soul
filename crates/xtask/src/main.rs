//! Repository guardrails as a command line.
//!
//! `just e0`, `just denylist` and `just schema` are thin wrappers around this.

use anyhow::{bail, Result};

use xtask::{denylist, egress, repo_root, sbom, schema_freeze};

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
    sbom              Write a CycloneDX bill of materials per shipped workspace.
        --out <dir>   Where to write them. Default: target/sbom.
    all               Run e0-audit, denylist-audit and schema-freeze --check.
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();

    match args.first().map(String::as_str) {
        Some("e0-audit") => run_e0(&root),
        Some("denylist-audit") => run_denylist(&root),
        Some("schema-freeze") => run_schema_freeze(&root, args.get(1).map(String::as_str)),
        Some("sbom") => run_sbom(&root, &args[1..]),
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
            "e0-audit failed: {} banned dependency path(s), {} gateway finding(s), {} url literal(s)",
            report.banned_dependencies.len(),
            report.gateway_findings.len(),
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

/// Write the bills of materials and say what went into them.
///
/// A crate with no stated licence fails the command rather than appearing in
/// the document as a blank: `deny.toml` judges an allowlist of SPDX names, and
/// a dependency whose terms nobody wrote down is the one case that list cannot
/// speak to.
fn run_sbom(root: &std::path::Path, args: &[String]) -> Result<()> {
    let out = match args.first().map(String::as_str) {
        Some("--out") => match args.get(1) {
            Some(dir) => std::path::PathBuf::from(dir),
            None => bail!("--out needs a directory"),
        },
        None => root.join(sbom::DEFAULT_OUT_DIR),
        Some(other) => {
            eprint!("{USAGE}");
            bail!("sbom takes --out <dir>, not `{other}`");
        }
    };

    let written = sbom::write_all(root, &out)?;
    let mut unlicensed = Vec::new();
    for (path, document) in &written {
        println!("sbom: {} -> {}", document, path.display());
        for (licence, crates) in document.licence_index() {
            println!("    {:<40} {} crate(s)", licence, crates.len());
        }
        unlicensed.extend(document.unlicensed.iter().cloned());
    }

    if !unlicensed.is_empty() {
        for purl in &unlicensed {
            eprintln!("  no stated licence: {purl}");
        }
        bail!(
            "sbom failed: {} dependency/dependencies state no licence",
            unlicensed.len(),
        );
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
