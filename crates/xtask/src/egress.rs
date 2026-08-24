//! `e0-audit`: proves the E0 class has no code path.
//!
//! PRODUCT_LOCK does not say "the cloud switch is off". It says there is no
//! implementation, no vendor domain and no HTTP client. A runtime flag cannot
//! demonstrate that, so the check is structural and runs in CI:
//!
//! 1. no HTTP client crate is reachable through the **normal** dependency
//!    graph of the shipped crates;
//! 2. no source file under the scanned trees contains a URL literal outside a
//!    short allowlist.
//!
//! Development dependencies are exempt. `soul-testkit` deliberately owns an
//! HTTP stack so AC-11 can watch a real request on the wire, and it may only
//! ever be reached through a dev edge. That exemption is enforced here too, by
//! keeping the test tooling out of the set of roots the walk starts from.
//!
//! There is exactly one shipped exception: `soul-egress`, the crate that talks
//! to the endpoint the user configured (E1) and to nothing else. Rule 1 would
//! otherwise ban it outright, so the walk treats it as a gateway — an HTTP
//! client reached *through* `soul-egress` is the sanctioned one and is not
//! reported, while the same crate reached any other way still fails. That
//! narrowing is only worth anything if it stays narrow, so [`audit_gateway`]
//! separately checks that `soul-egress` is the only workspace member naming an
//! HTTP client as a normal dependency, and that it still names one at all: an
//! exception nobody uses is an exception that should be deleted, not a hole
//! left open for the next crate to find.

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, PackageId};

/// Crates that speak HTTP to somewhere other than this machine.
pub const BANNED_HTTP_CLIENTS: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "curl",
    "isahc",
    "attohttpc",
    "surf",
];

/// Tauri plugins that would create an egress path behind the application's
/// back: automatic updates and an arbitrary HTTP bridge into the WebView.
pub const BANNED_TAURI_PLUGINS: &[&str] = &["tauri-plugin-updater", "tauri-plugin-http"];

/// Workspace members that are test instruments rather than shipped code. They
/// are not roots of the normal-dependency walk, which is what makes the
/// dev-dependency exemption concrete.
pub const TEST_TOOLING: &[&str] = &["soul-testkit", "xtask"];

/// The single shipped crate allowed to own an HTTP client.
///
/// `deny.toml` states the same rule for `cargo deny` with a `wrappers` entry.
/// Both have to agree, which is deliberate: one of them is about the licence
/// and advisory graph, the other about what can reach the network at runtime.
pub const EGRESS_GATEWAY: &str = "soul-egress";

/// URL prefixes a source file may contain.
///
/// `soul.local` is a naming authority for JSON Schema `$id`s and is never
/// resolved over the network. The loopback entries exist for the UI-to-core
/// channel and for local model endpoints.
pub const ALLOWED_URL_PREFIXES: &[&str] = &[
    "https://soul.local/schemas/",
    "http://127.0.0.1",
    "https://127.0.0.1",
    "http://localhost",
    "https://localhost",
];

/// Directory names that are exempt everywhere they appear.
pub const EXEMPT_DIRS: &[&str] = &["fixtures", "tests", "target", "node_modules", ".git"];

/// Workspace members exempt from the source scan. `xtask` has to spell the
/// allowlist and the banned names out in order to enforce them.
pub const EXEMPT_CRATES: &[&str] = &["xtask"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlHit {
    pub file: PathBuf,
    pub line: usize,
    pub url: String,
}

impl fmt::Display for UrlHit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.file.display(), self.line, self.url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannedDependency {
    /// Which shipped crate the walk started from.
    pub root: String,
    pub banned: String,
    /// Normal-dependency path from the root to the banned crate.
    pub path: Vec<String>,
}

impl fmt::Display for BannedDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} reaches {} through normal dependencies: {}",
            self.root,
            self.banned,
            self.path.join(" -> "),
        )
    }
}

