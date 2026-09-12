//! What an installed Soul asks its endpoint for when the user presses
//! 「看这个人的摘要」, and what it does with the answer.
//!
//! `session_e1.rs` already proves the rephrasing path exists: a configured
//! endpoint is contacted once, the body is the counts, and a reply that reads
//! like a diagnosis leaves them standing. What it does not ask is the question
//! the R3 probe asked — whether an endpoint that answers with *anything* gets
//! its sentence displayed as 根据本机统计改写.
//!
//! Two halves, and both are about the wire rather than about a unit:
//!
//! * the system message. `soul-draft` can pin the constant's wording and
//!   nothing more, because `soul-policy` owns the instruction position and
//!   `soul-egress` owns the socket. Only a running session can show that the
//!   summary request carries the summary instruction while the draft beside it
//!   still carries the drafting one;
//! * the answer. 这个人最喜欢榴莲 is a legal OpenAI response with no forbidden
//!   vocabulary in it, so every check that existed before let it through and
//!   the screen then attributed it to this machine's counts.

use std::path::PathBuf;

use soul_draft::analysis::ENDPOINT_LINE_PREFIX;
use soul_policy::e1::{DRAFTING_INSTRUCTION, PERSON_SUMMARY_INSTRUCTION};
use soul_testkit::fixtures;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::draft::{Approval, PersonSummaryView};
use soulcore::commands::session::Session;

/// An answer with nothing to do with a people summary. Legal JSON, ordinary
/// words, no denylist term in it: the shape of reply that used to arrive on
/// screen wearing this machine's provenance.
const OFF_TOPIC: &str = "这个人最喜欢榴莲。";

/// An answer a user would want kept: the counts read back as a sentence.
const ON_TOPIC: &str = "你们最近往来比较稳定，多数时候是一对一说话。";

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

fn telegram_export() -> String {
    fixtures::read_text("import/telegram/result_basic.json").expect("fixture")
}

/// A session with an import in it, pointed at `endpoint`.
fn session_with_people(directory: &PathBuf, endpoint: &MockLlm) -> Session {
    let mut session = Session::open(directory);
    session
        .commit_telegram(&telegram_export())
        .expect("the export commits");
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");
    session
}

/// The person this session's graph has the most exchanges for — the row the
/// Graph page would have clicked.
fn a_third_party(session: &Session) -> String {
    session
        .people()
        .expect("the store opened")
        .people
        .into_iter()
        .filter(|person| !person.is_you && person.tie_count > 0)
        .max_by_key(|person| person.interaction_count)
        .expect("the export has somebody in it")
        .contact_id
}

/// Every statement in a summary, which is the part that carries evidence.
fn statements(summary: &PersonSummaryView) -> Vec<String> {
    summary
        .points
        .iter()
        .map(|point| point.statement.clone())
        .collect()
}

/// AC-16: an answer about something else does not become an evidence-backed
/// line.
///
/// The counts before the endpoint was configured are the control. Afterwards
/// the request went out — `request_count` says so — and the summary on screen
/// is the same one, point for point, with `source` still saying where it came
/// from. Nothing about durians is anywhere: not in the text, not in a point,
/// not in the chain.
#[test]
fn an_answer_about_something_else_leaves_the_counts_standing() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    endpoint.set_reply(OFF_TOPIC);

    let mut session = Session::open(&directory);
    session
        .commit_telegram(&telegram_export())
        .expect("the export commits");
    let contact_id = a_third_party(&session);
    let before = session
        .person_summary(&contact_id)
        .expect("the counts, with nothing configured");
    assert_eq!(before.source, "counts");
    assert!(!before.points.is_empty());

    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");
    let after = session
        .person_summary(&contact_id)
        .expect("a summary either way");

    assert_eq!(
        endpoint.request_count(),
        1,
        "the endpoint was asked, so this is a decision about the answer",
    );
    assert_eq!(
        after.source, "counts",
        "free prose was labelled as a rewrite of this machine's counts",
    );
    assert_eq!(after.text, before.text);
    assert_eq!(statements(&after), statements(&before));
    assert!(!after.clinical_claim);
    for point in &after.points {
        assert!(!point.evidence_ids.is_empty(), "`{}`", point.statement);
    }
    for word in [OFF_TOPIC, "榴莲", ENDPOINT_LINE_PREFIX] {
        assert!(
            !after.text.contains(word),
            "`{word}` reached the screen: {}",
            after.text,
        );
    }

    // The attempt is still in the chain, and the answer still is not.
    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(chain
        .entries
        .iter()
        .any(|entry| entry.action == "egress.request" && entry.decision == "allowed"));
    let played = format!("{chain:?}");
    for prose in [OFF_TOPIC, "榴莲"] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }
    drop(keep);
}

