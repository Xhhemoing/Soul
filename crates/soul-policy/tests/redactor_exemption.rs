//! AC-13: one confirmed exemption carries the original once, and the next
//! draft is placeheld again.
//!
//! PRODUCT_LOCK gives four conditions and each has a test here: the exemption
//! needs a second confirmation, it covers one message rather than the request,
//! it is not remembered, and the research track cannot use it at all.
//!
//! "Not remembered" is the one that is easy to get wrong in a way tests miss,
//! so it is checked twice: behaviourally, by drafting again with the same
//! redactor and the same turns; and structurally, by asserting the redactor
//! holds no state that could remember. The type system does most of the work —
//! `OneShotExemption` is consumed by value and is not `Clone` — but a compile
//! error is not something a reader of the acceptance matrix can see, so the
//! behaviour is asserted too.

use uuid::Uuid;

use soul_policy::redactor::{
    ExemptionRequest, KnownIdentifiers, Redactor, Turn, ACCOUNT_PLACEHOLDER, NAME_PLACEHOLDER,
    THIRD_PARTY_PLACEHOLDER,
};
use soul_schema::common::SealedSubject;
use soul_testkit::leakage::LeakageChecker;

const ORIGINAL: &str = "明天上午十点在公司门口见，别迟到";
const OTHER_ORIGINAL: &str = "另外那份合同我周三才能签完，先别催";
const NAME: &str = "李雷";
const PHONE: &str = "13800138000";

/// The same person, spelled the way an export spells a display name.
///
/// `fixtures/import/telegram/result_basic.json` writes the space, and so does
/// every screen that shows a contact. It is the spelling the identifier set
/// would hold if anything had been imported — and the one that has to be
/// placeheld when nothing has.
const SPACED_LABEL: &str = "李 雷";

fn checker() -> LeakageChecker {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("original", ORIGINAL);
    checker.add_third_party_body("other", OTHER_ORIGINAL);
    checker.add_known_identifier("name", NAME);
    checker.add_known_identifier("phone", PHONE);
    checker
}

fn redactor() -> Redactor {
    Redactor::new(KnownIdentifiers::new().with_name(NAME).with_account(PHONE))
}

fn conversation() -> (Uuid, Uuid, Vec<Turn>) {
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    (
        first,
        second,
        vec![
            Turn::new(first, SealedSubject::ThirdParty, ORIGINAL),
            Turn::new(second, SealedSubject::ThirdParty, OTHER_ORIGINAL),
            Turn::new(Uuid::now_v7(), SealedSubject::Owner, "我看看时间再回复。"),
        ],
    )
}

#[test]
fn a_confirmed_exemption_carries_exactly_one_original_and_the_next_draft_does_not() {
    let redactor = redactor();
    let (exempted_turn, _other, turns) = conversation();

    // Default: nothing original goes out.
    let before = redactor.redact_for_e1(&turns);
    checker().assert_clean("the draft before the exemption", before.as_str());
    assert_eq!(before.placeheld_turns(), 2);

    // Two steps. The request alone is not permission.
    let request = ExemptionRequest::for_turn(exempted_turn);
    let exemption = request.confirm(true).expect("the user confirmed twice");
    let during = redactor.redact_for_e1_with_exemption(&turns, exemption);

    assert!(
        during.as_str().contains(ORIGINAL),
        "the confirmed turn must go out verbatim: {}",
        during.as_str(),
    );
    assert!(during.carries_exempted_original());
    assert_eq!(during.exempted_turn(), Some(exempted_turn));

    // One message, not the request: the other third-party turn stays hidden.
    assert!(
        !during.as_str().contains(OTHER_ORIGINAL),
        "an exemption covers one turn, not the conversation: {}",
        during.as_str(),
    );
    assert_eq!(during.third_party_turns(), 2);
    assert_eq!(during.placeheld_turns(), 1);

    // And it is not remembered. Same redactor, same turns, no exemption.
    let after = redactor.redact_for_e1(&turns);
    checker().assert_clean("the draft after the exemption", after.as_str());
    assert_eq!(
        after.as_str(),
        before.as_str(),
        "the draft after an exemption must be identical to the one before it",
    );
    assert!(!after.carries_exempted_original());
}