/// A way the one HTTP-client exception has stopped being one exception.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayFinding {
    /// A workspace member other than the gateway names an HTTP client.
    SecondHolder { crate_name: String, banned: String },
    /// The gateway no longer holds an HTTP client, so the exception carved out
    /// for it now only shelters whatever else is under it.
    ExceptionUnused,
    /// The gateway is not in the workspace at all.
    GatewayMissing,
}

impl fmt::Display for GatewayFinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GatewayFinding::SecondHolder { crate_name, banned } => write!(
                f,
                "{crate_name} depends on {banned}; only {EGRESS_GATEWAY} may hold an HTTP client",
            ),
            GatewayFinding::ExceptionUnused => write!(
                f,
                "{EGRESS_GATEWAY} no longer depends on an HTTP client, \
                 so its exemption should be removed rather than left open",
            ),
            GatewayFinding::GatewayMissing => {
                write!(f, "{EGRESS_GATEWAY} is not a workspace member")
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EgressReport {
    pub banned_dependencies: Vec<BannedDependency>,
    pub gateway_findings: Vec<GatewayFinding>,
    pub url_hits: Vec<UrlHit>,
    /// Roots the dependency walk started from, for the CI log.
    pub roots: Vec<String>,
    pub files_scanned: usize,
}

impl EgressReport {
    pub fn is_clean(&self) -> bool {
        self.banned_dependencies.is_empty()
            && self.gateway_findings.is_empty()
            && self.url_hits.is_empty()
    }
}

impl fmt::Display for EgressReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "e0-audit: {} shipped crate(s) walked, {} source file(s) scanned",
            self.roots.len(),
            self.files_scanned,
        )?;
        for hit in &self.banned_dependencies {
            writeln!(f, "  E0 dependency: {hit}")?;
        }
        for hit in &self.gateway_findings {
            writeln!(f, "  egress gateway: {hit}")?;
        }
        for hit in &self.url_hits {
            writeln!(f, "  E0 url literal: {hit}")?;
        }
        if self.is_clean() {
            write!(f, "  clean")?;
        }
        Ok(())
    }
}

/// Run both halves of the audit against a repository checkout.
pub fn audit(repo_root: &Path) -> Result<EgressReport> {
    let metadata = MetadataCommand::new()
        .manifest_path(repo_root.join("Cargo.toml"))
        .exec()
        .context("running cargo metadata")?;

    let (banned_dependencies, roots) = audit_dependencies(&metadata);
    let gateway_findings = audit_gateway(&metadata);

    let mut url_hits = Vec::new();
    let mut files_scanned = 0usize;
    for tree in ["crates", "apps"] {
        let dir = repo_root.join(tree);
        if !dir.is_dir() {
            continue;
        }
        let scan = scan_tree_for_urls_excluding(&dir, EXEMPT_CRATES)?;
        files_scanned += scan.files_scanned;
        url_hits.extend(scan.hits);
    }

    Ok(EgressReport {
        banned_dependencies,
        gateway_findings,
        url_hits,
        roots,
        files_scanned,
    })
}

/// Walk the normal-dependency closure of every shipped workspace member.
pub fn audit_dependencies(metadata: &Metadata) -> (Vec<BannedDependency>, Vec<String>) {
    audit_dependencies_from_roots(metadata, TEST_TOOLING)
}

