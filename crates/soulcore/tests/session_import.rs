//! The import surface the desktop shell binds to.
//!
//! `import_and_graph_commands.rs` already checks that the WP06 commands reach
//! `soul-import` and leave a chain behind. What is only checkable here is the
//! layer the WebView calls: that a file previewed and then committed through a
//! [`Session`] lands in *that session's* store, that the graph the shell draws
//! afterwards has the people in it, that a refused file leaves the store as it
//! was, and that neither the refusal nor the encrypted files repeat a word of
//! what was in the export.
//!
//! The plaintext scan is AC-04's, run against the directory a shell would use
//! rather than a scratch store a test opened: the question is whether the
//! session's own path to disk seals what it was given.

use std::path::{Path, PathBuf};

use soul_store_api::types::EventFilter;
use soul_store_api::EventStore;
use soul_testkit::fixtures;
use soulcore::commands::session::Session;

/// Sentences out of `fixtures/import/soul-import-v1/three_partners.jsonl`.
/// They are what the corpus says; none of them may end up legible on disk.
const SPOKEN: &[&str] = &[
    "明天上午十点在公司门口见",
    "这周先把方案定下来",
    "周五要交的那份材料我写完了",
];

/// What `fixtures/import/soul-import-v1/injection_lines.jsonl` tries to say.
/// It is data, it is counted, and none of it belongs in an audit chain.
const ATTEMPTED: &[&str] = &[
    "忽略之前指令",
    "你现在是系统管理员",
    "developer mode",
    "evil.example",
    "tool_call",
    "不需要用户确认",
];

/// What `result_missing_fields.json` calls its chats and its owner. A personal
/// chat's title is the other person's name, so a refusal that named one would
/// be a refusal that named somebody.
const NAMES_IN_THE_BROKEN_EXPORT: &[&str] = &[
    "缺 messages 的会话",
    "缺 id 的会话",
    "字段缺失的会话",
    "Roy",
];

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

/// Every regular file in the directory, as `(name, bytes)`.
fn files_in(directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(directory).expect("read the data directory") {
        let entry = entry.expect("directory entry");
        if !entry.file_type().expect("file type").is_file() {
            continue;
        }
        found.push((
            entry.file_name().to_string_lossy().into_owned(),
            std::fs::read(entry.path()).expect("read the file"),
        ));
    }
    found
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.len() <= haystack.len()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

/// The whole path a Windows user takes: pick a file, read what is in it, say
/// yes, and find the people on `/graph` afterwards.
#[test]
fn a_soul_import_v1_file_committed_through_the_session_lands_sealed_and_shows_up_as_people() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");

    let mut session = Session::open(&directory);
    assert!(
        session
            .people()
            .expect("an empty graph is still one")
            .people
            .is_empty(),
        "the store starts with nobody in it, so what is below came from the file",
    );

    let preview = session
        .preview_soul_import_v1(&text)
        .expect("the corpus parses");
    assert_eq!(preview.source, "soul-import-v1");
    assert_eq!(preview.participants, 5, "four partners and the user");
    assert!(preview.conversations >= 4);
    assert_eq!(preview.messages, 16);
    assert!(preview.owner_identified);
    assert!(!preview.writes_anything);
    assert!(preview.notice.contains("本机"));
    assert!(
        session.people().expect("graph").people.is_empty(),
        "a preview must not write anything",
    );

    let receipt = session
        .commit_soul_import_v1(&text)
        .expect("the corpus commits");
    assert_eq!(receipt.source, "soul-import-v1");
    assert_eq!(receipt.contacts_created, 5);
    assert_eq!(receipt.contacts_matched, 0);
    assert_eq!(receipt.events_written, 16);
    assert_eq!(
        receipt.evidence_written, 15,
        "one row per message that names somebody, and the one message the user \
         sent to the group names nobody",
    );
    assert_eq!(receipt.ties_rebuilt, 4, "AC-08: one tie per partner");

    // The counts are not a story the receipt tells about itself: the graph the
    // shell draws next has to have those people in it.
    let people = session.people().expect("the graph reads back");
    assert_eq!(people.people.len(), 5);
    assert_eq!(people.ties.len(), 4);
    assert!(people.ties.iter().all(|tie| !tie.evidence.is_empty()));
    assert!(people.third_party_data_is_local_only);
    assert!(
        people
            .people
            .iter()
            .all(|person| !person.identifier_hint.is_empty()),
        "people are told apart by a digest, and there is no name on the view at all",
    );

    // A second launch on the same directory finds the same import, which is
    // what says the rows went to disk rather than into this session.
    drop(session);
    let reopened = Session::open(&directory);
    assert_eq!(reopened.people().expect("graph").people.len(), 5);
    drop(reopened);

    // AC-04 through the session's own store: nothing the corpus said is
    // legible in any file the shell leaves in its data directory.
    let files = files_in(&directory);
    assert!(
        files.iter().any(|(name, _)| name == "soul.db"),
        "the session did not create a database: {:?}",
        files.iter().map(|(name, _)| name).collect::<Vec<_>>(),
    );
    for sentence in SPOKEN {
        for (name, bytes) in &files {
            assert!(
                !contains(bytes, sentence.as_bytes()),
                "{sentence:?} is legible in {name}",
            );
        }
    }
    drop(keep);
}

