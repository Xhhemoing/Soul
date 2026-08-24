//! WP09's draft page, from the core's side: what a paste turns into, and what
//! the page is told when it cannot turn into anything.
//!
//! `draft_commands.rs` already proves the drafting path itself — the redactor,
//! the plan hash, the single E1 origin, and the audit entries each outcome
//! owes. What is checked here is the layer the WebView actually calls: that a
//! process with no database refuses in a sentence instead of panicking, that
//! an empty box costs nothing, and that the value handed across the IPC says
//! `never_sent` because it cannot say anything else.
//!
//! No mock endpoint is started anywhere in this file. The view path runs under
//! the session a configuration with no endpoint produces, so the route is the
//! local template every time; a test that needed a model running would be
//! testing a branch the desktop cannot reach.

use std::sync::Arc;

use soul_policy::redactor::THIRD_PARTY_PLACEHOLDER;
use soul_schema::audit::{AuditAction, AuditDecision, SoulAuditEntry};
use soul_store_api::AuditLog;
use soulcore::commands::collect::share;
use soulcore::commands::draft::{draft_view, DRAFT_NEVER_SENT_EXPLANATION, TEMPLATE_ROUTE_LABEL};
use soulcore::commands::shell::{Session, ViewRefusedReason, NO_STORE_FOR_VIEW_EXPLANATION};
use soulcore::commands::store::{open_test_store, StoreSlot};

const SEED: &str = "soulcore draft view";

/// Somebody else's message, which is the case the page exists for.
const THIRD_PARTY_LINE: &str = "周五的场地我已经订好了，你直接过来就行";
const SECOND_LINE: &str = "另外记得把上次那份表带上";

/// A real database in a temporary directory, installed in a slot the way a
/// host installs one at startup.
fn fixture() -> (tempfile::TempDir, StoreSlot, Session) {
    let dir = tempfile::tempdir().expect("a directory for the store");
    let slot = StoreSlot::default();
    let store = open_test_store(dir.path(), SEED).expect("open the store");
    assert!(slot.install(share(store)), "an empty slot accepts a handle");
    (dir, slot, Session::new())
}

fn chain(slot: &StoreSlot) -> Vec<SoulAuditEntry> {
    slot.lock()
        .expect("the slot is filled")
        .list_audit()
        .expect("read the audit chain")
}

/// The slot is the whole reason a view can refuse rather than crash, so its
/// two states and its one-way transition are asserted before anything uses it.
#[test]
fn a_slot_is_empty_until_it_is_filled_and_will_not_be_filled_twice() {
    let dir = tempfile::tempdir().expect("a directory for the store");
    let slot = StoreSlot::default();
    assert!(
        slot.lock().is_none(),
        "a default slot has no store, which is the mock runtime's whole situation",
    );

    let handle = share(open_test_store(dir.path(), SEED).expect("open the store"));
    assert!(slot.install(Arc::clone(&handle)));
    assert!(slot.lock().is_some());

    // A second handle is refused rather than swapped in. Two handles on one
    // database is two write-ahead logs, and the caller hears about it here
    // instead of finding out from the disk later.
    let second_dir = tempfile::tempdir().expect("a second directory");
    let second = share(open_test_store(second_dir.path(), SEED).expect("open a second store"));
    assert!(!slot.install(second));

    // And what is still in the slot is the first handle: a draft written
    // through the slot shows up in the chain the original Arc reads.
    draft_view(&slot, &Session::new(), vec![THIRD_PARTY_LINE.to_owned()])
        .expect("the installed store can be drafted against");
    let through_the_arc = handle
        .lock()
        .expect("the handle is not poisoned")
        .list_audit()
        .expect("read the audit chain");
    assert!(
        !through_the_arc.is_empty(),
        "the slot kept a different store than the one it was first given",
    );
}

/// The mock runtime's answer. Both views give it, and both give this sentence.
#[test]
fn a_draft_without_a_store_is_refused_in_the_core_s_own_words() {
    let refused = draft_view(
        &StoreSlot::default(),
        &Session::new(),
        vec![THIRD_PARTY_LINE.to_owned()],
    )
    .expect_err("there is no database to record the request in");

    assert_eq!(refused.reason, ViewRefusedReason::NoStoreOpened);
    assert_eq!(refused.message, NO_STORE_FOR_VIEW_EXPLANATION);
    assert!(
        refused.code.is_none(),
        "no gate decided this; none was open"
    );
    assert_eq!(
        refused.to_string(),
        refused.message,
        "the Display text and the text that crosses the IPC must be one string",
    );

    // The refusal crosses an IPC boundary, so its JSON shape is part of the
    // contract with the WebView.
    let value = serde_json::to_value(&refused).expect("serialize");
    assert_eq!(value["reason"], serde_json::json!("no_store_opened"));
    assert_eq!(value["code"], serde_json::Value::Null);
    assert_eq!(
        value["message"],
        serde_json::json!(NO_STORE_FOR_VIEW_EXPLANATION),
    );
    assert_eq!(value.as_object().expect("an object").len(), 3);
}

