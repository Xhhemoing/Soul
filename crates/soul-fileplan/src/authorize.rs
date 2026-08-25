//! Which directories the user has authorized, and what "inside one" means.
//!
//! AC-18 is one sentence with a hard word in it: given an authorized directory
//! A and an unauthorized directory B, everything under B is refused *one
//! hundred percent of the time*. A percentage like that is a claim about the
//! cases nobody wrote down, so the containment decision here is made twice, in
//! two different ways, and a path has to survive both:
//!
//! 1. **Lexically**, against the spelling of the root the user authorized.
//!    `..` and `.` never get this far — [`crate::screen`] refused them — so the
//!    segments in front of the caller are the segments that will be opened.
//! 2. **Canonically**, after `std::fs::canonicalize` has resolved every
//!    symbolic link on the way. A link inside A that points at B is inside A
//!    lexically and outside it canonically, and the second check is the one
//!    that notices.
//!
//! Between the two, every component below the root is examined with
//! `symlink_metadata` and a link is refused outright rather than followed. That
//! is stricter than containment requires — a link that stays inside A is
//! refused too — and it is the rule that survives being wrong about the
//! filesystem: a scan that never follows a link cannot be walked out of the
//! root by one that appears between the check and the read.
//!
//! Nothing is authorized until somebody calls [`Authorization::authorize`].
//! `Authorization::new()` refuses everything, which is the same default the
//! consent ledger takes in WP07.

use std::path::{Path, PathBuf};

use crate::refusal::Refusal;
use crate::screen::{screen, PathPrefix, ScreenedPath};

/// Whether two path segments that differ only in case name the same file.
///
/// This has to be a decision rather than an assumption. Folding case makes
/// containment match *more* often, so applying NTFS's rule on a case-sensitive
/// filesystem would put `/A/secret` inside a root of `/a` — two different
/// directories, one of them never authorized. The platform default is
/// therefore the platform's actual behaviour, and the variant is spelled out
/// so both can be exercised on the CI host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathMatching {
    /// Segments must match byte for byte. POSIX filesystems.
    Exact,
    /// Segments match ignoring case. NTFS, and every Windows install.
    CaseFolded,
}

impl Default for PathMatching {
    fn default() -> PathMatching {
        PathMatching::for_this_platform()
    }
}

impl PathMatching {
    pub fn for_this_platform() -> PathMatching {
        match cfg!(windows) {
            true => PathMatching::CaseFolded,
            false => PathMatching::Exact,
        }
    }

    pub fn same_segment(self, left: &str, right: &str) -> bool {
        match self {
            PathMatching::Exact => left == right,
            PathMatching::CaseFolded => left.to_lowercase() == right.to_lowercase(),
        }
    }

    fn same_prefix(self, left: PathPrefix, right: PathPrefix) -> bool {
        // Drive letters are already folded to upper case by the screen, on both
        // platforms: there is no filesystem where `c:` and `C:` are two disks.
        left == right
    }

    /// Is `candidate` the root itself, or something under it?
    pub fn contains_screened(self, root: &ScreenedPath, candidate: &ScreenedPath) -> bool {
        if !self.same_prefix(root.prefix(), candidate.prefix()) {
            return false;
        }
        let root_segments = root.segments();
        let candidate_segments = candidate.segments();
        if candidate_segments.len() < root_segments.len() {
            return false;
        }
        root_segments
            .iter()
            .zip(candidate_segments)
            .all(|(a, b)| self.same_segment(a, b))
    }

    /// The same question about two paths the operating system produced.
    ///
    /// Used on canonical paths, where the platform's own spelling — including
    /// the `\\?\` prefix Windows returns — appears on both sides.
    pub fn contains_canonical(self, root: &Path, candidate: &Path) -> bool {
        let root_parts: Vec<String> = component_strings(root);
        let candidate_parts: Vec<String> = component_strings(candidate);
        if candidate_parts.len() < root_parts.len() {
            return false;
        }
        root_parts
            .iter()
            .zip(&candidate_parts)
            .all(|(a, b)| self.same_segment(a, b))
    }
}

fn component_strings(path: &Path) -> Vec<String> {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// A directory the user said Soul may read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedRoot {
    requested: ScreenedPath,
    canonical: PathBuf,
}

impl AuthorizedRoot {
    /// The spelling the user gave, normalized to one form.
    pub fn requested(&self) -> &ScreenedPath {
        &self.requested
    }

    /// The spelling the operating system gave back.
    pub fn canonical(&self) -> &Path {
        &self.canonical
    }

    pub fn display(&self) -> String {
        self.requested.display()
    }
}

/// Where a path ended up, once it was allowed to end up anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    root_index: usize,
    canonical: PathBuf,
    /// Below the root, `/`-separated on every platform. Empty for the root.
    relative: String,
    exists: bool,
}

impl Resolution {
    pub fn root_index(&self) -> usize {
        self.root_index
    }

    pub fn canonical(&self) -> &Path {
        &self.canonical
    }

    pub fn relative(&self) -> &str {
        &self.relative
    }

    /// Whether anything is there yet. A plan names destinations that do not
    /// exist, and they still have to be inside the root.
    pub fn exists(&self) -> bool {
        self.exists
    }

    pub fn is_root(&self) -> bool {
        self.relative.is_empty()
    }
}