/// As [`audit_dependencies`], with the set of excluded roots spelled out.
///
/// Passing an empty exclusion list turns `soul-testkit` into a root and should
/// therefore find its HTTP stack; `tests/self_test.rs` uses that to show the
/// walker is not vacuously clean.
pub fn audit_dependencies_from_roots(
    metadata: &Metadata,
    excluded_roots: &[&str],
) -> (Vec<BannedDependency>, Vec<String>) {
    let mut banned: BTreeSet<&str> = BANNED_HTTP_CLIENTS.iter().copied().collect();
    banned.extend(BANNED_TAURI_PLUGINS.iter().copied());

    let Some(resolve) = metadata.resolve.as_ref() else {
        return (Vec::new(), Vec::new());
    };

    let name_of = |id: &PackageId| -> String {
        metadata
            .packages
            .iter()
            .find(|p| &p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| id.repr.clone())
    };

    let shipped: Vec<&PackageId> = metadata
        .workspace_members
        .iter()
        .filter(|id| !excluded_roots.contains(&name_of(id).as_str()))
        .collect();

    let mut findings = Vec::new();
    let mut roots = Vec::new();

    for root in shipped {
        let root_name = name_of(root);
        roots.push(root_name.clone());

        // `via_gateway` is part of the visited key, not just the state: a crate
        // reachable both through `soul-egress` and around it must still be
        // reported for the second path, and keeping one bit per node would let
        // whichever path BFS happened to reach first silence the other.
        let mut seen: BTreeSet<(&PackageId, bool)> = BTreeSet::new();
        let mut queue: VecDeque<(&PackageId, Vec<String>, bool)> = VecDeque::new();
        let root_is_gateway = root_name == EGRESS_GATEWAY;
        queue.push_back((root, vec![root_name.clone()], root_is_gateway));
        seen.insert((root, root_is_gateway));

        while let Some((current, path, via_gateway)) = queue.pop_front() {
            let Some(node) = resolve.nodes.iter().find(|n| &n.id == current) else {
                continue;
            };
            for dep in &node.deps {
                // Only normal and build edges ship. Dev edges are how the test
                // instruments stay out of the product.
                let ships = dep.dep_kinds.is_empty()
                    || dep
                        .dep_kinds
                        .iter()
                        .any(|k| matches!(k.kind, DependencyKind::Normal | DependencyKind::Build));
                if !ships {
                    continue;
                }
                let dep_name = name_of(&dep.pkg);
                let dep_via_gateway = via_gateway || dep_name == EGRESS_GATEWAY;
                if !seen.insert((&dep.pkg, dep_via_gateway)) {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(dep_name.clone());

                if banned.contains(dep_name.as_str()) {
                    // The gateway's own HTTP client is the sanctioned E1 path.
                    // A Tauri plugin under it is not: those are banned for
                    // opening an egress path behind the application's back,
                    // which the permit check cannot see.
                    let sanctioned = dep_via_gateway
                        && BANNED_HTTP_CLIENTS.contains(&dep_name.as_str())
                        && !BANNED_TAURI_PLUGINS.contains(&dep_name.as_str());
                    if !sanctioned {
                        findings.push(BannedDependency {
                            root: root_name.clone(),
                            banned: dep_name,
                            path: next_path,
                        });
                    }
                    continue;
                }
                queue.push_back((&dep.pkg, next_path, dep_via_gateway));
            }
        }
    }

    (findings, roots)
}

/// Check that the HTTP-client exception is still exactly one crate wide.
///
/// The dependency walk stops objecting to an HTTP client once the path runs
/// through `soul-egress`, which is only safe while `soul-egress` is the only
/// crate that names one. This reads the manifests rather than the resolved
/// graph: what matters is which crate *asked* for the dependency, and that is
/// the thing a reviewer sees in a diff.
pub fn audit_gateway(metadata: &Metadata) -> Vec<GatewayFinding> {
    let members: Vec<&cargo_metadata::Package> = metadata
        .packages
        .iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .collect();

    if !members.iter().any(|p| p.name == EGRESS_GATEWAY) {
        return vec![GatewayFinding::GatewayMissing];
    }

    let mut findings = Vec::new();
    let mut gateway_holds_one = false;

    for package in members {
        let is_gateway = package.name == EGRESS_GATEWAY;
        // Test instruments are allowed an HTTP stack because they can only be
        // reached through a dev edge, which the walk already refuses to follow.
        let exempt = TEST_TOOLING.contains(&package.name.as_str());
        for dependency in &package.dependencies {
            if dependency.kind != DependencyKind::Normal
                || !BANNED_HTTP_CLIENTS.contains(&dependency.name.as_str())
            {
                continue;
            }
            if is_gateway {
                gateway_holds_one = true;
            } else if !exempt {
                findings.push(GatewayFinding::SecondHolder {
                    crate_name: package.name.clone(),
                    banned: dependency.name.clone(),
                });
            }
        }
    }

    if !gateway_holds_one {
        findings.push(GatewayFinding::ExceptionUnused);
    }
    findings
}

#[derive(Debug, Default)]
pub struct UrlScan {
    pub hits: Vec<UrlHit>,
    pub files_scanned: usize,
}

/// Scan a directory tree for URL literals outside the allowlist.
pub fn scan_tree_for_urls(root: &Path) -> Result<UrlScan> {
    scan_tree_for_urls_excluding(root, &[])
}

/// As [`scan_tree_for_urls`], additionally skipping named direct children of
/// `root`. Used to keep this audit from tripping over its own allowlist.
pub fn scan_tree_for_urls_excluding(root: &Path, exempt_children: &[&str]) -> Result<UrlScan> {
    let mut scan = UrlScan::default();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_entry(|e| {
        let is_dir = e.file_type().is_dir();
        if is_exempt_dir(e.path(), is_dir) {
            return false;
        }
        if is_dir && e.path().parent() == Some(root) {
            if let Some(name) = e.path().file_name().and_then(|n| n.to_str()) {
                if exempt_children.contains(&name) {
                    return false;
                }
            }
        }
        true
    }) {
        let entry = entry.with_context(|| format!("walking {}", root.display()))?;
        if !entry.file_type().is_file() || !is_scannable(entry.path()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue; // binary or non-UTF-8; nothing quotable in it
        };
        scan.files_scanned += 1;
        scan.hits.extend(find_url_literals(entry.path(), &text));
    }
    scan.hits.sort_by(|a, b| {
        (a.file.clone(), a.line, a.url.clone()).cmp(&(b.file.clone(), b.line, b.url.clone()))
    });
    Ok(scan)
}

fn is_exempt_dir(path: &Path, is_dir: bool) -> bool {
    if !is_dir {
        return false;
    }
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| EXEMPT_DIRS.contains(&name))
}