/// The one success the page is for, against a real encrypted database.
#[test]
fn a_pasted_message_becomes_a_local_template_draft() {
    let (_dir, slot, session) = fixture();

    // A blank line between two real ones: the empty item is dropped rather
    // than drafted against, and the counts describe what was actually used.
    let view = draft_view(
        &slot,
        &session,
        vec![
            THIRD_PARTY_LINE.to_owned(),
            "   ".to_owned(),
            SECOND_LINE.to_owned(),
        ],
    )
    .expect("a paste and an open store are all a draft needs");

    let value = serde_json::to_value(&view).expect("serialize");
    assert_eq!(
        value["route"],
        serde_json::json!("template"),
        "no endpoint can be configured from the shell, so no draft may claim one",
    );
    assert_eq!(
        value["route_label"],
        serde_json::json!(TEMPLATE_ROUTE_LABEL)
    );
    assert_eq!(value["never_sent"], serde_json::json!(true));
    assert_eq!(value["carries_exempted_original"], serde_json::json!(false));
    assert_eq!(
        value["notice"],
        serde_json::json!(DRAFT_NEVER_SENT_EXPLANATION),
        "the page renders this sentence verbatim; it must come from here",
    );
    assert_eq!(value["turns"], serde_json::json!(2));
    assert_eq!(value["third_party_turns"], serde_json::json!(2));
    assert!(
        value["placeheld_turns"].as_u64().expect("a count") >= 1,
        "an unattributed paste is somebody else's prose and is placeheld",
    );

    let text = value["text"].as_str().expect("a draft is text");
    assert!(!text.is_empty());
    assert!(
        text.contains(THIRD_PARTY_PLACEHOLDER),
        "the material the template quotes back is the redacted material",
    );
    assert!(
        !text.contains(THIRD_PARTY_LINE),
        "the pasted prose reached the draft unredacted",
    );

    // The chain records the draft, and `draft_reply` is the only thing that
    // writes it: the view adds no entry of its own.
    let entries = chain(&slot);
    assert!(
        entries
            .iter()
            .any(|entry| entry.action == AuditAction::DraftCreate
                && entry.decision == AuditDecision::Allowed),
        "a draft that happened has to be in the chain: {entries:?}",
    );
}

/// The empty box is not an event. Nothing is recorded, because nothing was
/// asked for — and a chain that grew a row every time somebody pressed a
/// button with an empty field would be a chain nobody could read.
#[test]
fn an_empty_paste_is_refused_before_anything_is_recorded() {
    let (_dir, slot, session) = fixture();
    assert!(chain(&slot).is_empty(), "a fresh store has nothing in it");

    for pasted in [
        Vec::new(),
        vec![String::new()],
        vec!["   ".to_owned(), "\n\t".to_owned()],
    ] {
        let refused = match draft_view(&slot, &session, pasted.clone()) {
            Ok(view) => panic!("{pasted:?} was drafted against: {view:?}"),
            Err(refused) => refused,
        };
        assert_eq!(
            refused.reason,
            ViewRefusedReason::EmptyPaste,
            "for {pasted:?}"
        );
        assert!(
            !refused.message.trim().is_empty(),
            "a refusal the user reads has to be a sentence: {refused:?}",
        );
        assert!(refused.code.is_none(), "no gate ran, so no gate has a code");
        assert_eq!(
            serde_json::to_value(&refused).expect("serialize")["reason"],
            serde_json::json!("empty_paste"),
        );
    }

    assert!(
        chain(&slot).is_empty(),
        "an empty box is not a request, and a request is what the chain records",
    );
}

/// The field set is the contract with `apps/desktop/src/core.ts`, so it is
/// written down here rather than left to whoever reads the struct next.
#[test]
fn a_draft_view_carries_these_fields_and_no_others() {
    let (_dir, slot, session) = fixture();
    let view = draft_view(&slot, &session, vec![THIRD_PARTY_LINE.to_owned()])
        .expect("a draft the page can show");

    let value = serde_json::to_value(&view).expect("serialize");
    let mut fields: Vec<&str> = value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort_unstable();
    assert_eq!(
        fields,
        vec![
            "carries_exempted_original",
            "never_sent",
            "notice",
            "placeheld_turns",
            "route",
            "route_label",
            "text",
            "third_party_turns",
            "turns",
        ],
    );
}
