//! `sbom`: a CycloneDX bill of materials for what Soul actually ships.
//!
//! Written here rather than delegated to `cargo cyclonedx` for two reasons.
//! The first is the rule this repository is built around: nothing downloads a
//! third-party binary in order to make a build artifact, and `cargo install`ing
//! a generator would be exactly that. The second is that the interesting
//! question — *which* crates ship — already has an answer in this crate.
//! `egress::audit_dependencies` walks the normal and build edges out of the
//! shipped workspace members and refuses to follow dev edges, because
//! `soul-testkit` owns an HTTP stack that must never reach a user. An SBOM
//! assembled from a different set of edges would describe a different program
//! than the one the egress audit clears.
//!
//! Two properties are deliberate and worth naming:
//!
//! * **No URL appears anywhere in the output.** A component's origin is
//!   recorded as a category word (`registry`, `path`, `git`) and its identity
//!   as a purl, which names crates.io by convention rather than by address.
//!   PRODUCT_LOCK bans vendor domains from this repository; an artifact this
//!   repository generates should not be the place they come back.
//! * **The document is a pure function of the lockfile and the manifests.**
//!   No timestamp, and the serial number is derived from the component list,
//!   so two runs on the same checkout produce identical bytes and a diff
//!   between two releases is a diff of the dependencies.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, PackageId};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::egress::TEST_TOOLING;

/// CycloneDX version the documents claim. 1.5 is the newest revision every
/// current consumer reads, and nothing here uses a 1.6 field.
pub const SPEC_VERSION: &str = "1.5";

/// Where `just sbom` puts the documents.
pub const DEFAULT_OUT_DIR: &str = "target/sbom";

/// One SBOM this repository produces.
///
/// Two, because `apps/desktop/src-tauri` is its own cargo workspace (WP09
/// trade-off 1) and is therefore invisible to the root workspace's lockfile.
/// The crates that end up inside `soul.exe` are mostly *its* dependencies, so
/// an SBOM that covered only the root workspace would omit the Tauri tree —
/// that is, almost all of the third-party code the user installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subject {
    /// File stem of the document, and the name of its root component.
    pub name: &'static str,
    /// Manifest to resolve, relative to the repository root.
    pub manifest: &'static str,
    /// Workspace members that are instruments rather than shipped code.
    pub excluded_roots: &'static [&'static str],
    /// Human sentence for the CI log and the artifact listing.
    pub what: &'static str,
}

pub const SUBJECTS: &[Subject] = &[
    Subject {
        name: "soul-core",
        manifest: "Cargo.toml",
        excluded_roots: TEST_TOOLING,
        what: "the Rust core workspace, minus the test instruments",
    },
    Subject {
        name: "soul-desktop",
        manifest: "apps/desktop/src-tauri/Cargo.toml",
        excluded_roots: &[],
        what: "the Tauri host, which is what a Windows installer contains",
    },
];

/// One entry in the bill of materials.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Component {
    pub name: String,
    pub version: String,
    /// SPDX expression where the crate states one, otherwise a `LicenseRef`
    /// pointing at the file it ships instead, otherwise `None`. `None` is not
    /// smoothed over: a crate whose terms nobody wrote down is a finding, not
    /// a blank, and a workspace member is held to the same rule as a
    /// dependency — Soul's own crates carry `LicenseRef-Soul-Proprietary`,
    /// and one that stopped doing so would ship as public domain by silence.
    pub license: Option<String>,
    /// `registry`, `path` or `git`. Never a URL; see the module docs.
    pub origin: &'static str,
    /// From the lockfile. Absent for path and git dependencies, which have no
    /// `.crate` archive to hash.
    pub checksum: Option<String>,
    pub workspace_member: bool,
}

impl Component {
    pub fn purl(&self) -> String {
        format!("pkg:cargo/{}@{}", self.name, self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sbom {
    pub subject: String,
    pub components: Vec<Component>,
    /// `bom-ref` to the refs it depends on, normal and build edges only.
    pub dependencies: BTreeMap<String, BTreeSet<String>>,
    /// Roots the walk started from.
    pub roots: Vec<String>,
    /// Components whose licence the manifests do not state.
    pub unlicensed: Vec<String>,
}

impl fmt::Display for Sbom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} component(s) from {} shipped root(s)",
            self.subject,
            self.components.len(),
            self.roots.len(),
        )?;
        if !self.unlicensed.is_empty() {
            write!(f, ", {} with no stated licence", self.unlicensed.len())?;
        }
        Ok(())
    }
}

impl Sbom {
    /// Licences in the document, each with the crates that carry it.
    pub fn licence_index(&self) -> BTreeMap<String, Vec<String>> {
        let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for component in &self.components {
            index
                .entry(
                    component
                        .license
                        .clone()
                        .unwrap_or_else(|| "UNSTATED".to_owned()),
                )
                .or_default()
                .push(component.purl());
        }
        index
    }

