//! The tidy-up this build would suggest, and never carries out.
//!
//! Every entry here is a suggestion derived from what the scan saw. There is
//! no `execute`, no `apply` and no `undo` in this crate — carrying a plan out
//! is v0.1.1 — so the honest name for the output is a preview, and
//! [`FilePlanPreview::written_to_disk`] answers the only question the type is
//! in a position to answer.
//!
//! Three suggestions, in this order, one per file:
//!
//! * **rename** when the name has stray whitespace, a trailing dot, or an
//!   extension that is not lower case;
//! * **move** when a file is loose at the top of the scanned directory, into
//!   a folder named after its extension;
//! * **group** when a file shares its extension with at least one sibling in
//!   the same folder, into a subfolder named after that extension.
//!
//! A file that is tidy, nested, and alone in its kind gets no suggestion at
//! all, because there is nothing to suggest about it. Directories get none
//! either.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use soul_policy::hitl::ActionKind;
use uuid::Uuid;

use crate::scan::ScanReport;

/// The folder name for files with no extension.
const NO_EXTENSION: &str = "no-extension";

/// What a plan entry proposes. A value, with no behaviour attached: nothing
/// in this crate can act on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanAction {
    Group,
    Move,
    Rename,
}

impl PlanAction {
    pub const ALL: &'static [PlanAction] =
        &[PlanAction::Group, PlanAction::Move, PlanAction::Rename];

    pub const fn as_str(self) -> &'static str {
        match self {
            PlanAction::Group => "group",
            PlanAction::Move => "move",
            PlanAction::Rename => "rename",
        }
    }
}

/// One suggestion about one file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanEntry {
    source_rel: PathBuf,
    action: PlanAction,
    target_rel: Option<PathBuf>,
}

impl PlanEntry {
    /// The file this is about, relative to the scanned directory. It came
    /// from a [`crate::scan::ScanEntry`], so every plan entry can be traced
    /// back to something that was actually on disk.
    pub fn source_rel(&self) -> &Path {
        &self.source_rel
    }

    pub fn action(&self) -> PlanAction {
        self.action
    }

    /// Where the suggestion would put it. A proposed name, not a reservation:
    /// nothing checks that it is free, because nothing is going to use it.
    pub fn target_rel(&self) -> Option<&Path> {
        self.target_rel.as_deref()
    }
}

/// A plan, as the user is shown it and as it is hashed for approval.
///
/// The fields are private and [`plan`] is the only constructor. That is what
/// makes [`FilePlanPreview::written_to_disk`] trustworthy: it is not a field
/// that a caller sets, so there is no argument anywhere that could set it to
/// `true`, and no future caller can be persuaded to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePlanPreview {
    scan_id: Uuid,
    root_fingerprint: String,
    entries: Vec<PlanEntry>,
}

impl FilePlanPreview {
    /// Always `false`, and structurally so; see the type note.
    pub fn written_to_disk(&self) -> bool {
        false
    }

    /// The scan this came from. What the audit entry names.
    pub fn scan_id(&self) -> Uuid {
        self.scan_id
    }

    /// The directory this is about, as a stable digest rather than a path.
    /// Part of the plan, and therefore part of what an approval covers.
    pub fn root_fingerprint(&self) -> &str {
        &self.root_fingerprint
    }

    /// The suggestions, ordered by source path, then action, then target.
    ///
    /// Explicitly ordered for the same reason the scan is: the plan hash is
    /// taken over this, and a plan that reordered itself between two scans of
    /// an unchanged tree would fail its own approval check.
    pub fn entries(&self) -> &[PlanEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// How many entries propose `action`.
    pub fn count_of(&self, action: PlanAction) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.action == action)
            .count()
    }

    /// The plan as the value the user's approval is taken over.
    ///
    /// Every semantic field is in here: the directory, the counts, and each
    /// entry's source, action and target. A hash that left the target out
    /// would approve "rename this file" without approving what to, which is
    /// not an approval. The encoding is explicit — path components joined
    /// with `/` — rather than a debug formatting, so the same tree hashes the
    /// same on a host that spells its separators differently.
    pub fn to_plan_json(&self) -> serde_json::Value {
        serde_json::json!({
            "action": ActionKind::PlanFiles.as_str(),
            "root": self.root_fingerprint,
            "entry_count": self.entries.len(),
            "group_count": self.count_of(PlanAction::Group),
            "move_count": self.count_of(PlanAction::Move),
            "rename_count": self.count_of(PlanAction::Rename),
            "entries": self
                .entries
                .iter()
                .map(|entry| serde_json::json!({
                    "source_rel": encode(&entry.source_rel),
                    "action": entry.action.as_str(),
                    "target_rel": entry.target_rel.as_deref().map(encode),
                }))
                .collect::<Vec<serde_json::Value>>(),
            "written_to_disk": false,
        })
    }

    /// The plan in words, for showing someone.
    ///
    /// Derived from the same entries as [`FilePlanPreview::to_plan_json`], so
    /// what is displayed and what is approved cannot drift apart: there is no
    /// second copy of the plan for a display layer to edit.
    pub fn render(&self) -> String {
        let mut lines = vec![format!(
            "{} suggestion(s). Nothing is carried out and no file is changed.",
            self.entries.len(),
        )];
        for entry in &self.entries {
            lines.push(match entry.target_rel() {
                Some(target) => format!(
                    "  {}: {} -> {}",
                    entry.action.as_str(),
                    encode(&entry.source_rel),
                    encode(target),
                ),
                None => format!("  {}: {}", entry.action.as_str(), encode(&entry.source_rel)),
            });
        }
        lines.join("\n")
    }
}