#[test]
fn declining_the_second_confirmation_yields_no_exemption() {
    let (turn_id, _other, turns) = conversation();
    let request = ExemptionRequest::for_turn(turn_id);
    assert!(
        request.confirm(false).is_none(),
        "one click is not two confirmations",
    );

    let redacted = redactor().redact_for_e1(&turns);
    assert!(!redacted.as_str().contains(ORIGINAL));
    assert_eq!(
        redacted.as_str().matches(THIRD_PARTY_PLACEHOLDER).count(),
        2
    );
}

/// The exemption is for the message, not for the people in it. A user who
/// agrees to send one reply has not agreed to publish a phone number.
#[test]
fn identifiers_stay_placeheld_inside_an_exempted_turn() {
    let redactor = redactor();
    let turn_id = Uuid::now_v7();
    let turns = vec![Turn::new(
        turn_id,
        SealedSubject::ThirdParty,
        format!("{NAME}说：{ORIGINAL}，电话 {PHONE}"),
    )];

    let exemption = ExemptionRequest::for_turn(turn_id)
        .confirm(true)
        .expect("confirmed");
    let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

    assert!(
        redacted.as_str().contains(ORIGINAL),
        "{}",
        redacted.as_str()
    );
    assert!(!redacted.as_str().contains(NAME), "{}", redacted.as_str());
    assert!(!redacted.as_str().contains(PHONE), "{}", redacted.as_str());
    assert!(redacted.as_str().contains(NAME_PLACEHOLDER));
    assert!(redacted.as_str().contains(ACCOUNT_PLACEHOLDER));
}

/// The same promise for a name nobody registered, which is every name on a
/// Soul that has imported nothing.
///
/// [`identifiers_stay_placeheld_inside_an_exempted_turn`] hands the redactor
/// the name first, so it proves the set is consulted rather than that the
/// promise holds. PRODUCT_LOCK does not make the promise conditional on an
/// import having happened, and the two confirmation screens do not either —
/// 「姓名与账号两种情况下都占位」 is what `E1_PLAN_NOTICE` says to a user who
/// has never opened 导入. So the redactor here knows nobody at all, and the
/// exempted turn still may not carry the label out.
///
/// 正文 exemption is not 姓名 exemption. The confirmation buys the message.
#[test]
fn an_exempted_turn_placeholds_a_display_label_nobody_registered() {
    let redactor = Redactor::new(KnownIdentifiers::new());
    assert!(
        redactor.identifiers().is_empty(),
        "the point of this test is a redactor that knows nobody",
    );

    let turn_id = Uuid::now_v7();
    let turns = vec![Turn::new(
        turn_id,
        SealedSubject::ThirdParty,
        format!("{SPACED_LABEL} 说：{ORIGINAL}"),
    )];
    let exemption = ExemptionRequest::for_turn(turn_id)
        .confirm(true)
        .expect("the user confirmed twice");
    let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

    assert!(
        !redacted.as_str().contains(SPACED_LABEL),
        "a name the graph never learned reached the body the user confirmed: {}",
        redacted.as_str(),
    );
    assert!(
        redacted.as_str().contains(NAME_PLACEHOLDER),
        "the name was dropped rather than placeheld: {}",
        redacted.as_str(),
    );
    // And the confirmation still bought what it was for.
    assert!(
        redacted.as_str().contains(ORIGINAL),
        "the message the user confirmed did not travel: {}",
        redacted.as_str(),
    );
    assert!(redacted.carries_exempted_original());
}

/// The placeholder is one name, not a licence to redact the sentence.
///
/// The rule is a shape, and a shape that fired on ordinary prose would take
/// the exemption back by another route: the user confirmed twice to send this
/// message, and a body full of placeholders is not the message. Each string
/// below is something a paste plausibly contains and nothing below is a
/// spaced display label.
#[test]
fn the_label_shape_leaves_the_prose_the_exemption_was_for_alone() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for intact in [
        ORIGINAL,
        "周五的场地我已经订好了，你直接过来就行",
        // The account owner's own name. PRODUCT_LOCK's placeholder is for
        // 第三人姓名, and a rule that ate a capitalized word would redact the
        // user out of their own draft.
        "Roy 说这周先把方案定下来",
        // Digits and punctuation break a run rather than joining one.
        "下午 3 点，第 2 会议室，预算 45000",
        // A single group is a word, not a label.
        "他在 café 里等了很久",
    ] {
        let turn_id = Uuid::now_v7();
        let turns = vec![Turn::new(turn_id, SealedSubject::ThirdParty, intact)];
        let exemption = ExemptionRequest::for_turn(turn_id)
            .confirm(true)
            .expect("confirmed");
        let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

        assert_eq!(
            redacted.as_str(),
            intact,
            "the exempted turn came back changed, so the confirmation bought less \
             than the user was told it would",
        );
    }
}

