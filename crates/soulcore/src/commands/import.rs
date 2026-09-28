//! WP06's command surface: read one of the two v0.1 export formats, put it in
//! the store, or fall back to the questionnaire when there is no file.
//!
//! Reading and committing are separate calls because they are separate
//! decisions. [`read_soul_import_v1`] and [`read_telegram`] touch nothing; a
//! caller can show the user what a file contains, and what is wrong with it,
//! before anything is written. [`commit`] is the step that writes.
//!
//! Nothing here interprets an imported message. Bodies arrive as
//! `UntrustedText`, are sealed, and are pointed at; the injection scan that
//! runs during a commit feeds an audit entry and decides nothing. This module
//! never builds a tool plan, and no code path in it reads what was imported in
//! order to choose what to do next.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use soul_import::commit::{ImportError, ImportReceipt};
use soul_import::defect::ImportFailure;
use soul_import::model::StagedImport;
use soul_policy::audit::append_or_store_error;
use soul_policy::injection;
use soul_store::SqlCipherStore;

/// What the import screen says about where the file goes.
///
/// A promise about this build rather than a description of the screen: the
/// file is read in the WebView the user picked it in, and what the core does
/// with it is seal it and point at it. Nothing on this path uploads anything
/// and nothing on it reads a message in order to decide what to do next.
pub const IMPORT_LOCAL_ONLY_NOTICE: &str =
    "导入全程在本机：文件由你自己挑，内容读进来就地加密入库，\
    不上传，也不会被当成指令执行。导入的正文一律当数据看待。";

/// What a file that did not parse is told, in front of the parser's own
/// account of what is wrong with it.
///
/// The account below it names lines, JSON paths and contract field names, and
/// `soul-import` is what keeps it from quoting the file — see that crate's
/// `redact` module and the tests that search a refusal for every sentence in
/// the corpus.
pub const IMPORT_REFUSED_NOTICE: &str = "这个文件没有导入，一行都没有写进库。\
    下面写的是它哪里对不上，不会复述文件里的内容：";

// ------------------------------------------------------ input budgets ---
//
// Wave 1 (2026-09-27): the import channel is the one place the product reads
// a file somebody else wrote, and until here it read however much arrived.
// The WebView holds the whole text, the IPC copies it, and the parsers walk
// it line by line — so a file only bounded by the disk it came from was a
// memory bound nobody had written down. The three budgets below are the
// written-down version. They are checked in `Session` (the core says no on
// its own) and the byte budget again in the import screen, against the
// file's size, before the WebView reads it at all.

/// The most import text one preview or commit will read, in bytes.
///
/// 64 MiB is far past both shipped exporters: the Q2 scale fixtures put ten
/// thousand messages well under ten megabytes, and a `result.json` this size
/// holds years of heavy use. Past it, the copies this path makes — the
/// WebView's string, the IPC's, the parser's staged form — stop being a cost
/// and start being the thing that falls over, and a refusal with a sentence
/// beats an allocation failure with none.
pub const MAX_IMPORT_BYTES: usize = 64 * 1024 * 1024;

/// The most messages one commit will seal.
///
/// An import and its graph rebuild are one transaction on one click. The
/// Q2-03 measurements put ten thousand messages at a few seconds of it;
/// ten times that is the ceiling at which the click still ends, rather than
/// holding the store's lock for however long a hostile line count decides.
pub const MAX_IMPORT_MESSAGES: usize = 100_000;

/// The longest single message body a staged import may carry, in characters.
///
/// No chat this product imports produces one message this long — Telegram
/// caps a message at a few thousand characters — so a "message" of sixty-five
/// thousand is an export that is broken or built to be expensive: each body
/// becomes one sealed blob and one injection scan. The budget is per message
/// so the sentence can say what is wrong without quoting anything.
pub const MAX_MESSAGE_CHARS: usize = 65_536;

/// What a file over [`MAX_IMPORT_BYTES`] is told. Nothing was parsed.
pub const IMPORT_OVER_BYTE_BUDGET_NOTICE: &str = "这个文件太大，没有读：\
    这一版一次最多读 64 MB 的导出文本。把导出按时间段拆成几份小的，一份一份来。";

/// What a file over [`MAX_IMPORT_MESSAGES`] is told. Nothing was written.
pub const IMPORT_OVER_MESSAGE_BUDGET_NOTICE: &str = "这个文件没有导入：\
    里面的消息条数超过了这一版一次能写的上限（100000 条）。\
    把导出按时间段拆成几份小的，一份一份来。";

/// What a file with a body over [`MAX_MESSAGE_CHARS`] is told.
pub const IMPORT_MESSAGE_TOO_LONG_NOTICE: &str = "这个文件没有导入：\
    里面有消息比这一版单条能存的上限（65536 个字符）还长。\
    聊天软件导不出这么长的单条消息，这更像是文件本身出了问题。";

