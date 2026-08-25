//! Forgetting an imported person actually destroys what they wrote.
//!
//! `soul-import-v1` has no field for a display name, so every contact a file
//! in that format produces is nameless. That used to make the forget a silent
//! no-op: the only content key a contact recorded was the one behind their
//! display label, and a contact with no label recorded none, so the preview
//! showed zeros, the receipt reported a completed forget, and every sealed
//! body they had written still opened.
//!
//! This runs the whole path — file, commit, preview, forget, restart — against
//! the real encrypted store, and checks the one thing PRODUCT_LOCK promises:
//! after a forget, the ciphertext cannot be opened again.

use std::collections::BTreeMap;

use uuid::Uuid;

use soul_import::model::{ImportSource, ParticipantHandle};
use soul_schema::common::SealedText;
use soul_schema::memory::ForgetState;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::ForgetUnit;
use soul_store_api::types::{EventFilter, StoreError};
use soul_store_api::{BlobStore, EventStore, ForgetOps, GraphStore, SoulStore};
use soul_testkit::fixtures;

const SEED: &str = "p1-2 forget a nameless contact";

/// The two messages `u-lilei` wrote in `valid_basic.jsonl`, and the three
/// nobody else's forget may touch.
const THEIRS: [&str; 2] = ["好的没问题", "今晚在 café 见面聊一下"];
const NOT_THEIRS: [&str; 3] = [
    "明天上午十点在公司门口见",
    "这周先把方案定下来，别拖到下周",
    "团队里那位 👩‍💻 很靠谱",
];

/// Every sealed body in the store, by the text it holds while it can be read.
fn bodies(store: &SqlCipherStore) -> BTreeMap<String, SealedText> {
    store
        .list_events(&EventFilter::all())
        .expect("events")
        .into_iter()
        .filter_map(|event| event.body_ref)
        .map(|sealed| {
            let text = String::from_utf8(store.open(&sealed).expect("readable before the forget"))
                .expect("utf-8");
            (text, sealed)
        })
        .collect()
}

/// The contact the file names `u-lilei`, found the way the importer matched
/// them: by the digest of their platform identifier.
fn contact_for(store: &SqlCipherStore, handle: &str) -> Uuid {
    let wanted = ParticipantHandle::platform_uid(handle).value_hash(ImportSource::SoulImportV1);
    store
        .list_contacts()
        .expect("contacts")
        .into_iter()
        .find(|contact| {
            contact
                .identifiers
                .iter()
                .flatten()
                .any(|identifier| identifier.value_hash == wanted)
        })
        .map(|contact| contact.contact_id)
        .expect("the file named them")
}

#[test]
fn forgetting_a_contact_the_file_never_named_takes_their_sealed_bodies_with_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("soul.db");
    let keys = TestKeyProvider::from_seed(SEED);

    let (them, sealed) = {
        let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");
        let staged = soul_import::soul_import_v1::parse(&text).expect("the fixture is valid");
        assert!(
            staged
                .participants
                .iter()
                .all(|participant| participant.display_label.is_none()),
            "the format carries no display name; that is the case under test",
        );

        let mut store = SqlCipherStore::open(&path, &keys).expect("open");
        soul_import::commit::commit(&mut store, &staged).expect("commit");

        let sealed = bodies(&store);
        assert_eq!(sealed.len(), 5, "one sealed body per message with text");

        let them = contact_for(&store, "u-lilei");
        assert_eq!(
            store.get_contact(them).expect("row").display_label_ref,
            None,
            "nothing named them, so nothing on the row can hold their key",
        );

        let impact = store
            .preview_impact(ForgetUnit::Contact(them))
            .expect("preview");
        assert_eq!(
            impact.content_key_ids.len(),
            1,
            "their bodies are sealed under one key, and the preview has to name it",
        );
        assert_eq!(impact.contacts_affected, 1);
        assert_eq!(
            impact.sealed_blobs_destroyed,
            THEIRS.len() as u64 + 1,
            "the two messages they wrote, plus the blob anchoring the key to them",
        );

        let receipt = store
            .execute_forget(ForgetUnit::Contact(them))
            .expect("forget them");
        assert_eq!(
            receipt.impact, impact,
            "the user must not be shown one number and charged another",
        );

        store.flush().expect("flush");
        store.close().expect("close, as if the application exited");
        (them, sealed)
    };

    // Everything below runs against the file, reopened from scratch.
    let store = SqlCipherStore::open(&path, &keys).expect("reopen after a restart");

    for written in THEIRS {
        let blob = sealed.get(written).expect("their message was sealed");
        assert!(
            matches!(store.open(blob), Err(StoreError::ContentKeyDestroyed(_))),
            "the forget reported destroying this, so it must not open: {written}",
        );
    }
    for written in NOT_THEIRS {
        let blob = sealed.get(written).expect("somebody else's message");
        assert_eq!(
            String::from_utf8(store.open(blob).expect("still readable")).expect("utf-8"),
            written,
            "forgetting one person must not reach anybody else's words",
        );
    }

    assert_eq!(
        store.get_contact(them).expect("row remains").forget_state,
        ForgetState::Forgotten,
        "the row stays as a tombstone, and it has to say what happened to it",
    );
}