    /// The CycloneDX document.
    pub fn to_cyclonedx(&self) -> Value {
        let components: Vec<Value> = self
            .components
            .iter()
            .map(|component| self.component_value(component))
            .collect();

        let dependencies: Vec<Value> = self
            .dependencies
            .iter()
            .map(|(reference, on)| {
                json!({
                    "ref": reference,
                    "dependsOn": on.iter().cloned().collect::<Vec<_>>(),
                })
            })
            .collect();

        json!({
            "bomFormat": "CycloneDX",
            "specVersion": SPEC_VERSION,
            "serialNumber": self.serial_number(),
            "version": 1,
            "metadata": {
                // A synthetic root: the subject is a set of shipped crates,
                // not one of them, and giving it a purl would claim there is
                // a crate by that name.
                "component": {
                    "type": "application",
                    "bom-ref": format!("soul:{}", self.subject),
                    "name": self.subject,
                },
                "tools": [{
                    "name": "xtask sbom",
                    "vendor": "Soul",
                    "version": env!("CARGO_PKG_VERSION"),
                }],
                "properties": [
                    property("soul:roots", &self.roots.join(",")),
                    property("soul:edges", "normal,build"),
                    property("soul:targets", "all"),
                ],
            },
            "components": components,
            "dependencies": dependencies,
        })
    }

    /// Pretty JSON with a trailing newline, which is what gets written and
    /// what the artifact upload picks up.
    pub fn to_json(&self) -> Result<String> {
        let mut text = serde_json::to_string_pretty(&self.to_cyclonedx())
            .context("serializing the CycloneDX document")?;
        text.push('\n');
        Ok(text)
    }

    fn component_value(&self, component: &Component) -> Value {
        let mut value = Map::new();
        value.insert("type".into(), json!("library"));
        value.insert("bom-ref".into(), json!(component.purl()));
        value.insert("name".into(), json!(component.name));
        value.insert("version".into(), json!(component.version));
        value.insert("purl".into(), json!(component.purl()));
        if let Some(license) = component.license.as_ref() {
            value.insert("licenses".into(), json!([{ "expression": license }]));
        }
        if let Some(checksum) = component.checksum.as_ref() {
            value.insert(
                "hashes".into(),
                json!([{ "alg": "SHA-256", "content": checksum }]),
            );
        }
        value.insert(
            "properties".into(),
            json!([
                property("cargo:origin", component.origin),
                property(
                    "cargo:workspace_member",
                    if component.workspace_member {
                        "true"
                    } else {
                        "false"
                    },
                ),
            ]),
        );
        Value::Object(value)
    }

    /// A UUID derived from the components, so the same checkout always yields
    /// the same document. A random one would make every run a diff.
    fn serial_number(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.subject.as_bytes());
        for component in &self.components {
            hasher.update(component.purl().as_bytes());
            hasher.update([0]);
            hasher.update(component.license.as_deref().unwrap_or("").as_bytes());
            hasher.update([0]);
            hasher.update(component.checksum.as_deref().unwrap_or("").as_bytes());
            hasher.update([0]);
        }
        let digest = hasher.finalize();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        // RFC 9562 version 8: "custom", which is what a content hash is. A v4
        // shape would claim these bytes were random.
        bytes[6] = (bytes[6] & 0x0f) | 0x80;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let hex = hex::encode(bytes);
        format!(
            "urn:uuid:{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32],
        )
    }
}

fn property(name: &str, value: &str) -> Value {
    json!({ "name": name, "value": value })
}

/// Build every SBOM and write it under `out_dir`.
///
/// Returns the documents alongside the paths they were written to, so the
/// caller can print a summary and a test can assert on the contents without
/// reading them back.
pub fn write_all(repo_root: &Path, out_dir: &Path) -> Result<Vec<(PathBuf, Sbom)>> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let mut written = Vec::new();
    for subject in SUBJECTS {
        let manifest = repo_root.join(subject.manifest);
        if !manifest.is_file() {
            anyhow::bail!(
                "{} does not exist; SUBJECTS names a manifest this checkout does not have",
                manifest.display(),
            );
        }
        let sbom = build(&manifest, subject)?;
        let path = out_dir.join(format!("{}.cdx.json", subject.name));
        std::fs::write(&path, sbom.to_json()?)
            .with_context(|| format!("writing {}", path.display()))?;
        written.push((path, sbom));
    }
    Ok(written)
}

/// Resolve one manifest and turn its shipped closure into a bill of materials.
pub fn build(manifest_path: &Path, subject: &Subject) -> Result<Sbom> {
    let metadata = MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .with_context(|| format!("running cargo metadata for {}", manifest_path.display()))?;
    let checksums = read_lock_checksums(
        &manifest_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("Cargo.lock"),
    );
    Ok(from_metadata(&metadata, subject, &checksums))
}