/// The stricter rule belongs to the exempted turn and changes nothing else.
///
/// `a_confirmed_exemption_carries_exactly_one_original_and_the_next_draft_does_not`
/// asserts the draft after an exemption is byte-identical to the one before
/// it; this is the same statement made about a conversation whose turns carry
/// a display label, so a build that started placeholding labels on the default
/// path — where every third-party turn is already a whole placeholder — would
/// be visible rather than silent.
#[test]
fn the_default_path_is_unchanged_by_the_label_rule() {
    let redactor = Redactor::new(KnownIdentifiers::new());
    let turn_id = Uuid::now_v7();
    let turns = vec![
        Turn::new(
            turn_id,
            SealedSubject::ThirdParty,
            format!("{SPACED_LABEL} 说：{ORIGINAL}"),
        ),
        Turn::new(Uuid::now_v7(), SealedSubject::Owner, "我看看时间再回复。"),
    ];

    let before = redactor.redact_for_e1(&turns);
    assert_eq!(
        before.as_str(),
        format!("{THIRD_PARTY_PLACEHOLDER}\n我看看时间再回复。"),
    );

    let exemption = ExemptionRequest::for_turn(turn_id)
        .confirm(true)
        .expect("confirmed");
    let _ = redactor.redact_for_e1_with_exemption(&turns, exemption);

    assert_eq!(
        redactor.redact_for_e1(&turns).as_str(),
        before.as_str(),
        "the draft after an exemption must be identical to the one before it",
    );
}

/// An exemption names one turn. Presenting it against a conversation that does
/// not contain that turn changes nothing.
#[test]
fn an_exemption_for_an_absent_turn_placeholds_everything() {
    let redactor = redactor();
    let (_first, _second, turns) = conversation();

    let exemption = ExemptionRequest::for_turn(Uuid::now_v7())
        .confirm(true)
        .expect("confirmed");
    let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

    checker().assert_clean("a draft with a mismatched exemption", redacted.as_str());
    assert!(!redacted.carries_exempted_original());
    assert_eq!(redacted.placeheld_turns(), 2);
}

/// The redactor is stateless with respect to exemptions, so there is nothing
/// for a second call to inherit. Two independently built redactors must agree.
#[test]
fn the_redactor_carries_no_memory_between_calls() {
    let (turn_id, _other, turns) = conversation();

    let used = redactor();
    let exemption = ExemptionRequest::for_turn(turn_id)
        .confirm(true)
        .expect("confirmed");
    let _ = used.redact_for_e1_with_exemption(&turns, exemption);

    assert_eq!(
        used.redact_for_e1(&turns).as_str(),
        redactor().redact_for_e1(&turns).as_str(),
        "a redactor that has honoured an exemption must behave like a fresh one",
    );
}

/// The research path has no exemption parameter, and must not grow one.
///
/// A signature is invisible to a reader of the acceptance matrix, so this
/// reads the source back and asserts the shape. It is the same technique WP02
/// used to prove the research preview writes no files.
#[test]
fn the_research_path_has_no_exemption_entry_point() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/redactor.rs"),
    )
    .expect("read the redactor source");

    let research_fns: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub fn") && line.contains("research"))
        .collect();
    assert_eq!(
        research_fns.len(),
        1,
        "there should be exactly one research entry point; found {research_fns:#?}",
    );
    assert!(
        !research_fns[0].contains("xemption"),
        "the research path must not take an exemption: {}",
        research_fns[0],
    );

    // And nothing else may reach the exempted branch. There is exactly one
    // parameter of the exemption type in the whole module, and it belongs to
    // the E1 path.
    let lines: Vec<&str> = source.lines().map(str::trim).collect();
    let taking: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("exemption: OneShotExemption"))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        taking.len(),
        1,
        "exactly one function may consume an exemption; found {} at {taking:?}",
        taking.len(),
    );

    let owner = lines[..taking[0]]
        .iter()
        .rev()
        .find(|line| line.starts_with("pub fn") || line.starts_with("fn "))
        .expect("the parameter belongs to some function");
    assert_eq!(owner, &"pub fn redact_for_e1_with_exemption(");
}