fn is_scannable(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "json" | "html" | "css" | "toml" | "conf")
    )
}

/// Every URL literal in `text` that is not covered by the allowlist.
pub fn find_url_literals(file: &Path, text: &str) -> Vec<UrlHit> {
    let mut hits = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for url in extract_urls(line) {
            if ALLOWED_URL_PREFIXES
                .iter()
                .any(|prefix| url.starts_with(prefix))
            {
                continue;
            }
            hits.push(UrlHit {
                file: file.to_path_buf(),
                line: index + 1,
                url,
            });
        }
    }
    hits
}

/// Pull `http://…` and `https://…` runs out of a line.
///
/// A bare scheme with no host, as produced by `format!("http://{addr}")`, is
/// not reported: there is no domain in the source to object to, and the host
/// it will be given is the egress guard's problem, not the grep's.
fn extract_urls(line: &str) -> Vec<String> {
    const TERMINATORS: &[char] = &[
        '"', '\'', '`', ' ', '\t', '<', '>', ')', ']', '}', ',', ';', '\\', '(',
    ];
    let mut urls = Vec::new();
    let bytes = line.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let Some(offset) = line[cursor..].find("://") else {
            break;
        };
        let scheme_end = cursor + offset;
        let scheme_start = line[cursor..scheme_end]
            .rfind(|c: char| !c.is_ascii_alphabetic())
            .map(|i| cursor + i + 1)
            .unwrap_or(cursor);
        let scheme = &line[scheme_start..scheme_end];
        cursor = scheme_end + 3;
        if scheme != "http" && scheme != "https" {
            continue;
        }
        let rest = &line[cursor..];
        let end = rest.find(TERMINATORS).unwrap_or(rest.len());
        let host_and_path = &rest[..end];
        cursor += end.max(1);
        if host_and_path.is_empty() || host_and_path.starts_with('{') {
            continue;
        }
        urls.push(format!("{scheme}://{host_and_path}"));
    }
    urls
}
