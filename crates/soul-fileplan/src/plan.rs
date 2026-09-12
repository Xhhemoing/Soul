//! The plan: what tidying this directory *would* mean, and a hash over it.
//!
//! Everything in here is a proposal. There is no method on [`OrganizePlan`]
//! that performs one, no method that returns a path to write to, and no field
//! that a later work package can flip to make one happen — the write half is
//! AC-27, which PRODUCT_LOCK puts in v0.1.1. What the type does carry is
//! `executable_in_this_version: false`, inside the value the hash is taken
//! over, so a build that started executing plans would produce different
//! hashes than the build the user approved one in.
//!
//! The rule is deliberately dull, because a preview whose reasoning the user
//! cannot follow is not a preview of anything they can consent to: group the
//! files sitting loose at the top of the directory into one folder per kind,
//! and touch nothing else. Directories stay. Files already in one of those
//! folders stay. Anything whose extension the table does not recognise stays.
//! A move that would land on a name already taken is not proposed at all.
//!
//! [`OrganizePlan::plan_hash`] is `soul_policy::PlanHash` over the canonical
//! JSON, which is the same hash HITL compares at approval time. Change a file
//! on disk and rescan, and the hash changes; that is AC-19's middle case.

use soul_policy::hitl::PlanHash;

use crate::kind::FileKind;
use crate::scan::DirectoryScan;

/// One file this crate would suggest moving, if it could move anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedMove {
    from_relative: String,
    to_relative: String,
    kind: FileKind,
    size_bytes: u64,
}

impl ProposedMove {
    pub fn from_relative(&self) -> &str {
        &self.from_relative
    }

    pub fn to_relative(&self) -> &str {
        &self.to_relative
    }

    pub fn kind(&self) -> FileKind {
        self.kind
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

/// Why something in the directory is not in the move list.
///
/// Present in the plan rather than omitted: a preview that only showed what it
/// wanted to change would leave the user guessing whether the rest was
/// considered or merely missed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeaveReason {
    /// A directory. v0.1 proposes nothing about directories.
    Directory,
    /// Already inside the folder its kind would be moved to.
    AlreadySorted,
    /// Inside some other subdirectory. Only the loose files are grouped.
    Nested,
    /// No folder is defined for this kind.
    UnrecognisedKind,
    /// Something is already called that in the destination folder.
    DestinationTaken,
}

impl LeaveReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            LeaveReason::Directory => "directory",
            LeaveReason::AlreadySorted => "already_sorted",
            LeaveReason::Nested => "nested",
            LeaveReason::UnrecognisedKind => "unrecognised_kind",
            LeaveReason::DestinationTaken => "destination_taken",
        }
    }

    /// What the user is told, in the language the product speaks.
    pub const fn explanation(self) -> &'static str {
        match self {
            LeaveReason::Directory => "这是目录，本版本不动目录",
            LeaveReason::AlreadySorted => "已经在它该在的分类文件夹里",
            LeaveReason::Nested => "在子目录里，本版本只整理散在最外层的文件",
            LeaveReason::UnrecognisedKind => "认不出这是哪一类，不猜",
            LeaveReason::DestinationTaken => "目标文件夹里已经有同名的东西",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftAlone {
    relative: String,
    reason: LeaveReason,
}

impl LeftAlone {
    pub fn relative(&self) -> &str {
        &self.relative
    }

    pub fn reason(&self) -> LeaveReason {
        self.reason
    }
}

/// A tidying plan for one directory, which this version will not carry out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizePlan {
    root_display: String,
    snapshot_hash: String,
    moves: Vec<ProposedMove>,
    left_alone: Vec<LeftAlone>,
    scanned_entries: usize,
    skipped_entries: usize,
    truncated: bool,
}

impl OrganizePlan {
    pub fn root_display(&self) -> &str {
        &self.root_display
    }

    /// The directory snapshot the plan was made from. Two plans over the same
    /// files agree on it; a file appearing changes it.
    pub fn snapshot_hash(&self) -> &str {
        &self.snapshot_hash
    }

    pub fn moves(&self) -> &[ProposedMove] {
        &self.moves
    }