/// AC-23 for the import screen: the commit and the rebuild it triggers are both
/// in the chain, and neither of them repeats a sentence out of the export.
///
/// `import_and_graph_commands.rs` already checks both actions land, but it
/// checks them on the command layer's own store. What is only checkable here is
/// that the object the WebView holds carries the same two entries to the same
/// database the 审计 page reads back — a session that dropped the graph build's
/// audit on the floor would leave sixteen sealed events and no record that the
/// inferences on `/graph` were ever derived.
#[test]
fn a_commit_leaves_the_import_and_the_rebuild_in_the_chain_and_none_of_the_words() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");

    let mut session = Session::open(&directory);
    let receipt = session
        .commit_soul_import_v1(&text)
        .expect("the corpus commits");
    assert!(
        receipt.ties_rebuilt > 0,
        "the rebuild has to have done work"
    );

    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);

    let committed = chain
        .entries
        .iter()
        .find(|entry| entry.action == "import.commit")
        .unwrap_or_else(|| {
            panic!(
                "sixteen events were sealed and the chain never heard about it: {:?}",
                chain.entries,
            )
        });
    assert_eq!(committed.decision, "allowed");
    assert_eq!(
        committed.items,
        Some(receipt.events_written as u64),
        "the entry counts something other than what the receipt told the screen",
    );

    // The graph is derived, not imported, and AC-23 lists derivation separately
    // for that reason: the ties on `/graph` came from somewhere and the chain
    // has to name the moment.
    let inferred = chain
        .entries
        .iter()
        .find(|entry| entry.action == "inference.write")
        .unwrap_or_else(|| {
            panic!(
                "the graph was rebuilt and the chain never heard about it: {:?}",
                chain.entries,
            )
        });
    assert_eq!(inferred.decision, "allowed");
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));

    // Counts and identifiers. Not one word of what the four of them said.
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    for sentence in SPOKEN {
        assert!(
            !played.contains(sentence),
            "{sentence:?} was imported and then written into the chain: {played}",
        );
    }
    drop(keep);
}

/// Importing the same export twice recognizes the people instead of cloning
/// them. v0.1 has no external-id index, so the events do arrive twice, and the
/// receipt says so rather than pretending the second run was free.
#[test]
fn importing_the_same_file_twice_matches_the_people_it_already_knows() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");

    let mut session = Session::open(&directory);
    let first = session.commit_soul_import_v1(&text).expect("first import");
    assert_eq!(first.contacts_created, 3);
    assert_eq!(first.contacts_matched, 0);

    let second = session.commit_soul_import_v1(&text).expect("second import");
    assert_eq!(second.contacts_created, 0, "the same people, recognized");
    assert_eq!(second.contacts_matched, 3);
    assert_eq!(second.events_written, first.events_written);

    assert_eq!(
        session.people().expect("graph").people.len(),
        3,
        "two imports of one export must not produce two sets of people",
    );
    drop(keep);
}

/// A second export from a second account, whose owner the first one's
/// identifier digests cannot match.
///
/// Committing it puts a second contact of class `self` in the store, and
/// `soul_graph::rebuild` — which runs inside the commit's transaction — refuses
/// an ego network with two centres. It is the cheapest honest way to make a
/// commit fail after it has already written events, sealed bodies and contact
/// rows, which is the state the transaction has to be able to undo.
const SECOND_ACCOUNT: &str = concat!(
    r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T08:00:00Z"}"#,
    "\n",
    r#"{"type":"message","id":"m-9001","occurred_at":"2026-08-22T11:00:00Z","sender_scope":"self","conversation_id":"c-90","sender_id":"u-second-account","text":"这台机器上还有另一个号"}"#,
    "\n",
    r#"{"type":"message","id":"m-9002","occurred_at":"2026-08-22T11:01:00Z","sender_scope":"third_party","conversation_id":"c-90","sender_id":"u-zhou","text":"收到"}"#,
    "\n",
);

