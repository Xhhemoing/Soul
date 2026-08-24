//! The read-only walk, and the proof that it was read-only.
//!
//! A promise not to change anything is worth what it can be checked against, so
//! the scan takes a [`DirectorySnapshot`] before it starts and another when it
//! finishes, and hands both back. The hash covers every name, size and
//! modification time it can see, which is enough to catch a walk that created a
//! file, truncated one, or touched a timestamp. It is deliberately computed by
//! the same code on both sides: a snapshot the caller could not reproduce would
//! only prove that this module agrees with itself, so
//! [`DirectorySnapshot::of`] is public and the tests take their own.
//!
//! Two things the walk will not do:
//!
//! * **It does not follow symbolic links.** Every entry is examined with
//!   `symlink_metadata`, and a link is recorded as skipped rather than
//!   descended into. `authorize` already refuses a requested path that goes
//!   through one; this is the same rule applied to the paths nobody requested.
//! * **It does not open anything.** Names, sizes and times come from the
//!   directory entry. No file is opened, so no file's access time moves and no
//!   content reaches this crate.
//!
//! File names are external content, and they arrive as
//! [`soul_policy::injection::UntrustedText`] for the same reason an imported
//! line does. AC-25 names the file-name channel explicitly. What a name is
//! *trying* to do is counted for the audit trail and changes nothing else.

use std::path::Path;
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};
use soul_policy::audit::AuditContent;
use soul_policy::injection::{self, UntrustedText};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditCounts, AuditDecision};

use crate::authorize::Authorization;
use crate::kind::FileKind;
use crate::refusal::Refusal;
use crate::screen::{screen_segment, shorten};

/// How far, and how much, one scan will look at.
///
/// Limits rather than exhaustiveness: the user is being shown a preview, and a
/// preview that takes ten minutes on a home directory is a hang. When a limit
/// is reached the scan says so instead of quietly showing less.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanLimits {
    pub max_depth: usize,
    pub max_entries: usize,
}

pub const DEFAULT_MAX_DEPTH: usize = 8;
pub const DEFAULT_MAX_ENTRIES: usize = 20_000;

impl Default for ScanLimits {
    fn default() -> ScanLimits {
        ScanLimits {
            max_depth: DEFAULT_MAX_DEPTH,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }
}

/// One file or directory the scan looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedEntry {
    relative: String,
    name: UntrustedText,
    depth: usize,
    is_dir: bool,
    size_bytes: u64,
    modified_unix_seconds: Option<i64>,
    extension: Option<String>,
    kind: FileKind,
}

impl ScannedEntry {
    /// Below the scanned directory, `/`-separated on every platform.
    pub fn relative(&self) -> &str {
        &self.relative
    }

    /// The file's own name. Data, never instruction.
    pub fn name(&self) -> &UntrustedText {
        &self.name
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn modified_unix_seconds(&self) -> Option<i64> {
        self.modified_unix_seconds
    }

    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    pub fn kind(&self) -> FileKind {
        self.kind
    }
}

/// Why the walk looked at something and moved on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// A symbolic link. v0.1 does not follow them.
    Symlink,
    /// Neither a file nor a directory: a socket, a device, a pipe.
    NotAFileOrDirectory,
    /// The directory entry could not be read.
    Unreadable,
    /// Below [`ScanLimits::max_depth`].
    DepthLimit,
    /// Past [`ScanLimits::max_entries`].
    EntryLimit,
    /// The name itself is one no plan may repeat; see [`crate::screen`].
    UnplannableName,
}

impl SkipReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            SkipReason::Symlink => "symlink",
            SkipReason::NotAFileOrDirectory => "not_a_file_or_directory",
            SkipReason::Unreadable => "unreadable",
            SkipReason::DepthLimit => "depth_limit",
            SkipReason::EntryLimit => "entry_limit",
            SkipReason::UnplannableName => "unplannable_name",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedEntry {
    /// Shortened for display; a skipped name is the one most likely to be
    /// hostile.
    pub shown: String,
    pub reason: SkipReason,
}

/// What a directory looked like at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectorySnapshot {
    hash: String,
    entries: usize,
    bytes: u64,
}

