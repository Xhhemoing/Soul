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

use soul_testkit::fixtures;
use soulcore::commands::session::Session;

/// Sentences out of `fixtures/import/soul-import-v1/three_partners.jsonl`.
/// They are what the corpus says; none of them may end up legible on disk.
const SPOKEN: &[&str] = &[
    "明天上午十点在公司门口见",
    "这周先把方案定下来",
    "周五要交的那份材料我写完了",
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
    needle.len() <= haystack.len() && haystack.windows(needle.len()).any(|window| window == needle)
}

/// The whole path a Windows user takes: pick a file, read what is in it, say
/// yes, and find the people on `/graph` afterwards.
#[test]
fn a_soul_import_v1_file_committed_through_the_session_lands_sealed_and_shows_up_as_people() {
    let (keep, directory) = scratch();
    let text = fixtures::read_text("import/soul-import-v1/three_partners.jsonl").expect("fixture");

    let mut session = Session::open(&directory);
    assert!(
        session.people().expect("an empty graph is still one").people.is_empty(),
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
    assert!(receipt.evidence_written >= 16);
    assert_eq!(receipt.ties_rebuilt, 4, "AC-08: one tie per partner");

    // The counts are not a story the receipt tells about itself: the graph the
    // shell draws next has to have those people in it.
    let people = session.people().expect("the graph reads back");
    assert_eq!(people.people.len(), 5);
    assert_eq!(people.ties.len(), 4);
    assert!(people.ties.iter().all(|tie| !tie.evidence.is_empty()));
    assert!(people.third_party_data_is_local_only);
    assert!(
        people.people.iter().all(|person| !person.identifier_hint.is_empty()),
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
    let text: String = owned
        .lines()
        .chain(attempts.lines().filter(|line| !line.contains("\"header\"")))
        .filter(|line| !line.trim().is_empty())
        .map(|line| format!("{line}\n"))
        .collect();

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