/// Refuse text over the byte budget, before any parser reads it.
///
/// Bytes rather than characters, because bytes are what the IPC carried and
/// what the parsers will walk; the shell checks the same constant against
/// `File.size` before reading, and this check is the one that holds when
/// something other than the import screen is doing the sending.
pub fn within_byte_budget(text: &str) -> Result<(), String> {
    match text.len() > MAX_IMPORT_BYTES {
        true => Err(IMPORT_OVER_BYTE_BUDGET_NOTICE.to_owned()),
        false => Ok(()),
    }
}

/// Refuse a parsed import that is over the record budget or carries a body
/// no chat produced. Counts only: the sentence never quotes the file.
pub fn within_staged_budget(staged: &StagedImport) -> Result<(), String> {
    if staged.messages.len() > MAX_IMPORT_MESSAGES {
        return Err(IMPORT_OVER_MESSAGE_BUDGET_NOTICE.to_owned());
    }
    let over_long = staged
        .messages
        .iter()
        .filter(|message| message.body.char_count() > MAX_MESSAGE_CHARS)
        .count();
    match over_long > 0 {
        true => Err(format!(
            "{IMPORT_MESSAGE_TOO_LONG_NOTICE}（{over_long} 条超长）"
        )),
        false => Ok(()),
    }
}

/// Parse a `soul-import-v1` JSONL file. Writes nothing.
pub fn read_soul_import_v1(text: &str) -> Result<StagedImport, ImportFailure> {
    soul_import::soul_import_v1::parse(text)
}

/// Parse a Telegram Desktop `result.json` document. Writes nothing.
///
/// The caller supplies the parsed JSON. This crate does not open files, and
/// v0.1 does not sniff archives: the user points at the `result.json` that
/// Telegram's own *Settings → Advanced → Export Telegram data* produced.
pub fn read_telegram(document: &Value) -> Result<StagedImport, ImportFailure> {
    soul_import::telegram::parse(document)
}

/// Write a parsed import into the encrypted store.
///
/// Every audit entry the commit produced is appended here, including the
/// `injection.blocked` one when the export carried an attempt — the user is
/// told that their file tried something and that it went nowhere.
pub fn commit(
    store: &mut SqlCipherStore,
    staged: &StagedImport,
    at_unix_seconds: i64,
) -> Result<ImportReceipt, ImportError> {
    let receipt = soul_import::commit::commit(store, staged)?;
    for content in receipt.audit.clone() {
        append_or_store_error(store, content, at_unix_seconds)?;
    }
    Ok(receipt)
}

/// Is the questionnaire the path to take?
pub fn questionnaire_needed(staged: Option<&StagedImport>) -> bool {
    soul_import::questionnaire::fallback_needed(staged)
}

/// The questions this build asks, for a UI that has to draw them.
///
/// The same list [`crate::commands::profile::questions`] returns, paired there
/// with what each one moves. There is one questionnaire in v0.1 and it is
/// asked once: whichever path the user came down, the answers go in through
/// [`crate::commands::profile::intake`], which records them and builds the
/// profile in one call.
pub fn questions() -> &'static [soul_import::questionnaire::Question] {
    soul_import::questionnaire::QUESTIONS
}

/// What one parsed file contains, as counts, before anything is written.
///
/// Every field is a number, a boolean or the name of a format. There is no
/// field here that could hold a message, a display name or an account: the
/// user is being asked to approve *how much* of their export is about to be
/// sealed, and re-reading what is in it is not part of that decision. The
/// bodies stay `UntrustedText` inside the staged import and never come out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPreview {
    /// `soul-import-v1` or `telegram-desktop`.
    pub source: String,
    /// People the file names, the account owner included.
    pub participants: usize,
    pub conversations: usize,
    pub messages: usize,
    /// Lines whose body carries injection markers. Counted here so the user
    /// knows before committing; obeyed by nothing, here or downstream.
    pub messages_with_injection_markers: u64,
    /// Whether the file says which participant is the user. A commit without
    /// one is refused: there would be no centre to the graph.
    pub owner_identified: bool,
    /// Always false: reading a file seals no contact, no event and no piece of
    /// evidence, and the counts above are the whole of what it produced.
    ///
    /// Not a claim that the database was left untouched. A preview whose file
    /// carried injection markers has already appended one `injection.blocked`
    /// row through `Session::note_injection` by the time this value is
    /// returned, and abandoning the preview leaves that row standing — the
    /// import channel was exercised and AC-25 wants that on the chain. A
    /// screen that reads this field as "nothing has been written" would be
    /// telling a hostile-export user something the audit table disagrees with.
    pub writes_anything: bool,
    pub notice: String,
}

impl ImportPreview {
    pub fn of(staged: &StagedImport) -> ImportPreview {
        ImportPreview {
            source: staged.source.as_str().to_owned(),
            participants: staged.participants.len(),
            conversations: staged.conversation_count(),
            messages: staged.messages.len(),
            messages_with_injection_markers: staged
                .messages
                .iter()
                .filter(|message| injection::looks_like_injection(&message.body))
                .count() as u64,
            owner_identified: staged.owner().is_some(),
            writes_anything: false,
            notice: IMPORT_LOCAL_ONLY_NOTICE.to_owned(),
        }
    }
}