/// Suggest a tidy-up for what the scan found.
///
/// Pure: it reads the report and touches neither the disk nor the clock.
pub fn plan(report: &ScanReport) -> FilePlanPreview {
    let crowd = extension_crowd(report);
    let mut entries: Vec<PlanEntry> = Vec::new();

    for entry in report.entries().iter().filter(|entry| entry.is_file()) {
        let source_rel = entry.rel_path();
        let parent = parent_of(source_rel);
        let name = name_of(source_rel);
        let extension = extension_of(source_rel);

        let tidied = tidied(&name);
        if tidied != name {
            entries.push(PlanEntry {
                source_rel: source_rel.to_path_buf(),
                action: PlanAction::Rename,
                target_rel: Some(parent.join(tidied)),
            });
            continue;
        }
        if parent.as_os_str().is_empty() {
            entries.push(PlanEntry {
                source_rel: source_rel.to_path_buf(),
                action: PlanAction::Move,
                target_rel: Some(PathBuf::from(&extension).join(&name)),
            });
            continue;
        }
        if crowd
            .get(&(parent.clone(), extension.clone()))
            .copied()
            .unwrap_or_default()
            >= 2
        {
            entries.push(PlanEntry {
                source_rel: source_rel.to_path_buf(),
                action: PlanAction::Group,
                target_rel: Some(parent.join(&extension).join(&name)),
            });
        }
    }

    entries.sort();
    FilePlanPreview {
        scan_id: report.scan_id(),
        root_fingerprint: report.root_fingerprint().to_owned(),
        entries,
    }
}

/// How many files of each extension sit in each folder. The grouping
/// suggestion has no other input, so a folder with one `.csv` in it never
/// produces one.
fn extension_crowd(report: &ScanReport) -> BTreeMap<(PathBuf, String), usize> {
    let mut crowd: BTreeMap<(PathBuf, String), usize> = BTreeMap::new();
    for entry in report.entries().iter().filter(|entry| entry.is_file()) {
        let key = (parent_of(entry.rel_path()), extension_of(entry.rel_path()));
        *crowd.entry(key).or_default() += 1;
    }
    crowd
}

fn parent_of(rel_path: &Path) -> PathBuf {
    rel_path.parent().unwrap_or(Path::new("")).to_path_buf()
}

fn name_of(rel_path: &Path) -> String {
    rel_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The extension, lowercased, or a folder name for files without one.
fn extension_of(rel_path: &Path) -> String {
    rel_path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .filter(|extension| !extension.is_empty())
        .unwrap_or_else(|| NO_EXTENSION.to_owned())
}

/// The name with stray whitespace and trailing dots removed and the extension
/// lowercased. Equal to the original when there is nothing to tidy, which is
/// how the rename suggestion decides whether to say anything.
fn tidied(name: &str) -> String {
    let trimmed = name.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        return name.to_owned();
    }
    let path = Path::new(trimmed);
    let (Some(stem), Some(extension)) = (path.file_stem(), path.extension()) else {
        return trimmed.to_owned();
    };
    format!(
        "{}.{}",
        stem.to_string_lossy().trim(),
        extension.to_string_lossy().to_lowercase(),
    )
}

/// A relative path as one string, components joined with `/`.
fn encode(rel_path: &Path) -> String {
    rel_path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidying_touches_only_what_is_untidy() {
        assert_eq!(tidied("budget.csv"), "budget.csv");
        assert_eq!(tidied("NOTES.MD"), "NOTES.md");
        assert_eq!(tidied("  spaced .txt"), "spaced.txt");
        assert_eq!(tidied("trailing.dot.."), "trailing.dot");
        assert_eq!(tidied(".hidden-leading-dot"), ".hidden-leading-dot");
    }

    #[test]
    fn an_extension_becomes_a_folder_name() {
        assert_eq!(extension_of(Path::new("a/b.CSV")), "csv");
        assert_eq!(extension_of(Path::new("a/plain")), NO_EXTENSION);
    }

    #[test]
    fn paths_encode_with_one_separator_everywhere() {
        assert_eq!(encode(Path::new("inbox")), "inbox");
        assert_eq!(
            encode(&PathBuf::from("inbox").join("budget.csv")),
            "inbox/budget.csv",
        );
    }
}
