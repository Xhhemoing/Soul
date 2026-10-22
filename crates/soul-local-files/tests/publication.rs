#![cfg(windows)]

use std::{
    fs,
    path::Path,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};

use soul_local_files::Destination;

const SYNTHETIC: &[u8] = b"synthetic research bundle, no user data";

#[test]
fn complete_file_is_readable_only_after_publication() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    let destination = Destination::prepare(&target).unwrap();
    let cancelled = AtomicBool::new(false);
    let pending = destination.write(SYNTHETIC, &cancelled).unwrap();
    assert!(fs::read(&target).is_err());
    pending.publish().unwrap();
    assert_eq!(fs::read(&target).unwrap(), SYNTHETIC);
}

#[test]
fn dropping_pending_file_removes_unpublished_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    let destination = Destination::prepare(&target).unwrap();
    let cancelled = AtomicBool::new(false);
    drop(destination.write(SYNTHETIC, &cancelled).unwrap());
    assert!(!target.exists());
}

#[test]
fn cancellation_before_write_does_not_create_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    let destination = Destination::prepare(&target).unwrap();
    assert!(destination
        .write(SYNTHETIC, &AtomicBool::new(true))
        .is_err());
    assert!(!target.exists());
}

#[test]
fn cancellation_after_write_prevents_publication_and_removes_file() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    let destination = Destination::prepare(&target).unwrap();
    let cancelled = AtomicBool::new(false);
    let pending = destination.write(SYNTHETIC, &cancelled).unwrap();
    cancelled.store(true, Ordering::Release);
    assert!(pending.publish().is_err());
    assert!(!target.exists());
}

#[test]
fn existing_file_is_never_overwritten() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    fs::write(&target, SYNTHETIC).unwrap();
    assert!(Destination::prepare(&target).is_err());
    assert_eq!(fs::read(&target).unwrap(), SYNTHETIC);
}

#[test]
fn target_created_after_prepare_is_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    let destination = Destination::prepare(&target).unwrap();
    fs::write(&target, SYNTHETIC).unwrap();
    assert!(destination
        .write(b"other", &AtomicBool::new(false))
        .is_err());
    assert_eq!(fs::read(&target).unwrap(), SYNTHETIC);
}

#[test]
fn two_prepared_destinations_cannot_publish_to_the_same_target() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("bundle.zip");
    let first = Destination::prepare(&target).unwrap();
    let second = Destination::prepare(&target).unwrap();
    let cancelled = AtomicBool::new(false);
    let pending = first.write(SYNTHETIC, &cancelled).unwrap();
    assert!(second.write(b"other", &cancelled).is_err());
    pending.publish().unwrap();
    assert!(second.write(b"other", &cancelled).is_err());
    assert_eq!(fs::read(&target).unwrap(), SYNTHETIC);
}

#[test]
fn parent_cannot_be_renamed_while_destination_or_pending_file_is_held() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let target = parent.join("bundle.zip");
    let destination = Destination::prepare(&target).unwrap();
    assert!(fs::rename(&parent, directory.path().join("other")).is_err());
    let cancelled = AtomicBool::new(false);
    let pending = destination.write(SYNTHETIC, &cancelled).unwrap();
    drop(destination);
    assert!(fs::rename(&parent, directory.path().join("other")).is_err());
    drop(pending);
    fs::rename(&parent, directory.path().join("other")).unwrap();
}

#[test]
fn pinned_parent_refuses_write_handles_that_could_change_reparse_metadata() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let open_writer = || {
        fs::OpenOptions::new()
            .access_mode(0x4000_0000)
            .custom_flags(0x0200_0000)
            .share_mode(7)
            .open(&parent)
    };
    drop(open_writer().unwrap());
    let destination = Destination::prepare(&parent.join("bundle.zip")).unwrap();
    let writer = open_writer();
    assert!(writer.is_err());
    drop(destination);
}

#[test]
fn unsafe_path_forms_are_refused() {
    for path in [
        "relative.zip",
        "C:relative.zip",
        r"\\server\share\bundle.zip",
        r"\\?\C:\bundle.zip",
        r"\\.\C:\bundle.zip",
        r"C:\bundle.zip:stream",
        r"C:\parent\..\bundle.zip",
        r"C:\NUL",
        r"C:\bundle.zip.",
    ] {
        assert!(Destination::prepare(Path::new(path)).is_err(), "{path}");
    }
}

#[test]
#[ignore = "requires Windows symbolic-link privilege or developer mode"]
fn reparse_parent_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let actual = directory.path().join("actual");
    fs::create_dir(&actual).unwrap();
    let alias = directory.path().join("alias");
    std::os::windows::fs::symlink_dir(&actual, &alias).unwrap();
    assert!(Destination::prepare(&alias.join("bundle.zip")).is_err());
    assert!(fs::read_dir(&actual).unwrap().next().is_none());
}

#[test]
fn child_exit_before_publish_removes_file_but_after_publish_preserves_it() {
    let directory = tempfile::tempdir().unwrap();
    for publish in [false, true] {
        let target = directory.path().join(if publish {
            "published.zip"
        } else {
            "pending.zip"
        });
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "synthetic_child_exit_helper"])
            .env("SOUL_SYNTHETIC_FILE_TARGET", &target)
            .env(
                "SOUL_SYNTHETIC_FILE_PUBLISH",
                if publish { "1" } else { "0" },
            )
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(37));
        if publish {
            assert_eq!(fs::read(&target).unwrap(), SYNTHETIC);
        } else {
            assert!(!target.exists());
        }
    }
}

#[test]
#[ignore = "subprocess helper; only synthetic parent-supplied temporary path"]
fn synthetic_child_exit_helper() {
    let target = std::env::var_os("SOUL_SYNTHETIC_FILE_TARGET").expect("parent target");
    let destination = Destination::prepare(Path::new(&target)).unwrap();
    let cancelled = AtomicBool::new(false);
    let pending = destination.write(SYNTHETIC, &cancelled).unwrap();
    if std::env::var("SOUL_SYNTHETIC_FILE_PUBLISH").unwrap() == "1" {
        pending.publish().unwrap();
    }
    std::process::exit(37);
}