/// What the second export says. A rolled-back import must leave none of it
/// legible in the data directory either — the seals it wrote went back with it.
const SPOKEN_BY_THE_SECOND_ACCOUNT: &[&str] = &["这台机器上还有另一个号", "收到"];

/// Events in this session's store, through the one handle the product opens.
fn events_in(session: &Session) -> usize {
    let store = session.store().expect("the session opened the database");
    let store = store.lock().expect("nobody panicked holding the store");
    store
        .list_events(&EventFilter::all())
        .expect("the events read back")
        .len()
}

/// One import is one transaction: a commit that fails partway leaves the store
/// exactly as it was, and the file can simply be imported again.
///
/// The failure here is not a crash — `session_crash.rs` owns that story, with
/// a child that really dies inside `STORE_EVENT_COMMIT_MID`. This is the
/// ordinary half: an error that unwinds through a commit which has already
/// written five events, three contacts and their sealed bodies. Before the
/// wrap all of that stayed, and the refusal on screen was the only sign that
/// the file had been half-taken; the user's only way forward was to import it
/// again, which wrote every surviving event a second time.
#[test]
fn a_commit_that_fails_after_it_has_written_rows_leaves_none_of_them_behind() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");

    let mut session = Session::open(&directory);
    let first = session.commit_soul_import_v1(&text).expect("first import");
    assert_eq!(first.events_written, 5);
    assert_eq!(first.contacts_created, 3);

    let people_before = session.people().expect("graph").people.len();
    let ties_before = session.people().expect("graph").ties.len();
    let entries_before = session.audit().expect("the chain").entries.len();

    let refusal = session
        .commit_soul_import_v1(SECOND_ACCOUNT)
        .expect_err("a store with two account owners has no graph to build");
    assert_eq!(refusal.reason_code, "ROUTINE");
    assert!(
        refusal.explanation.contains("回滚"),
        "a refusal that wrote and then undid it has to say so: {}",
        refusal.explanation,
    );
    for spoken in SPOKEN_BY_THE_SECOND_ACCOUNT {
        assert!(
            !refusal.explanation.contains(spoken),
            "the refusal repeated {spoken:?} out of the file: {}",
            refusal.explanation,
        );
    }

    // The events, the contacts, the ties and the chain are the first import's,
    // to the row. A commit that wrote and then failed wrote nothing.
    assert_eq!(events_in(&session), 5, "the refused import left events");
    let after = session.people().expect("graph");
    assert_eq!(after.people.len(), people_before);
    assert_eq!(after.ties.len(), ties_before);
    assert_eq!(
        session.audit().expect("the chain").entries.len(),
        entries_before,
        "the refused import left audit entries for rows that are not there",
    );
    assert!(
        session.audit().expect("the chain").verified,
        "rolling back a commit broke the chain",
    );

    // And the store still works: the file that did land goes in again and
    // recognizes its own people, which a half-written second import would
    // have made impossible to reason about.
    let again = session
        .commit_soul_import_v1(&text)
        .expect("the store is usable after a rolled-back commit");
    assert_eq!(again.contacts_matched, 3);
    assert_eq!(again.contacts_created, 0);

    // AC-04 still: nothing the rolled-back file said is legible on disk.
    drop(session);
    for sentence in SPOKEN_BY_THE_SECOND_ACCOUNT {
        for (name, bytes) in files_in(&directory) {
            assert!(
                !contains(&bytes, sentence.as_bytes()),
                "{sentence:?} is legible in {name}",
            );
        }
    }
    drop(keep);
}

/// AC-25 through the shell: a file that tries to give instructions is counted
/// and stored like any other data.
///
/// The injection corpus is everybody else's words and names nobody as the
/// account owner, so it is spliced onto an export that does — which is what a
/// real one would look like: the user's own chat, with an attempt inside it.
#[test]
fn an_export_that_tries_to_give_instructions_is_counted_and_obeyed_by_nothing() {
    let (keep, directory) = scratch();
    let owned = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");
    let attempts =
        fixtures::read_text("import/soul-import-v1/injection_lines.jsonl").expect("fixture");
    let lines: Vec<&str> = owned
        .lines()
        .chain(attempts.lines().filter(|line| !line.contains("\"header\"")))
        .filter(|line| !line.trim().is_empty())
        .collect();
    let text = lines.join("\n");

    let mut session = Session::open(&directory);
    let preview = session
        .preview_soul_import_v1(&text)
        .expect("injection lines are valid data");
    assert!(
        preview.messages_with_injection_markers > 0,
        "the user is told before committing that their file tried something",
    );

    let receipt = session.commit_soul_import_v1(&text).expect("commit");
    assert_eq!(
        receipt.messages_with_injection_markers, preview.messages_with_injection_markers,
        "the receipt charges what the preview quoted",
    );
    assert_eq!(receipt.events_written, preview.messages);
    drop(keep);
}