impl DirectorySnapshot {
    /// Hash every name, type, size and modification time below `root`.
    ///
    /// Reads directories, never files. Unreadable entries are folded into the
    /// hash as unreadable, so a directory that becomes unreadable during a scan
    /// changes the snapshot rather than disappearing from it.
    pub fn of(root: &Path, limits: ScanLimits) -> DirectorySnapshot {
        let mut lines: Vec<String> = Vec::new();
        let mut bytes = 0u64;
        let mut pending = vec![(root.to_path_buf(), String::new(), 0usize)];

        while let Some((directory, prefix, depth)) = pending.pop() {
            let Ok(read) = std::fs::read_dir(&directory) else {
                lines.push(format!("{prefix}\u{0}unreadable-dir"));
                continue;
            };
            for entry in read {
                let Ok(entry) = entry else {
                    lines.push(format!("{prefix}\u{0}unreadable-entry"));
                    continue;
                };
                let name = entry.file_name().to_string_lossy().into_owned();
                let relative = join_relative(&prefix, &name);
                match std::fs::symlink_metadata(entry.path()) {
                    Ok(metadata) => {
                        let file_type = metadata.file_type();
                        let tag = if file_type.is_symlink() {
                            "l"
                        } else if file_type.is_dir() {
                            "d"
                        } else if file_type.is_file() {
                            "f"
                        } else {
                            "o"
                        };
                        let modified = metadata
                            .modified()
                            .ok()
                            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                            .map(|since| since.as_nanos().to_string())
                            .unwrap_or_else(|| "-".to_owned());
                        lines.push(format!(
                            "{relative}\u{0}{tag}\u{0}{}\u{0}{modified}",
                            metadata.len(),
                        ));
                        if file_type.is_file() {
                            bytes = bytes.saturating_add(metadata.len());
                        }
                        if file_type.is_dir() && !file_type.is_symlink() && depth < limits.max_depth
                        {
                            pending.push((entry.path(), relative, depth + 1));
                        }
                    }
                    Err(_) => lines.push(format!("{relative}\u{0}unreadable")),
                }
            }
        }

        lines.sort();
        let entries = lines.len();
        let mut digest = Sha256::new();
        for line in &lines {
            digest.update(line.as_bytes());
            digest.update([b'\n']);
        }
        DirectorySnapshot {
            hash: hex::encode(digest.finalize()),
            entries,
            bytes,
        }
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn entries(&self) -> usize {
        self.entries
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

/// One read-only look at one authorized directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryScan {
    root_display: String,
    entries: Vec<ScannedEntry>,
    skipped: Vec<SkippedEntry>,
    before: DirectorySnapshot,
    after: DirectorySnapshot,
    injection_signals: usize,
    truncated: bool,
}

impl DirectoryScan {
    pub fn root_display(&self) -> &str {
        &self.root_display
    }

    pub fn entries(&self) -> &[ScannedEntry] {
        &self.entries
    }

    pub fn skipped(&self) -> &[SkippedEntry] {
        &self.skipped
    }

    pub fn snapshot_before(&self) -> &DirectorySnapshot {
        &self.before
    }

    pub fn snapshot_after(&self) -> &DirectorySnapshot {
        &self.after
    }

    /// Whether the directory is byte for byte, timestamp for timestamp, what it
    /// was when the scan started.
    pub fn disk_unchanged(&self) -> bool {
        self.before == self.after
    }

    /// How many entries carried something that reads like an instruction.
    /// Counted for the audit trail. Nothing branches on it.
    pub fn injection_signals(&self) -> usize {
        self.injection_signals
    }

    /// Whether a limit cut the walk short.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    pub fn files(&self) -> impl Iterator<Item = &ScannedEntry> {
        self.entries.iter().filter(|entry| !entry.is_dir())
    }

    /// A scan happened, over this many entries. No names, no path.
    pub fn audit(&self) -> AuditContent {
        AuditContent::new(AuditAction::FilePlan, AuditDecision::Allowed)
            .because(ReasonCode::Routine)
            .counting(AuditCounts {
                items: Some(self.entries.len() as u64),
                bytes: None,
            })
    }

    /// The `injection.blocked` entry, when a name tried something.
    ///
    /// `bytes` stays empty here as everywhere else: the length of a hostile
    /// file name is still information about the file name.
    pub fn injection_audit(&self) -> Option<AuditContent> {
        (self.injection_signals > 0).then(|| {
            AuditContent::denied(
                AuditAction::InjectionBlocked,
                ReasonCode::InjectionMarkersFound,
            )
            .counting(AuditCounts {
                items: Some(self.injection_signals as u64),
                bytes: None,
            })
        })
    }
}

/// Scan one directory inside one authorized root.
///
/// `raw` may name the root or any directory below it. Anything else is refused
/// by [`Authorization::resolve_directory`] before a single directory is opened.
pub fn scan(
    authorization: &Authorization,
    raw: &str,
    limits: ScanLimits,
) -> Result<DirectoryScan, Refusal> {
    let resolution = authorization.resolve_directory(raw)?;
    let root = resolution.canonical().to_path_buf();

    let before = DirectorySnapshot::of(&root, limits);

    let mut entries: Vec<ScannedEntry> = Vec::new();
    let mut skipped: Vec<SkippedEntry> = Vec::new();
    let mut injection_signals = 0usize;
    let mut truncated = false;
    let mut pending = vec![(root.clone(), String::new(), 0usize)];

    while let Some((directory, prefix, depth)) = pending.pop() {
        let Ok(read) = std::fs::read_dir(&directory) else {
            skipped.push(SkippedEntry {
                shown: shorten(&prefix),
                reason: SkipReason::Unreadable,
            });
            continue;
        };
        for entry in read {
            let Ok(entry) = entry else {
                skipped.push(SkippedEntry {
                    shown: shorten(&prefix),
                    reason: SkipReason::Unreadable,
                });
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = join_relative(&prefix, &name);

            if entries.len() >= limits.max_entries {
                truncated = true;
                skipped.push(SkippedEntry {
                    shown: shorten(&relative),
                    reason: SkipReason::EntryLimit,
                });
                continue;
            }

            let untrusted = UntrustedText::new(name.clone());
            if !injection::scan(&untrusted).is_empty() {
                injection_signals += 1;
            }

            if screen_segment(&name).is_err() {
                skipped.push(SkippedEntry {
                    shown: shorten(&relative),
                    reason: SkipReason::UnplannableName,
                });
                continue;
            }

            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                skipped.push(SkippedEntry {
                    shown: shorten(&relative),
                    reason: SkipReason::Unreadable,
                });
                continue;
            };
            let file_type = metadata.file_type();

            if file_type.is_symlink() {
                skipped.push(SkippedEntry {
                    shown: shorten(&relative),
                    reason: SkipReason::Symlink,
                });
                continue;
            }
            if !file_type.is_dir() && !file_type.is_file() {
                skipped.push(SkippedEntry {
                    shown: shorten(&relative),
                    reason: SkipReason::NotAFileOrDirectory,
                });
                continue;
            }

            let extension = match file_type.is_dir() {
                true => None,
                false => FileKind::extension_of(&name),
            };
            entries.push(ScannedEntry {
                relative: relative.clone(),
                name: untrusted,
                depth: depth + 1,
                is_dir: file_type.is_dir(),
                size_bytes: match file_type.is_dir() {
                    true => 0,
                    false => metadata.len(),
                },
                modified_unix_seconds: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|since| since.as_secs() as i64),
                kind: FileKind::of_extension(extension.as_deref()),
                extension,
            });

            if file_type.is_dir() {
                if depth + 1 < limits.max_depth {
                    pending.push((entry.path(), relative, depth + 1));
                } else {
                    truncated = true;
                    skipped.push(SkippedEntry {
                        shown: shorten(&relative),
                        reason: SkipReason::DepthLimit,
                    });
                }
            }
        }
    }

    entries.sort_by(|a, b| a.relative.cmp(&b.relative));
    skipped.sort_by(|a, b| (&a.shown, a.reason.as_str()).cmp(&(&b.shown, b.reason.as_str())));

    let after = DirectorySnapshot::of(&root, limits);

    Ok(DirectoryScan {
        root_display: root.to_string_lossy().into_owned(),
        entries,
        skipped,
        before,
        after,
        injection_signals,
        truncated,
    })
}

fn join_relative(prefix: &str, name: &str) -> String {
    match prefix.is_empty() {
        true => name.to_owned(),
        false => format!("{prefix}/{name}"),
    }
}