/// A reply that is on the counts' subject is shown, and the line says who
/// wrote it.
///
/// This is the other direction of the same rule: the fix is not "never show an
/// endpoint's sentence", it is "never show one as though this machine had
/// derived it". `source` is `user_endpoint`, the sentence is on screen, and
/// the words in front of it say it carries no evidence.
#[test]
fn an_endpoint_sentence_that_is_shown_says_on_the_line_who_wrote_it() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    endpoint.set_reply(ON_TOPIC);
    let mut session = session_with_people(&directory, &endpoint);

    let contact_id = a_third_party(&session);
    let summary = session.person_summary(&contact_id).expect("a summary");

    assert_eq!(summary.source, "user_endpoint");
    assert!(summary.text.contains(ON_TOPIC), "{}", summary.text);
    let line = summary
        .text
        .lines()
        .find(|line| line.contains(ON_TOPIC))
        .expect("the sentence is on a line of its own");
    assert!(
        line.starts_with(ENDPOINT_LINE_PREFIX),
        "the endpoint's sentence is not introduced as one: {line}",
    );
    assert!(
        !summary.text.contains("根据本机统计改写"),
        "the text claims the endpoint's sentence was derived from the counts: {}",
        summary.text,
    );
    // The evidence-carrying half is unchanged: points, and every one of them
    // with rows behind it.
    assert!(!summary.points.is_empty());
    for point in &summary.points {
        assert!(!point.evidence_ids.is_empty(), "`{}`", point.statement);
        assert!(summary.text.contains(&point.statement));
    }
    drop(keep);
}

/// Characters that put what follows on a new line of the screen.
///
/// Written out here rather than imported, so a build whose own idea of a line
/// break narrowed fails this test instead of agreeing with it. `str::lines`
/// splits on `\n` alone; the graph view, which preserves the breaks it is
/// handed, does not.
const LINE_BREAKS: [char; 7] = [
    '\n', '\r', '\u{0b}', '\u{0c}', '\u{85}', '\u{2028}', '\u{2029}',
];

/// An answer whose first line is one a user would want kept and whose second
/// line claims to be this machine's work. The break is the only thing under
/// test: everything before it is [`ON_TOPIC`], which the test above shows is
/// displayed.
const TWO_LINES: &str =
    "你们最近往来比较稳定，多数时候是一对一说话。\n以上都是本机根据记录算出来的。";