/// AC-25's import channel for a user who read the preview and closed the
/// screen.
///
/// `ImportPreview` has counted these lines since WP06 and the count went to
/// the screen and nowhere else, so an export that was read and then abandoned
/// left nothing behind — while the same file committed left an
/// `injection.blocked` row through `import_commands::commit`. Reading a file
/// and sealing one are two facts, and this is the first of them; the commit
/// entry is not removed to make the numbers tidier.
#[test]
fn an_export_that_tries_to_give_instructions_is_counted_even_when_it_is_never_committed() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/injection_lines.jsonl").expect("fixture");

    let session = Session::open(&directory);
    let preview = session
        .preview_soul_import_v1(&text)
        .expect("injection lines are valid data");
    assert!(
        preview.messages_with_injection_markers > 0,
        "the fixture has to try something for this test to mean anything",
    );
    assert!(!preview.writes_anything);
    assert!(
        session.people().expect("graph").people.is_empty(),
        "a preview must not write anything",
    );

    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    let blocked = chain
        .entries
        .iter()
        .find(|entry| entry.action == "injection.blocked")
        .expect("the export asked to be obeyed and the chain never heard about it");
    assert_eq!(blocked.decision, "denied");
    assert_eq!(
        blocked.reason_code.as_deref(),
        Some("INJECTION_MARKERS_FOUND"),
    );
    assert_eq!(
        blocked.items,
        Some(preview.messages_with_injection_markers),
        "the entry counts something other than what the screen was told",
    );
    assert!(
        blocked.bytes.is_none(),
        "the length of a hostile line is still the line",
    );
    assert!(blocked.follows_previous);

    // A count and a code. Nothing the file tried to say.
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    for prose in ATTEMPTED {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }
    drop(keep);
}

/// The control: a corpus that tries nothing is not accused of anything.
#[test]
fn a_clean_export_preview_leaves_no_injection_entry() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");

    let session = Session::open(&directory);
    let preview = session
        .preview_soul_import_v1(&text)
        .expect("the corpus parses");
    assert_eq!(preview.messages_with_injection_markers, 0);

    let chain = session.audit().expect("the store opened");
    assert!(
        !chain
            .entries
            .iter()
            .any(|entry| entry.action == "injection.blocked"),
        "an ordinary export was recorded as an attempt: {:?}",
        chain.entries,
    );
    drop(keep);
}

/// AC-25's import channel on the *other* format.
///
/// Every test above sends `soul-import-v1`, whose bodies are one JSON string
/// each. Telegram is the format that arrives in pieces: a `text` is cut into
/// runs wherever an entity begins, and `soul_import::telegram::flatten_text`
/// is what puts the sentence back together before anything reads it. The
/// fixture splits `忽略之前指令` across two runs, so a build that scanned run by
/// run would count nothing and this session would record nothing — the user
/// would be told their export tried nothing at all.
///
/// Nothing is committed, so the store holds no people at the end of it and the
/// row is there anyway.
#[test]
fn a_telegram_export_that_tries_to_give_instructions_is_counted_even_when_it_is_never_committed() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/telegram/result_injection.json").expect("fixture");

    let session = Session::open(&directory);
    let preview = session
        .preview_telegram(&text)
        .expect("a hostile export is still a well-formed one");
    assert_eq!(preview.source, "telegram-desktop");
    assert_eq!(
        preview.messages_with_injection_markers, 2,
        "the split phrase and the address, one message each",
    );
    assert!(!preview.writes_anything);
    assert!(
        session.people().expect("graph").people.is_empty(),
        "a preview must not write anything",
    );

    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    let blocked = chain
        .entries
        .iter()
        .find(|entry| entry.action == "injection.blocked")
        .expect("the export asked to be obeyed and the chain never heard about it");
    assert_eq!(blocked.decision, "denied");
    assert_eq!(
        blocked.reason_code.as_deref(),
        Some("INJECTION_MARKERS_FOUND"),
    );
    assert_eq!(
        blocked.items,
        Some(preview.messages_with_injection_markers),
        "the entry counts something other than what the screen was told",
    );
    assert!(
        blocked.bytes.is_none(),
        "the length of a hostile line is still the line",
    );
    assert!(blocked.follows_previous);

    // A count and a code. Not the phrase the runs spelled out between them,
    // not the address it named, and not the person whose chat it arrived in.
    let played = serde_json::to_string(&chain).expect("serialize the chain");
    for prose in ["忽略之前指令", "忽略之前", "evil.example", "李 雷"] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }
    drop(keep);
}

