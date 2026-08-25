//! Which directories the user has authorized, and the order in which the
//! question gets asked.
//!
//! The order is the security property, so it is written out once here and
//! every entry point goes through [`AuthorizedRoots::authorize`]:
//!
//! 1. the roots are canonicalized when they are accepted, not when they are
//!    used, so a root is resolved once and cannot resolve differently later;
//! 2. an empty root list authorizes nothing — the default this product ships
//!    with is "no directory", and that has to be a refusal rather than a
//!    vacuous pass;
//! 3. the requested path is canonicalized *before* it is compared, because
//!    `A/../B` and "a link in A pointing at B" only look like B afterwards;
//!    a path that cannot be canonicalized cannot be shown to be inside a
//!    root, so it is refused;
//! 4. the comparison is [`Path::starts_with`], which compares whole path
//!    components. A string prefix test would let an authorized `/data/a`
//!    stand in for `/data/ab`.

use std::path::{Path, PathBuf};

use soul_policy::hitl::PlanHash;

use crate::error::FilePlanError;

/// The directories the user has authorized for read-only scanning.
///
/// A newtype rather than a `Vec<PathBuf>` so that "these paths have been
/// canonicalized" is a fact about the value instead of a habit of its
/// callers. [`AuthorizedRoots::canonicalized`] is the only way to make one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedRoots {
    roots: Vec<PathBuf>,
}

impl AuthorizedRoots {
    /// Accept a list of roots, resolving each one now.
    ///
    /// Every root must exist and be a directory: a root that does not resolve
    /// is a configuration error the user can fix, and silently dropping it
    /// would quietly widen or narrow what they authorized without telling
    /// them. Duplicates collapse, and an empty list is accepted — it is the
    /// shipped default, and it authorizes nothing at all.
    pub fn canonicalized(roots: &[PathBuf]) -> Result<AuthorizedRoots, FilePlanError> {
        let mut canonical: Vec<PathBuf> = Vec::new();
        for root in roots {
            let resolved =
                std::fs::canonicalize(root).map_err(|error| FilePlanError::RootUnreadable {
                    path: root.clone(),
                    reason: error.to_string(),
                })?;
            if !resolved.is_dir() {
                return Err(FilePlanError::NotADirectory { path: resolved });
            }
            if !canonical.contains(&resolved) {
                canonical.push(resolved);
            }
        }
        canonical.sort();
        Ok(AuthorizedRoots { roots: canonical })
    }

    /// The roots as resolved. Safe to show a user: they are the paths that
    /// user typed in, resolved.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn len(&self) -> usize {
        self.roots.len()
    }

    /// Decide whether `target` may be looked at, and resolve it while doing so.
    ///
    /// The steps run in the order the module note gives them. Every refusal is
    /// the same value, whatever went wrong, because the difference between
    /// "outside the roots", "does not exist" and "is not readable" is exactly
    /// the difference a caller would use to map a disk it was not authorized
    /// to see.
    pub fn authorize(&self, target: &Path) -> Result<AuthorizedTarget, FilePlanError> {
        if self.roots.is_empty() {
            return Err(self.refusal());
        }
        let Ok(resolved) = std::fs::canonicalize(target) else {
            return Err(self.refusal());
        };
        let Some(root) = self
            .roots
            .iter()
            .find(|root| resolved.starts_with(root.as_path()))
        else {
            return Err(self.refusal());
        };
        Ok(AuthorizedTarget {
            fingerprint: fingerprint(&resolved),
            root: root.clone(),
            path: resolved,
        })
    }

    fn refusal(&self) -> FilePlanError {
        FilePlanError::PathNotAuthorized {
            roots: self.roots.clone(),
        }
    }
}

/// A path that was shown to be inside an authorized root.
///
/// Constructed only by [`AuthorizedRoots::authorize`], so a function that
/// takes one has proof rather than a promise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedTarget {
    fingerprint: String,
    root: PathBuf,
    path: PathBuf,
}

impl AuthorizedTarget {
    /// The canonical path, which is the one to read from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The root that authorized it.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// A stable name for this directory; see [`fingerprint`].
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// The name one directory always answers to.
///
/// Derived rather than minted, because two scans of an unchanged tree have to
/// produce one plan and therefore one plan hash. A fresh identifier per scan
/// would change the hash every time and turn "the user approved this plan"
/// into a refusal for no reason.
///
/// It is deliberately not a UUID and deliberately never written to the audit
/// chain. A digest of a path is not a path, but it does let anyone holding it
/// confirm a guess about one, and the chain is the one place in this product
/// that must carry nothing of the sort. Its home is the plan the user is
/// looking at — which shows them their own file names anyway.
fn fingerprint(resolved: &Path) -> String {
    PlanHash::of(&serde_json::json!({
        "authorized_directory": resolved.to_string_lossy(),
    }))
    .as_str()
    .to_owned()
}
