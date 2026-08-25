//! `DPAPI → KEK → DEK → soul.db`, end to end, on whichever platform is running.
//!
//! ## Why this file has no `#[cfg]` in it
//!
//! `soulcore`'s `key_provider` picks the platform provider with `cfg!` rather
//! than `#[cfg]`, on the grounds that a Windows arm compiled only on Windows
//! is an arm nobody type-checks until CI. The same reasoning is stronger here,
//! because `soul-store` cannot even be cross-checked: it builds vendored
//! OpenSSL, so `cargo check --target x86_64-pc-windows-msvc` does not run on a
//! Linux host at all.
//!
//! So the tests below are written as one body with a runtime branch. Both
//! halves are compiled everywhere; on Linux the refusal half runs and the
//! success half is dead code the compiler still had to accept, and on
//! `windows-latest` it is the other way round. What cannot be faked — the
//! `CryptProtectData` round trip itself — is tested in `soul-win-dpapi`, which
//! *can* be cross-checked and whose Windows tests are `#[cfg(windows)]`.

use soul_schema::common::Subject;
use soul_schema::event::EventKind;
use soul_store::{DpapiKeyProvider, KeyProvider, SqlCipherStore};
use soul_store_api::EventStore;

mod common;

const BLOB: &str = "keys.dpapi";
const DATABASE: &str = "soul.db";

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().expect("temporary directory")
}

/// On Windows the store opens, survives a close, and reopens under the same
/// protected KEK. Anywhere else nothing opens and nothing is written.
///
/// This is the assertion that says the provider stopped being a skeleton. It
/// is deliberately not tolerant on Windows: a runner where DPAPI does not work
/// is a fact worth seeing, and a test that shrugged at it would let the
/// Windows path rot back to "the store does not open" without anyone noticing.
#[test]
fn the_windows_provider_opens_the_store_and_the_next_launch_opens_the_same_one() {
    let directory = scratch();
    let blob = directory.path().join(BLOB);
    let database = directory.path().join(DATABASE);
    let event_id = common::id("01");

    let provider = DpapiKeyProvider::new(&blob);
    match SqlCipherStore::open(&database, &provider) {
        Ok(mut store) => {
            assert!(
                cfg!(windows),
                "a platform with no DPAPI produced key material anyway",
            );
            assert!(blob.is_file(), "the KEK was never handed to DPAPI");

            store
                .append_event(common::event(
                    event_id,
                    "2026-08-24T11:00:00Z",
                    EventKind::AppForeground,
                    Subject::Owner,
                ))
                .expect("write one row");
            drop(store);

            // The next launch: a new provider, the same blob, the same DEK.
            let next_launch = DpapiKeyProvider::new(&blob);
            let reopened = SqlCipherStore::open(&database, &next_launch)
                .expect("the protected KEK did not reopen the database it created");
            assert_eq!(
                reopened
                    .get_event(event_id)
                    .expect("the row written before the close did not come back")
                    .event_id,
                event_id,
            );
        }
        Err(error) => {
            assert!(
                !cfg!(windows),
                "DPAPI is implemented and Windows still refused: {error}",
            );
            assert!(
                !blob.exists(),
                "a provider that cannot serve this platform wrote key material anyway",
            );
            assert!(
                !database.exists(),
                "the store was created before its key was available",
            );
        }
    }
}

/// The two root secrets are independent keys, not two views of one.
///
/// `TestKeyProvider` derives both from one seed with domain separation;
/// `DpapiKeyProvider` mints two random keys and wraps one under the other.
/// Either way the store must never be handed the same bytes twice, because
/// then destroying a content key would be undone by the database key.
#[test]
fn the_database_key_and_the_kek_are_never_the_same_bytes() {
    let directory = scratch();
    let provider = DpapiKeyProvider::new(directory.path().join(BLOB));

    match (provider.database_key(), provider.key_encryption_key()) {
        (Ok(dek), Ok(kek)) => {
            assert!(cfg!(windows));
            assert_ne!(dek.to_hex(), kek.to_hex());
            assert_eq!(dek.to_hex().len(), 64);
        }
        (Err(_), Err(_)) => assert!(!cfg!(windows)),
        _ => panic!("one root secret was available and the other was not"),
    }
}

/// Whatever the provider says about itself goes into logs and the audit
/// trail, so it has to name the protection and nothing else.
#[test]
fn the_provider_describes_itself_without_naming_a_path_or_a_key() {
    let directory = scratch();
    let blob = directory.path().join(BLOB);
    let described = DpapiKeyProvider::new(&blob).describe();

    assert!(described.contains("dpapi"), "got {described}");
    assert!(
        !described.contains(&blob.display().to_string()),
        "the description repeats the path to the key blob: {described}",
    );
}
