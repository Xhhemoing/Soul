//! E0 has no code path in the shipped desktop binary either.
//!
//! `xtask e0-audit` makes this check for the root workspace, and it cannot
//! make it here: this crate is its own workspace so that a Linux CI host with
//! no WebKitGTK can still run every Rust check in the repository. The rule is
//! the same one, restated against this crate's own graph — an HTTP client is
//! only allowed when the path to it runs through `soul-egress`, which is the
//! single crate permitted to hold one, and a Tauri plugin that opens a network
//! path behind the application's back is not allowed at all.
//!
//! The lists below are copies of the ones in `crates/xtask/src/egress.rs`.
//! They have to be: a `dev-dependency` on `xtask` would put a second workspace
//! member in this tree, and `xtask` is deliberately not shipped. If a name is
//! added there, add it here.

use std::collections::{BTreeSet, VecDeque};
use std::process::Command;

use serde_json::Value;

const BANNED_HTTP_CLIENTS: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "curl",
    "isahc",
    "attohttpc",
    "surf",
];

const BANNED_TAURI_PLUGINS: &[&str] = &["tauri-plugin-updater", "tauri-plugin-http"];

const EGRESS_GATEWAY: &str = "soul-egress";

/// The platform v0.1 ships on, plus the one CI compiles on.
const PLATFORMS: &[&str] = &["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];

struct Graph {
    /// package id -> name
    names: std::collections::BTreeMap<String, String>,
    /// package id -> ids reachable through normal and build edges
    edges: std::collections::BTreeMap<String, Vec<String>>,
    root: String,
}

fn resolve(platform: &str) -> Graph {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            platform,
            "--manifest-path",
            concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
        ])
        .output()
        .expect("running cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed for {platform}: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata is JSON");

    let mut names = std::collections::BTreeMap::new();
    for package in metadata["packages"].as_array().expect("packages") {
        names.insert(
            package["id"].as_str().expect("an id").to_owned(),
            package["name"].as_str().expect("a name").to_owned(),
        );
    }

    let mut edges: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for node in metadata["resolve"]["nodes"].as_array().expect("nodes") {
        let id = node["id"].as_str().expect("an id").to_owned();
        let mut reachable = Vec::new();
        for dep in node["deps"].as_array().expect("deps") {
            let kinds = dep["dep_kinds"].as_array().expect("dep kinds");
            // A null kind is a normal dependency. Development edges do not
            // ship, which is the exemption `soul-testkit` relies on upstream.
            let ships = kinds
                .iter()
                .any(|kind| matches!(kind["kind"].as_str(), None | Some("build")));
            if ships {
                reachable.push(dep["pkg"].as_str().expect("a pkg id").to_owned());
            }
        }
        edges.insert(id, reachable);
    }

    Graph {
        names,
        edges,
        root: metadata["resolve"]["root"]
            .as_str()
            .expect("a resolve root")
            .to_owned(),
    }
}

/// Every banned crate reachable from the shell, with the path that reached it.
fn findings(graph: &Graph) -> Vec<String> {
    let mut found = Vec::new();
    // `via_gateway` is part of the visited key rather than a per-node flag: a
    // crate reachable both through `soul-egress` and around it has to be
    // reported for the second path.
    let mut seen: BTreeSet<(String, bool)> = BTreeSet::new();
    let mut queue: VecDeque<(String, Vec<String>, bool)> = VecDeque::new();

    let root_name = graph.names[&graph.root].clone();
    queue.push_back((graph.root.clone(), vec![root_name], false));
    seen.insert((graph.root.clone(), false));

    while let Some((id, path, via_gateway)) = queue.pop_front() {
        for dependency in graph.edges.get(&id).into_iter().flatten() {
            let name = graph.names[dependency].clone();
            let next_via_gateway = via_gateway || name == EGRESS_GATEWAY;
            if !seen.insert((dependency.clone(), next_via_gateway)) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(name.clone());

            let http_client = BANNED_HTTP_CLIENTS.contains(&name.as_str());
            let plugin = BANNED_TAURI_PLUGINS.contains(&name.as_str());
            if http_client || plugin {
                if !(http_client && next_via_gateway) {
                    found.push(next_path.join(" -> "));
                }
                continue;
            }
            queue.push_back((dependency.clone(), next_path, next_via_gateway));
        }
    }
    found
}

#[test]
fn no_http_client_is_reachable_except_through_soul_egress() {
    for platform in PLATFORMS {
        let graph = resolve(platform);
        let hits = findings(&graph);
        assert!(
            hits.is_empty(),
            "on {platform} the shell reaches an egress crate outside the gateway:\n  {}",
            hits.join("\n  "),
        );
    }
}

/// The exemption above is only narrow while the gateway is still the thing it
/// was carved out for. If `soul-egress` ever stops being reachable, the walk
/// above becomes vacuous and should be simplified rather than left open.
#[test]
fn the_gateway_is_still_the_only_way_an_http_client_arrives() {
    let graph = resolve("x86_64-pc-windows-msvc");

    let reaches_gateway = graph.names.values().any(|name| name == EGRESS_GATEWAY);
    assert!(
        reaches_gateway,
        "{EGRESS_GATEWAY} is not in the graph, so the exemption in this test shelters nothing",
    );

    // And it really does hold one, or the exemption is dead weight.
    let gateway_id = graph
        .names
        .iter()
        .find(|(_, name)| name.as_str() == EGRESS_GATEWAY)
        .map(|(id, _)| id.clone())
        .expect("the gateway is in the graph");
    let holds_client = graph.edges[&gateway_id]
        .iter()
        .any(|dep| BANNED_HTTP_CLIENTS.contains(&graph.names[dep].as_str()));
    assert!(
        holds_client,
        "{EGRESS_GATEWAY} no longer depends on an HTTP client",
    );
}

/// The manifest is where a plugin would be added, and reading it is what a
/// reviewer does. Say it in a test as well.
#[test]
fn the_manifest_names_no_banned_plugin() {
    let manifest = include_str!("../Cargo.toml");
    for plugin in BANNED_TAURI_PLUGINS {
        assert!(
            !manifest.contains(&format!("\n{plugin} "))
                && !manifest.contains(&format!("\n{plugin}=")),
            "the shell depends on {plugin}",
        );
    }
}