/// The part that is a pure function, so `tests/self_test.rs` can drive it.
pub fn from_metadata(
    metadata: &Metadata,
    subject: &Subject,
    checksums: &BTreeMap<(String, String), String>,
) -> Sbom {
    let by_id: BTreeMap<&PackageId, &Package> =
        metadata.packages.iter().map(|p| (&p.id, p)).collect();
    let is_member = |id: &PackageId| metadata.workspace_members.contains(id);

    let roots: Vec<&PackageId> = metadata
        .workspace_members
        .iter()
        .filter(|id| {
            by_id
                .get(id)
                .is_some_and(|p| !subject.excluded_roots.contains(&p.name.as_str()))
        })
        .collect();

    let mut reachable: BTreeSet<&PackageId> = BTreeSet::new();
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut queue: VecDeque<&PackageId> = VecDeque::new();
    for root in &roots {
        if reachable.insert(root) {
            queue.push_back(root);
        }
    }

    if let Some(resolve) = metadata.resolve.as_ref() {
        while let Some(current) = queue.pop_front() {
            let Some(node) = resolve.nodes.iter().find(|n| &n.id == current) else {
                continue;
            };
            let Some(package) = by_id.get(current) else {
                continue;
            };
            let from = reference_of(package);
            for dep in &node.deps {
                let ships = dep.dep_kinds.is_empty()
                    || dep
                        .dep_kinds
                        .iter()
                        .any(|k| matches!(k.kind, DependencyKind::Normal | DependencyKind::Build));
                if !ships {
                    continue;
                }
                let Some(target) = by_id.get(&dep.pkg) else {
                    continue;
                };
                edges
                    .entry(from.clone())
                    .or_default()
                    .insert(reference_of(target));
                if reachable.insert(&dep.pkg) {
                    queue.push_back(&dep.pkg);
                }
            }
            edges.entry(from).or_default();
        }
    }

    let mut components: Vec<Component> = reachable
        .iter()
        .filter_map(|id| by_id.get(id).copied())
        .map(|package| Component {
            name: package.name.clone(),
            version: package.version.to_string(),
            license: licence_of(package),
            origin: origin_of(package),
            checksum: checksums
                .get(&(package.name.clone(), package.version.to_string()))
                .cloned(),
            workspace_member: is_member(&package.id),
        })
        .collect();
    components.sort();
    components.dedup();

    let unlicensed = components
        .iter()
        .filter(|component| component.license.is_none())
        .map(Component::purl)
        .collect();

    Sbom {
        subject: subject.name.to_owned(),
        components,
        dependencies: edges,
        roots: roots
            .iter()
            .filter_map(|id| by_id.get(id).map(|p| p.name.clone()))
            .collect(),
        unlicensed,
    }
}

fn reference_of(package: &Package) -> String {
    format!("pkg:cargo/{}@{}", package.name, package.version)
}

/// The SPDX expression, normalized off the deprecated `/` spelling.
///
/// A crate with no `license` but a `license-file` is not treated as
/// unlicensed: it ships terms, they are just not an SPDX name, and
/// `LicenseRef-` is how SPDX says exactly that.
fn licence_of(package: &Package) -> Option<String> {
    if let Some(license) = package.license.as_ref() {
        let trimmed = license.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.replace('/', " OR "));
        }
    }
    package.license_file.as_ref().map(|file| {
        let name: String = file
            .file_name()
            .unwrap_or("license-file")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        format!("LicenseRef-{name}")
    })
}

/// Where the crate came from, as a word. Never the registry address; see the
/// module docs.
fn origin_of(package: &Package) -> &'static str {
    match package.source.as_ref() {
        None => "path",
        Some(source) if source.repr.starts_with("git+") => "git",
        Some(_) => "registry",
    }
}

/// `(name, version) -> sha256` out of a `Cargo.lock`.
///
/// Hand-parsed rather than pulled through a TOML crate: the file is a flat
/// list of `[[package]]` tables with unquoted keys and quoted scalars, the
/// three keys wanted here are always on their own line, and a dependency
/// added to a build tool is still a dependency somebody has to review. A
/// lockfile this cannot read yields no hashes rather than a wrong one.
pub fn read_lock_checksums(lock_path: &Path) -> BTreeMap<(String, String), String> {
    let Ok(text) = std::fs::read_to_string(lock_path) else {
        return BTreeMap::new();
    };
    parse_lock_checksums(&text)
}

pub fn parse_lock_checksums(text: &str) -> BTreeMap<(String, String), String> {
    let mut found = BTreeMap::new();
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;

    let flush = |found: &mut BTreeMap<(String, String), String>,
                 name: &Option<String>,
                 version: &Option<String>,
                 checksum: String| {
        if let (Some(name), Some(version)) = (name, version) {
            found.insert((name.clone(), version.clone()), checksum);
        }
    };

    for line in text.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            name = None;
            version = None;
        } else if let Some(value) = scalar(line, "name") {
            name = Some(value);
        } else if let Some(value) = scalar(line, "version") {
            version = Some(value);
        } else if let Some(value) = scalar(line, "checksum") {
            flush(&mut found, &name, &version, value);
        }
    }
    found
}

fn scalar(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
