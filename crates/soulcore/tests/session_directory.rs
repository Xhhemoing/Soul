//! Where Soul keeps its data, and how that answer is arrived at.
//!
//! One test function in its own file, deliberately. Working the directory out
//! means reading the environment, and setting a variable in one thread while
//! another reads one is the kind of thing that is fine until it is not — so
//! this runs as its own binary and does its steps in order rather than as
//! several tests a runner may schedule side by side.

use std::path::{Path, PathBuf};

use soulcore::commands::session::{
    data_directory, DirectoryError, DATA_DIRECTORY_OVERRIDE, UNIX_DIRECTORY_NAME,
    WINDOWS_DIRECTORY_NAME,
};

#[test]
fn the_data_directory_comes_from_the_platform_or_from_the_override() {
    let saved: Vec<(&str, Option<std::ffi::OsString>)> = [
        DATA_DIRECTORY_OVERRIDE,
        "LOCALAPPDATA",
        "XDG_DATA_HOME",
        "HOME",
    ]
    .into_iter()
    .map(|name| (name, std::env::var_os(name)))
    .collect();

    let clear = || {
        for (name, _) in &saved {
            std::env::remove_var(name);
        }
    };

    // The override wins, and wins outright: a test that had to pass through
    // the platform rules to reach a scratch directory would be a test running
    // against half of a real installation.
    clear();
    std::env::set_var("LOCALAPPDATA", "/should/not/be/used");
    std::env::set_var("XDG_DATA_HOME", "/should/not/be/used");
    std::env::set_var(DATA_DIRECTORY_OVERRIDE, "/tmp/soul-somewhere-else");
    assert_eq!(
        data_directory().expect("the override answers"),
        PathBuf::from("/tmp/soul-somewhere-else"),
    );

    // An empty variable is not an answer. An installer that exports it blank
    // would otherwise put the store at the filesystem root.
    std::env::set_var(DATA_DIRECTORY_OVERRIDE, "");
    let resolved = data_directory().expect("the platform answers");
    assert_ne!(resolved, PathBuf::new());

    clear();
    if cfg!(windows) {
        std::env::set_var("LOCALAPPDATA", "C:/Users/example/AppData/Local");
        assert_eq!(
            data_directory().expect("LOCALAPPDATA answers"),
            Path::new("C:/Users/example/AppData/Local").join(WINDOWS_DIRECTORY_NAME),
        );
    } else {
        std::env::set_var("XDG_DATA_HOME", "/tmp/soul-xdg");
        assert_eq!(
            data_directory().expect("XDG_DATA_HOME answers"),
            Path::new("/tmp/soul-xdg").join(UNIX_DIRECTORY_NAME),
        );

        std::env::remove_var("XDG_DATA_HOME");
        std::env::set_var("HOME", "/tmp/soul-home");
        assert_eq!(
            data_directory().expect("HOME answers"),
            Path::new("/tmp/soul-home")
                .join(".local")
                .join("share")
                .join(UNIX_DIRECTORY_NAME),
        );
    }

    // Nothing to go on is a refusal that names what it looked at, rather than
    // a guess at a directory the user never chose.
    clear();
    let refusal = data_directory().expect_err("nothing says where to put it");
    let DirectoryError::Undiscoverable { variables } = refusal;
    assert!(variables.contains(&DATA_DIRECTORY_OVERRIDE));

    for (name, value) in saved {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}