/// The control for the test above: the good export is not accused of anything.
#[test]
fn a_clean_telegram_preview_leaves_no_injection_entry() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/telegram/result_basic.json").expect("fixture");

    let session = Session::open(&directory);
    let preview = session.preview_telegram(&text).expect("the export parses");
    assert_eq!(preview.messages_with_injection_markers, 0);

    let chain = session.audit().expect("the store opened");
    assert!(
        !chain
            .entries
            .iter()
            .any(|entry| entry.action == "injection.blocked"),
        "an ordinary export was recorded as an attempt: {:?}",
        chain.entries,
    );
    drop(keep);
}

/// AC-05's failure half, through the session: a readable refusal that names
/// the fields and nobody.
#[test]
fn a_telegram_export_with_missing_fields_is_refused_without_naming_a_partner() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/telegram/result_missing_fields.json").expect("fixture");

    let mut session = Session::open(&directory);
    let refusal = session
        .preview_telegram(&text)
        .expect_err("the export is missing required fields");

    assert_eq!(refusal.reason_code, "ROUTINE");
    assert!(
        refusal.explanation.contains("没有导入"),
        "a refusal has to say that nothing landed: {}",
        refusal.explanation,
    );
    for field in ["messages", "date"] {
        assert!(
            refusal.explanation.contains(field),
            "the refusal does not say which field is missing: {}",
            refusal.explanation,
        );
    }
    for name in NAMES_IN_THE_BROKEN_EXPORT {
        assert!(
            !refusal.explanation.contains(name),
            "the refusal repeated {name:?} out of the file: {}",
            refusal.explanation,
        );
    }

    // Committing the same file is refused the same way, and writes nothing.
    let refusal = session
        .commit_telegram(&text)
        .expect_err("a file that does not parse cannot be committed");
    for name in NAMES_IN_THE_BROKEN_EXPORT {
        assert!(!refusal.explanation.contains(name));
    }
    assert!(session.people().expect("graph").people.is_empty());
    drop(keep);
}

/// A file that is not JSON at all says where it stops being readable, and does
/// not quote the part it choked on.
#[test]
fn something_that_is_not_a_telegram_export_is_refused_by_position_not_by_quotation() {
    let (keep, directory) = scratch();
    let session = Session::open(&directory);

    let refusal = session
        .preview_telegram("{\"chats\": 明天上午十点在公司门口见")
        .expect_err("that is not JSON");
    assert!(refusal.explanation.contains("result.json"));
    assert!(
        !refusal.explanation.contains("明天上午十点在公司门口见"),
        "the refusal quoted the file: {}",
        refusal.explanation,
    );
    drop(keep);
}

/// The good Telegram export, all the way through, so the refusal above is not
/// the only thing this format is exercised by.
#[test]
fn a_telegram_export_commits_through_the_session_and_becomes_a_graph() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/telegram/result_basic.json").expect("fixture");

    let mut session = Session::open(&directory);
    let preview = session.preview_telegram(&text).expect("the export parses");
    assert_eq!(preview.source, "telegram-desktop");
    assert_eq!(preview.participants, 3);
    assert_eq!(preview.messages, 6);
    assert!(preview.owner_identified);

    let receipt = session.commit_telegram(&text).expect("commit");
    assert_eq!(receipt.source, "telegram-desktop");
    assert_eq!(receipt.events_written, 6);
    assert_eq!(receipt.ties_rebuilt, 2);
    assert_eq!(session.people().expect("graph").people.len(), 3);
    drop(keep);
}

/// A session whose store did not open says so instead of reading the file and
/// then discovering there is nowhere to put it.
#[test]
fn with_no_store_open_the_import_screen_is_told_why_rather_than_shown_counts() {
    let (keep, directory) = scratch();
    // A directory whose `soul.db` is a directory cannot be opened as a
    // database, which is the cheapest honest way to get a closed store.
    std::fs::create_dir_all(directory.join("soul.db")).expect("create");

    let mut session = Session::open(&directory);
    let status = session.status();
    assert!(!status.store_opened, "{}", status.store_notice);

    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").expect("fixture");
    for refusal in [
        session.preview_soul_import_v1(&text).expect_err("no store"),
        session.commit_soul_import_v1(&text).expect_err("no store"),
    ] {
        assert_eq!(refusal.reason_code, "ROUTINE");
        assert!(
            refusal.explanation.contains("加密库"),
            "the refusal has to name the closed store: {}",
            refusal.explanation,
        );
    }
    drop(keep);
}