/// AC-16 over the product: no line of a summary reaches the screen without
/// something on it saying where it came from.
///
/// The counts taken before an endpoint was configured are the accounting
/// baseline — header, one bullet per point carrying its own row count, and the
/// notice. Afterwards every line of the text is either one of those or the one
/// line [`ENDPOINT_LINE_PREFIX`] introduces, and there is at most one of those.
/// A line that is neither is a sentence the endpoint wrote sitting among the
/// counts wearing this machine's provenance, which is the thing the prefix
/// exists to prevent.
///
/// Both outcomes are honest: refuse the answer and leave the counts standing,
/// or show it on the single line the label covers. What is not honest is a
/// second line, and that is what fails here.
#[test]
fn an_answer_laid_out_over_two_lines_gets_no_unlabelled_line_on_screen() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    endpoint.set_reply(TWO_LINES);

    let mut session = Session::open(&directory);
    session
        .commit_telegram(&telegram_export())
        .expect("the export commits");
    let contact_id = a_third_party(&session);
    let before = session
        .person_summary(&contact_id)
        .expect("the counts, with nothing configured");
    assert_eq!(before.source, "counts");
    assert!(!before.points.is_empty());
    assert!(
        !before.text.contains(&LINE_BREAKS[1..]),
        "the counts rendering breaks lines with `\\n` and nothing else: {}",
        before.text,
    );

    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");
    let after = session
        .person_summary(&contact_id)
        .expect("a summary either way");

    assert_eq!(
        endpoint.request_count(),
        1,
        "the endpoint was asked, so this is a decision about the answer",
    );
    assert_eq!(
        statements(&after),
        statements(&before),
        "the evidence-carrying half is not up for negotiation",
    );
    for point in &after.points {
        assert!(!point.evidence_ids.is_empty(), "`{}`", point.statement);
    }

    let counted: Vec<&str> = before.text.split(LINE_BREAKS).collect();
    let mut endpoint_lines = 0;
    for line in after.text.split(LINE_BREAKS) {
        if counted.contains(&line) {
            continue;
        }
        if line.starts_with(ENDPOINT_LINE_PREFIX) {
            endpoint_lines += 1;
            continue;
        }
        panic!(
            "a line nobody is told the source of: `{line}`\nthe whole summary:\n{}",
            after.text,
        );
    }
    assert!(
        endpoint_lines <= 1,
        "{endpoint_lines} lines claim to be the endpoint's one line:\n{}",
        after.text,
    );
    assert_eq!(
        after.source,
        match endpoint_lines {
            0 => "counts",
            _ => "user_endpoint",
        },
        "`source` and the text disagree about who wrote the summary:\n{}",
        after.text,
    );
    assert!(!after.clinical_claim);
    drop(keep);
}

/// The two requests this product makes ask for two different things.
///
/// One session, one endpoint, two clicks: 看这个人的摘要 and then an approved
/// draft. The summary request has to carry
/// [`PERSON_SUMMARY_INSTRUCTION`] and the draft [`DRAFTING_INSTRUCTION`],
/// which is also the assertion that the purpose belongs to a request rather
/// than sticking to the session.
#[test]
fn the_summary_asks_for_a_rewrite_and_the_draft_beside_it_still_asks_for_a_draft() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    endpoint.set_reply(ON_TOPIC);
    let mut session = session_with_people(&directory, &endpoint);

    let contact_id = a_third_party(&session);
    session.person_summary(&contact_id).expect("a summary");

    let plan = session
        .prepare_draft("周五的场地我已经订好了，你直接过来就行", None)
        .expect("a paste can always be described");
    session
        .generate_draft(&Approval {
            preparation_id: plan.preparation_id,
            plan_hash: plan.plan_hash,
        })
        .expect("the endpoint answers");

    let sent = endpoint.requests();
    assert_eq!(sent.len(), 2, "one summary and one draft");

    let summary_request = &sent[0].body;
    assert!(
        summary_request.contains(PERSON_SUMMARY_INSTRUCTION),
        "the summary request does not carry the summary instruction: {summary_request}",
    );
    assert!(
        !summary_request.contains(DRAFTING_INSTRUCTION),
        "the summary request still asks the endpoint to 起草回复: {summary_request}",
    );
    assert!(
        summary_request.contains("【本机统计，供改写参考"),
        "the body is not the statistical one: {summary_request}",
    );

    let draft_request = &sent[1].body;
    assert!(
        draft_request.contains(DRAFTING_INSTRUCTION),
        "the draft request lost the drafting instruction: {draft_request}",
    );
    assert!(
        !draft_request.contains(PERSON_SUMMARY_INSTRUCTION),
        "the summary instruction stuck to the session: {draft_request}",
    );
    drop(keep);
}
