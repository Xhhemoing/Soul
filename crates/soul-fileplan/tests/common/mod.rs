//! The directory tree every acceptance test in this crate works against, and
//! an independent way of describing it.
//!
//! Two directories side by side: `Alpha`, which the user authorizes, and
//! `Bravo`, which nobody ever does. `alpha` exists too — same letters, other
//! case — because on a case-sensitive filesystem it is a third directory that
//! was never authorized either, and on Windows it is `Alpha` under another
//! spelling. AC-18 needs both readings tested and only one machine to test them
//! on.
//!
//! [`walk`] deliberately does not call [`soul_fileplan::DirectorySnapshot`].
//! The crate's own snapshot is the mechanism under test; "the disk did not
//! change" has to be checkable by something that would still notice if that
//! mechanism were hashing a constant.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One entry, as an outside observer sees it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Observed {
    pub relative: String,
    pub kind: &'static str,
    pub length: u64,
    pub modified_nanos: u128,
}

/// Every entry below `root`, links included, sorted. Opens nothing.
pub fn walk(root: &Path) -> Vec<Observed> {
    let mut out = Vec::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((directory, prefix)) = pending.pop() {
        let Ok(read) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = match prefix.is_empty() {
                true => name,
                false => format!("{prefix}/{name}"),
            };
            let metadata = std::fs::symlink_metadata(entry.path()).expect("entry metadata");
            let file_type = metadata.file_type();
            let kind = if file_type.is_symlink() {
                "link"
            } else if file_type.is_dir() {
                "dir"
            } else {
                "file"
            };
            out.push(Observed {
                relative: relative.clone(),
                kind,
                length: metadata.len(),
                modified_nanos: metadata
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH)
                    .duration_since(UNIX_EPOCH)
                    .map(|since| since.as_nanos())
                    .unwrap_or_default(),
            });
            if file_type.is_dir() && !file_type.is_symlink() {
                pending.push((entry.path(), relative));
            }
        }
    }
    out.sort();
    out
}

#[derive(Debug)]
pub struct Tree {
    directory: tempfile::TempDir,
    base: PathBuf,
    case_variant_is_separate: bool,
}

impl Tree {
    pub fn build() -> Tree {
        let directory = tempfile::tempdir().expect("a temporary directory");
        // Canonical from the start, so that a `/tmp` that is itself a link on
        // some host does not turn every assertion below into a symlink test.
        let base = std::fs::canonicalize(directory.path()).expect("canonical temp dir");

        write(&base.join("Alpha"), "photo.jpg", "jpeg-ish bytes");
        write(&base.join("Alpha"), "report.pdf", "pdf-ish bytes");
        write(
            &base.join("Alpha"),
            "笔记.txt",
            "第三人说过的一些话，不该出现在审计里",
        );
        write(&base.join("Alpha"), "budget.csv", "a,b\n1,2\n");
        write(&base.join("Alpha"), "mystery.qqq", "?");
        write(&base.join("Alpha"), "Makefile", "all:\n");
        write(&base.join("Alpha"), "taken.png", "png-ish bytes");
        write(&base.join("Alpha/图片"), "already.png", "png-ish bytes");
        write(&base.join("Alpha/图片"), "taken.png", "a different png");
        write(&base.join("Alpha/sub"), "inner.md", "# nested");

        write(&base.join("Bravo"), "secret.txt", "第三人的私事");
        write(&base.join("Bravo/nested"), "deep.txt", "更深的私事");

        // Whether two differently cased names are separate is a property of
        // the filesystem, not the operating system. Create the variant first
        // so both paths can be canonicalized, then compare the places they
        // actually name.
        let lower_alpha = base.join("alpha");
        std::fs::create_dir_all(&lower_alpha).expect("create the case-variant directory");
        let case_variant_is_separate = std::fs::canonicalize(&lower_alpha)
            .expect("canonical case variant")
            != std::fs::canonicalize(base.join("Alpha")).expect("canonical Alpha");
        if case_variant_is_separate {
            write(&lower_alpha, "decoy.txt", "同名不同大小写的第三个目录");
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(base.join("Bravo"), base.join("Alpha/escape"))
                .expect("a link out of the authorized root");
            std::os::unix::fs::symlink(base.join("Alpha/sub"), base.join("Alpha/loopback"))
                .expect("a link that stays inside the authorized root");
        }

        Tree {
            directory,
            base,
            case_variant_is_separate,
        }
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Keeps the temporary directory alive for as long as the tree is.
    pub fn keep(&self) -> &tempfile::TempDir {
        &self.directory
    }

    pub fn path_of(&self, relative: &str) -> String {
        self.base
            .join(relative)
            .to_str()
            .expect("a UTF-8 temporary path")
            .to_owned()
    }

    /// The directory the user authorizes.
    pub fn alpha(&self) -> String {
        self.path_of("Alpha")
    }

    /// The directory nobody authorizes, ever.
    pub fn bravo(&self) -> String {
        self.path_of("Bravo")
    }

    /// Same letters as [`Tree::alpha`], other case.
    pub fn lower_alpha(&self) -> String {
        self.path_of("alpha")
    }

    pub fn case_variant_is_separate(&self) -> bool {
        self.case_variant_is_separate
    }
}

fn write(directory: &Path, name: &str, contents: &str) {
    std::fs::create_dir_all(directory).expect("create a fixture directory");
    std::fs::write(directory.join(name), contents).expect("write a fixture file");
}