/// What one commit wrote, as counts.
///
/// The same rule as the preview: contacts and events are counted, never
/// listed, because a list of contact identifiers on this screen would be the
/// export's account handles wearing a different hat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReceiptView {
    pub source: String,
    pub contacts_created: usize,
    /// People this file named who were already in the store. Matched by
    /// identifier digest, so a second export of the same chat does not clone
    /// everybody.
    pub contacts_matched: usize,
    pub events_written: usize,
    pub evidence_written: usize,
    pub messages_with_injection_markers: u64,
    /// Ties the rebuild that followed the commit left behind. The graph is
    /// derived from the evidence this import wrote, not imported alongside it.
    pub ties_rebuilt: usize,
    pub notice: String,
}

impl ImportReceiptView {
    pub fn of(receipt: &ImportReceipt, ties_rebuilt: usize) -> ImportReceiptView {
        ImportReceiptView {
            source: receipt
                .source
                .map(|source| source.as_str().to_owned())
                .unwrap_or_default(),
            contacts_created: receipt.contacts_created.len(),
            contacts_matched: receipt.contacts_matched.len(),
            events_written: receipt.events_written.len(),
            evidence_written: receipt.evidence_written.len(),
            messages_with_injection_markers: receipt.messages_with_injection_markers,
            ties_rebuilt,
            notice: IMPORT_LOCAL_ONLY_NOTICE.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    //! The budget arithmetic, on staged imports built by hand.
    //!
    //! The session tests exercise the same checks through `Session` with real
    //! files; these pin the boundaries themselves — at the budget is allowed,
    //! one past it is refused — without allocating a hundred-thousand-line
    //! export inside a parser.

    use soul_import::model::{ImportSource, ParticipantHandle, StagedImport, StagedMessage};
    use soul_policy::injection::UntrustedText;
    use soul_schema::common::Timestamp;
    use soul_schema::contact::IdentifierKind;
    use soul_schema::soul_import_v1::SenderScope;

    use super::{
        within_byte_budget, within_staged_budget, IMPORT_MESSAGE_TOO_LONG_NOTICE,
        IMPORT_OVER_BYTE_BUDGET_NOTICE, IMPORT_OVER_MESSAGE_BUDGET_NOTICE, MAX_IMPORT_BYTES,
        MAX_IMPORT_MESSAGES, MAX_MESSAGE_CHARS,
    };

    fn message_of(chars: usize) -> StagedMessage {
        StagedMessage {
            external_id: "m-1".to_owned(),
            occurred_at: Timestamp::new("2026-01-01T00:00:00Z"),
            sender: ParticipantHandle {
                kind: IdentifierKind::Handle,
                value: "someone".to_owned(),
            },
            conversation_id: "c-1".to_owned(),
            group: false,
            scope: SenderScope::ThirdParty,
            body: UntrustedText::new("a".repeat(chars)),
        }
    }

    fn staged_with(messages: Vec<StagedMessage>) -> StagedImport {
        StagedImport {
            source: ImportSource::SoulImportV1,
            exported_at: None,
            participants: Vec::new(),
            messages,
        }
    }

    #[test]
    fn the_byte_budget_allows_the_boundary_and_refuses_one_past_it() {
        let at_the_limit = "a".repeat(MAX_IMPORT_BYTES);
        assert_eq!(within_byte_budget(&at_the_limit), Ok(()));
        let mut over = at_the_limit;
        over.push('a');
        assert_eq!(
            within_byte_budget(&over),
            Err(IMPORT_OVER_BYTE_BUDGET_NOTICE.to_owned()),
        );
    }

    #[test]
    fn the_message_budget_allows_the_boundary_and_refuses_one_past_it() {
        let at_the_limit = staged_with(vec![message_of(1); MAX_IMPORT_MESSAGES]);
        assert_eq!(within_staged_budget(&at_the_limit), Ok(()));
        let mut messages = at_the_limit.messages;
        messages.push(message_of(1));
        assert_eq!(
            within_staged_budget(&staged_with(messages)),
            Err(IMPORT_OVER_MESSAGE_BUDGET_NOTICE.to_owned()),
        );
    }

    #[test]
    fn a_body_at_the_character_limit_passes_and_one_past_it_is_counted_not_quoted() {
        let at_the_limit = staged_with(vec![message_of(MAX_MESSAGE_CHARS)]);
        assert_eq!(within_staged_budget(&at_the_limit), Ok(()));

        let over = staged_with(vec![
            message_of(MAX_MESSAGE_CHARS + 1),
            message_of(2),
            message_of(MAX_MESSAGE_CHARS + 1),
        ]);
        let refusal = within_staged_budget(&over).expect_err("two bodies are over");
        assert!(
            refusal.contains(IMPORT_MESSAGE_TOO_LONG_NOTICE),
            "{refusal}"
        );
        assert!(refusal.contains("2 条超长"), "{refusal}");
        assert!(
            !refusal.contains("aaa"),
            "a refusal must not quote the file"
        );
    }
}
