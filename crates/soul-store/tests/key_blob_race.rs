//! Two first runs at once, and the database that has to survive them.
//!
//! The key file — `keys.dpapi` on Windows, `soul-test-keys.bin` everywhere
//! else — is the only thing that opens `soul.db`. It used to be written with
//! "write a temporary file, rename it over the final name", which is atomic
//! per writer and says nothing at all about two writers: both see no file,
//! both mint a DEK, both rename, and the second rename takes the database the
//! first process just created away from it for good.
//!
//! So the property under test is not "the file is not torn". It is that every
//! process that came up during the race opened *the same* database, and that
//! the database is the one the surviving key file opens afterwards.
//!
//! Like `dpapi_key_chain.rs`, this file has no `#[cfg]` in it: the provider is
//! chosen at runtime, so Linux CI runs the whole shape against
//! `TestKeyProvider` and `windows-latest` runs it against real DPAPI.

use std::path::Path;
use std::sync::{Arc, Barrier};

use soul_schema::common::Subject;
use soul_schema::event::EventKind;
use soul_store::{DpapiKeyProvider, KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::EventStore;

mod common;

const BLOB: &str = "keys.dpapi";
const DATABASE: &str = "soul.db";
const RACERS: usize = 6;

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().expect("temporary directory")
}

/// What a launch on this platform would construct, and nothing else.
fn launch(directory: &Path) -> Box<dyn KeyProvider> {
    if DpapiKeyProvider::is_available() {
        Box::new(DpapiKeyProvider::new(directory.join(BLOB)))
    } else {
        Box::new(TestKeyProvider::in_dir(directory))
    }
}

/// `SqlCipherStore` sets no busy timeout, so several processes opening one
/// database at the same instant can be told the file is locked. That is a
/// separate question from the one this file asks — it is recoverable, it is
/// reported, and it does not lose a key — so the racers retry through it
/// rather than letting it decide whether the key race passed.
fn with_retries<T>(what: &str, mut attempt: impl FnMut() -> Result<T, String>) -> T {
    let mut last = String::new();
    for _ in 0..200 {
        match attempt() {
            Ok(value) => return value,
            Err(error) if is_contention(&error) => {
                last = error;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) => panic!("{what}: {error}"),
        }
    }
    panic!("{what} never stopped being contended: {last}");
}

fn is_contention(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("locked") || error.contains("busy")
}

/// Every racer opens the same database, and the key file left behind opens it
/// too.
///
/// Each racer writes one row of its own. If any of them had built the store
/// against key material that lost the race, either its own open would have
/// failed against the pages another racer had already written, or the reopen
/// at the end would come back missing that racer's row.
#[test]
fn first_runs_that_race_all_open_the_database_the_surviving_key_file_opens() {
    let directory = scratch();
    let database = directory.path().join(DATABASE);
    let barrier = Arc::new(Barrier::new(RACERS));

    let described: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..RACERS)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                let root = directory.path().to_path_buf();
                let database = database.clone();
                scope.spawn(move || {
                    let keys = launch(&root);
                    barrier.wait();

                    let mut store = with_retries("open the store during the race", || {
                        SqlCipherStore::open(&database, keys.as_ref()).map_err(|e| e.to_string())
                    });
                    with_retries("write one row during the race", || {
                        store
                            .append_event(common::event(
                                common::id(&format!("{:02}", index + 1)),
                                "2026-08-25T09:00:00Z",
                                EventKind::AppForeground,
                                Subject::Owner,
                            ))
                            .map_err(|e| e.to_string())
                    });
                    store.close().expect("close");
                    keys.describe()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("racer"))
            .collect()
    });

    assert_eq!(described.len(), RACERS);
    assert_eq!(
        described
            .iter()
            .filter(|label| *label == &described[0])
            .count(),
        RACERS,
        "the racers did not all use the same kind of key provider",
    );

    // The next launch: a provider that has only ever read the surviving key
    // file, against the database the race built.
    let next_launch = launch(directory.path());
    let reopened = SqlCipherStore::open(&database, next_launch.as_ref())
        .expect("the key file left by the race does not open the database the race created");
    for index in 0..RACERS {
        let event_id = common::id(&format!("{:02}", index + 1));
        assert_eq!(
            reopened
                .get_event(event_id)
                .unwrap_or_else(|error| panic!("racer {index}'s row: {error}"))
                .event_id,
            event_id,
            "racer {index} wrote into a database the surviving key file cannot read",
        );
    }

    assert!(
        !directory.path().join("keys.partial").exists(),
        "the race left a half-written key file behind",
    );
}

/// The root secrets themselves, with the database taken out of the picture.
///
/// This is the same race one level down: if the racers disagree here, nothing
/// above them can agree either.
#[test]
fn racing_first_runs_all_receive_the_root_secrets_that_are_on_disk() {
    let directory = scratch();
    let barrier = Arc::new(Barrier::new(RACERS));

    let seen: Vec<(String, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..RACERS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let root = directory.path().to_path_buf();
                scope.spawn(move || {
                    let keys = launch(&root);
                    barrier.wait();
                    (
                        keys.database_key().expect("a DEK for this racer").to_hex(),
                        keys.key_encryption_key()
                            .expect("a KEK for this racer")
                            .to_hex(),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("racer"))
            .collect()
    });

    let next_launch = launch(directory.path());
    let expected = (
        next_launch
            .database_key()
            .expect("the surviving DEK")
            .to_hex(),
        next_launch
            .key_encryption_key()
            .expect("the surviving KEK")
            .to_hex(),
    );
    assert_ne!(expected.0, expected.1);

    for (index, pair) in seen.iter().enumerate() {
        assert_eq!(
            pair, &expected,
            "racer {index} was handed key material the next launch will not find",
        );
    }
}