    pub fn left_alone(&self) -> &[LeftAlone] {
        &self.left_alone
    }

    pub fn scanned_entries(&self) -> usize {
        self.scanned_entries
    }

    pub fn skipped_entries(&self) -> usize {
        self.skipped_entries
    }

    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Always false, and part of the hashed value rather than a comment.
    pub const fn executable_in_this_version(&self) -> bool {
        false
    }

    /// The canonical form the hash is taken over.
    ///
    /// `serde_json::Value`'s object is key-sorted, and every list here is built
    /// in sorted order, so the encoding is the same in every process.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "action": soul_policy::hitl::ActionKind::PlanFiles.as_str(),
            "root": self.root_display,
            "directory_snapshot": self.snapshot_hash,
            "executable_in_this_version": self.executable_in_this_version(),
            "scanned_entries": self.scanned_entries,
            "skipped_entries": self.skipped_entries,
            "truncated": self.truncated,
            "moves": self
                .moves
                .iter()
                .map(|proposed| serde_json::json!({
                    "from": proposed.from_relative,
                    "to": proposed.to_relative,
                    "kind": proposed.kind.as_str(),
                    "size_bytes": proposed.size_bytes,
                }))
                .collect::<Vec<_>>(),
            "left_alone": self
                .left_alone
                .iter()
                .map(|left| serde_json::json!({
                    "path": left.relative,
                    "reason": left.reason.as_str(),
                }))
                .collect::<Vec<_>>(),
        })
    }

    pub fn plan_hash(&self) -> PlanHash {
        PlanHash::of(&self.to_json())
    }
}

/// Build the plan a scan implies.
///
/// Pure: it reads the scan and touches nothing. The scan is the only thing that
/// went near the disk, and it went there read-only.
pub fn build(scan: &DirectoryScan) -> OrganizePlan {
    let folders = FileKind::folders();
    let taken: Vec<&str> = scan.entries().iter().map(|e| e.relative()).collect();

    let mut moves: Vec<ProposedMove> = Vec::new();
    let mut left_alone: Vec<LeftAlone> = Vec::new();
    let mut claimed: Vec<String> = Vec::new();

    for entry in scan.entries() {
        let relative = entry.relative().to_owned();

        if entry.is_dir() {
            // Only the top-level directories are worth listing: the ones below
            // them are covered by their parent being left alone.
            if entry.depth() == 1 {
                left_alone.push(LeftAlone {
                    relative,
                    reason: LeaveReason::Directory,
                });
            }
            continue;
        }

        if entry.depth() > 1 {
            let sorted_already = relative
                .split_once('/')
                .and_then(|(head, rest)| (!rest.contains('/')).then_some(head))
                .and_then(FileKind::for_folder)
                .is_some_and(|kind| kind == entry.kind());
            left_alone.push(LeftAlone {
                relative,
                reason: match sorted_already {
                    true => LeaveReason::AlreadySorted,
                    false => LeaveReason::Nested,
                },
            });
            continue;
        }

        let Some(folder) = entry.kind().folder() else {
            left_alone.push(LeftAlone {
                relative,
                reason: LeaveReason::UnrecognisedKind,
            });
            continue;
        };
        debug_assert!(folders.contains(&folder));

        let destination = format!("{folder}/{relative}");
        if taken.contains(&destination.as_str()) || claimed.contains(&destination) {
            left_alone.push(LeftAlone {
                relative,
                reason: LeaveReason::DestinationTaken,
            });
            continue;
        }

        claimed.push(destination.clone());
        moves.push(ProposedMove {
            from_relative: relative,
            to_relative: destination,
            kind: entry.kind(),
            size_bytes: entry.size_bytes(),
        });
    }

    moves.sort_by(|a, b| a.from_relative.cmp(&b.from_relative));
    left_alone.sort_by(|a, b| (&a.relative, a.reason).cmp(&(&b.relative, b.reason)));

    OrganizePlan {
        root_display: scan.root_display().to_owned(),
        snapshot_hash: scan.snapshot_before().hash().to_owned(),
        moves,
        left_alone,
        scanned_entries: scan.entries().len(),
        skipped_entries: scan.skipped().len(),
        truncated: scan.truncated(),
    }
}