/// The set of roots this session may read, and nothing else.
#[derive(Debug, Clone, Default)]
pub struct Authorization {
    roots: Vec<AuthorizedRoot>,
    matching: PathMatching,
}

impl Authorization {
    /// An authorization that covers nothing.
    pub fn new() -> Authorization {
        Authorization::default()
    }

    /// As [`Authorization::new`], with the containment rule stated instead of
    /// inferred from the host. Tests use it to run both rules on one machine.
    pub fn with_matching(matching: PathMatching) -> Authorization {
        Authorization {
            roots: Vec::new(),
            matching,
        }
    }

    pub fn matching(&self) -> PathMatching {
        self.matching
    }

    pub fn roots(&self) -> &[AuthorizedRoot] {
        &self.roots
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Record that the user authorized one directory.
    ///
    /// The directory has to exist, be a directory, and not itself be a symbolic
    /// link: authorizing a link would mean the authorization moves when the
    /// link is repointed, and the user would have consented to a name rather
    /// than to a place.
    pub fn authorize(&mut self, raw: &str) -> Result<AuthorizedRoot, Refusal> {
        let requested = screen(raw)?;
        let shown = requested.display();
        let path = requested.to_path_buf();

        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Refusal::SymlinkNotFollowed { shown });
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(Refusal::NotADirectory { shown });
            }
            Ok(_) => {}
            Err(_) => return Err(Refusal::Unreadable { shown }),
        }

        let canonical = std::fs::canonicalize(&path).map_err(|_| Refusal::Unreadable {
            shown: requested.display(),
        })?;

        let root = AuthorizedRoot {
            requested,
            canonical,
        };
        if let Some(existing) = self.roots.iter().find(|r| **r == root) {
            return Ok(existing.clone());
        }
        self.roots.push(root.clone());
        Ok(root)
    }

    /// Decide whether one path may be read, and where it really is.
    ///
    /// Every refusal from here is an authorization refusal: the caller learns
    /// that the path is not one it may touch, and learns nothing about what is
    /// there. That is on purpose — a refusal that distinguished "outside the
    /// root and exists" from "outside the root and does not" would answer
    /// questions about the rest of the disk.
    pub fn resolve(&self, raw: &str) -> Result<Resolution, Refusal> {
        if self.roots.is_empty() {
            return Err(Refusal::NothingAuthorized);
        }
        let requested = screen(raw)?;
        let shown = requested.display();

        let Some(root_index) = self.roots.iter().position(|root| {
            self.matching
                .contains_screened(root.requested(), &requested)
        }) else {
            return Err(Refusal::OutsideAuthorizedRoot { shown });
        };
        let root = &self.roots[root_index];
        let below = &requested.segments()[root.requested().segments().len()..];

        // Walk down from the canonical root rather than from the filesystem
        // root: whatever links sit above the directory the user authorized are
        // part of the place they authorized, and re-litigating them would refuse
        // every path on a machine where, say, the home directory is a link.
        let mut cursor = root.canonical().to_path_buf();
        for segment in below {
            cursor.push(segment);
            if let Ok(metadata) = std::fs::symlink_metadata(&cursor) {
                if metadata.file_type().is_symlink() {
                    return Err(Refusal::SymlinkNotFollowed { shown });
                }
            }
        }

        let (canonical, exists) =
            canonicalize_deepest(&requested.to_path_buf()).ok_or(Refusal::Unreadable {
                shown: shown.clone(),
            })?;

        if !self
            .matching
            .contains_canonical(root.canonical(), &canonical)
        {
            // Lexically inside, canonically outside: something on the way is a
            // link that leaves the root, and the walk above did not see it
            // because it is above the part of the path this root owns.
            return Err(Refusal::SymlinkEscapesRoot { shown });
        }

        Ok(Resolution {
            root_index,
            canonical,
            relative: below.join("/"),
            exists,
        })
    }

    /// Resolve a path that must be a directory that exists.
    pub fn resolve_directory(&self, raw: &str) -> Result<Resolution, Refusal> {
        let resolution = self.resolve(raw)?;
        let shown = crate::screen::shorten(raw);
        if !resolution.exists() {
            return Err(Refusal::Unreadable { shown });
        }
        match std::fs::symlink_metadata(resolution.canonical()) {
            Ok(metadata) if metadata.is_dir() => Ok(resolution),
            Ok(_) => Err(Refusal::NotADirectory { shown }),
            Err(_) => Err(Refusal::Unreadable { shown }),
        }
    }
}

/// Canonicalize as much of `path` as exists, and keep the rest verbatim.
///
/// A plan proposes destinations that are not there yet, and "is this
/// destination inside the root" has to be answerable about them. Returns
/// `None` only when not even the filesystem root of the path resolves.
fn canonicalize_deepest(path: &Path) -> Option<(PathBuf, bool)> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Some((canonical, true));
    }
    let mut trailing = Vec::new();
    let mut cursor = path.to_path_buf();
    loop {
        let name = cursor.file_name()?.to_owned();
        if !cursor.pop() {
            return None;
        }
        trailing.push(name);
        if let Ok(mut canonical) = std::fs::canonicalize(&cursor) {
            for name in trailing.iter().rev() {
                canonical.push(name);
            }
            return Some((canonical, false));
        }
    }
}
