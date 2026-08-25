//! AC-12: the default E1 request body carries no third-party prose and no
//! unplaceheld name or account.
//!
//! The fixture corpus is `fixtures/leakage/third_party_unicode.json`, the same
//! one WP01 built the checker against, so this test inherits its Unicode
//! edges: an eight-scalar run is a leak and a seven-scalar run is not, NFC and
//! NFD compare equal in both directions, a ZWJ emoji sequence does not break a
//! window, and a name is a leak at two scalars.
//!
//! Every case is run twice. Once through the redactor, where the body must be
//! clean; and once with the redactor bypassed, where the same checker must
//! report the leak. Without the second half a redactor that returned the empty
//! string would pass.

use uuid::Uuid;

use soul_policy::redactor::{
    ExemptionRequest, KnownIdentifiers, RedactedBody, Redactor, Turn, ACCOUNT_PLACEHOLDER,
    NAME_PLACEHOLDER, THIRD_PARTY_PLACEHOLDER,
};
use soul_schema::common::SealedSubject;
use soul_testkit::leakage::{LeakageChecker, LeakageFixture};

fn fixture() -> LeakageFixture {
    soul_testkit::fixtures::leakage_fixture().expect("the leakage corpus loads")
}

fn checker(fixture: &LeakageFixture) -> LeakageChecker {
    LeakageChecker::from_fixture(fixture)
}

/// A redactor that knows the same names and handles the corpus does, as the
/// contact graph would supply them.
fn redactor(fixture: &LeakageFixture) -> Redactor {
    let mut identifiers = KnownIdentifiers::new();
    for entry in &fixture.known_identifiers {
        match entry.id.starts_with("name_") {
            true => identifiers.add_name(&entry.text),
            false => identifiers.add_account(&entry.text),
        };
    }
    Redactor::new(identifiers)
}

fn third_party(body: &str) -> Turn {
    Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, body)
}

fn own(body: &str) -> Turn {
    Turn::new(Uuid::now_v7(), SealedSubject::Owner, body)
}

fn body_text(body: &RedactedBody) -> String {
    body.as_str().to_owned()
}

#[test]
fn no_third_party_body_from_the_corpus_survives_the_default_path() {
    let fixture = fixture();
    let checker = checker(&fixture);
    let redactor = redactor(&fixture);

    assert!(
        fixture.third_party_bodies.len() >= 5,
        "the corpus should still hold the Unicode edge cases WP01 wrote",
    );

    for entry in &fixture.third_party_bodies {
        let turns = vec![
            own("我需要回复对方，语气保持简短。"),
            third_party(&entry.text),
        ];
        let redacted = redactor.redact_for_e1(&turns);

        checker.assert_clean(
            &format!("the default E1 body for corpus entry {}", entry.id),
            &body_text(&redacted),
        );
        assert!(
            redacted.as_str().contains(THIRD_PARTY_PLACEHOLDER),
            "{}: the third-party turn must be replaced, not dropped silently",
            entry.id,
        );
        assert_eq!(redacted.third_party_turns(), 1);
        assert_eq!(redacted.placeheld_turns(), 1);
        assert!(!redacted.carries_exempted_original());
    }
}

/// The other side of the same coin: with the redactor bypassed, every one of
/// those bodies is caught. A checker that never fires would make the test
/// above meaningless.
#[test]
fn the_same_corpus_leaks_when_the_redactor_is_bypassed() {
    let fixture = fixture();
    let checker = checker(&fixture);

    for entry in &fixture.third_party_bodies {
        let raw = format!("草稿参考：{}", entry.text);
        let findings = checker.inspect(&raw);
        let long_enough = entry.text.chars().count() >= checker.min_ngram();
        assert_eq!(
            !findings.is_empty(),
            long_enough,
            "{}: a body of {} scalars against a threshold of {} should {}have been caught",
            entry.id,
            entry.text.chars().count(),
            checker.min_ngram(),
            if long_enough { "" } else { "not " },
        );
    }
}

#[test]
fn names_and_accounts_are_placeheld_even_inside_the_users_own_words() {
    let fixture = fixture();
    let checker = checker(&fixture);
    let redactor = redactor(&fixture);

    // The user's own turn is not placeheld as a body, so any identifier in it
    // has to be caught by the identifier rule rather than by the subject rule.
    let turns = vec![own(
        "我准备回复李雷，抄送 lilei@example.invalid，必要时打 13800138000，或者找 @wang_xiao2。",
    )];
    let redacted = redactor.redact_for_e1(&turns);

    checker.assert_clean("the user's own turn", redacted.as_str());
    assert!(redacted.as_str().contains(NAME_PLACEHOLDER));
    assert!(redacted.as_str().contains(ACCOUNT_PLACEHOLDER));
    assert_eq!(
        redacted.third_party_turns(),
        0,
        "the user's own turn is not third-party prose",
    );
}

/// Identifier shapes the contact graph never learned about must still go.
#[test]
fn unregistered_identifier_shapes_are_placeheld_too() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    let redacted = redactor.redact_for_e1(&[own(
        "联系 someone.else@mail.invalid 或 @never_seen_handle，电话 15912345678。",
    )]);
    let text = redacted.as_str();

    assert!(!text.contains("someone.else@mail.invalid"), "{text}");
    assert!(!text.contains("@never_seen_handle"), "{text}");
    assert!(!text.contains("15912345678"), "{text}");
    assert_eq!(text.matches(ACCOUNT_PLACEHOLDER).count(), 3, "{text}");
}

/// The same number, written the way a person writes one down for another
/// person: in groups.
///
/// `13800138000` has a shape the scrub has always seen. `138 0013 8000` is the
/// same eleven digits and none of its groups is long enough to be a number on
/// its own, so until the groups were counted rather than the run, the number a
/// contact card would have had placeheld travelled verbatim out of the user's
/// own turn — which is the one turn that is never replaced wholesale.
///
/// The identifier set is empty, so a placeholder in these bodies can only have
/// come from the shape, and the whole grouped run has to go: a placeholder
/// with `8000` left beside it is still the last four digits on the wire.
#[test]
fn a_phone_number_written_in_groups_is_placeheld_on_the_default_path() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for grouped in [
        "138 0013 8000",
        "138-0013-8000",
        "138.0013.8000",
        "138　0013　8000",
        "010-1234-5678",
    ] {
        let redacted = redactor.redact_for_e1(&[own(&format!("回头打 {grouped} 找他。"))]);
        assert_eq!(
            redacted.as_str(),
            format!("回头打 {ACCOUNT_PLACEHOLDER} 找他。"),
            "`{grouped}` did not go whole",
        );
        assert_eq!(
            redacted.third_party_turns(),
            0,
            "the user's own turn is not third-party prose",
        );
    }
}

/// The rule the grouped shape was added beside, still doing its job.
///
/// A contiguous run of seven digits or more was the whole of the phone shape
/// before the groups were counted, and it is the spelling an export and a
/// contact card use. Widening a rule is the easiest way to lose the case it
/// started as, so the original one is pinned here on its own.
#[test]
fn a_contiguous_run_of_digits_is_still_placeheld() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    let redacted = redactor.redact_for_e1(&[own("回头打 13800138000 找他。")]);
    assert_eq!(
        redacted.as_str(),
        format!("回头打 {ACCOUNT_PLACEHOLDER} 找他。"),
    );

    // The boundary the number 7 draws, from both sides.
    assert_eq!(
        redactor
            .redact_for_e1(&[own("单号 1234567 和房间 123456")])
            .as_str(),
        format!("单号 {ACCOUNT_PLACEHOLDER} 和房间 123456"),
    );
}

/// The deliberate false positive, written down so it is a fact somebody has to
/// change a test to move.
///
/// `2026-08-25` is eight digits in three groups joined by single hyphens, so
/// the grouped shape reads it as a number and replaces it. It is a date, and
/// prose that quotes one comes back with a placeholder where the date was.
///
/// The alternative was to excuse the `\d{4}-\d{2}-\d{2}` shape by name, which
/// would also excuse any number punctuated 4-2-2. The trade taken here is the
/// same one-directional one `KnownIdentifiers::add_name` takes: a placeholder
/// too many is something the user can see and work around, and a number on the
/// wire is not.
#[test]
fn an_iso_date_is_placeheld_too_and_that_is_the_trade() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    let redacted = redactor.redact_for_e1(&[own("合同签在 2026-08-25，别记错。")]);
    assert_eq!(
        redacted.as_str(),
        format!("合同签在 {ACCOUNT_PLACEHOLDER}，别记错。"),
        "if the ISO date now survives, the shape has been narrowed — say so \
         here and in `phone_shape_end`, and check a 4-2-2 number still goes",
    );

    // A date written the way Chinese prose writes one has Han characters
    // between its groups, so nothing joins them and it is left alone.
    assert_eq!(
        redactor
            .redact_for_e1(&[own("合同签在 2026 年 8 月 25 日，别记错。")])
            .as_str(),
        "合同签在 2026 年 8 月 25 日，别记错。",
    );
}

/// The corpus's name, written the way an export writes a display label, with
/// nothing registered and the turn exempted.
///
/// The one hole the shape scrub above cannot cover on its own. `13800138000`
/// and `@wang_xiao2` have shapes; `李 雷` is two ordinary characters and a
/// space, and until the contact rows fill [`KnownIdentifiers`] there is
/// nothing to match it against — which is the state of every Soul that has
/// imported nothing. Everywhere but the exempted turn that costs nothing,
/// because a third-party turn is a placeholder whole; the turn the user
/// confirmed twice for is the one place the label would travel.
///
/// The identifier set is deliberately empty, so a placeholder in the body
/// below can only have come from the shape.
#[test]
fn a_display_label_nobody_registered_is_placeheld_inside_an_exempted_turn() {
    let redactor = Redactor::new(KnownIdentifiers::new());
    let mut checker = LeakageChecker::new();
    checker.add_known_identifier("name_li_lei_spaced", "李 雷");

    let turn = third_party("李 雷 说周五的场地他已经订好了，你直接过来就行");
    let exemption = ExemptionRequest::for_turn(turn.turn_id)
        .confirm(true)
        .expect("the user confirmed twice");
    let redacted = redactor.redact_for_e1_with_exemption(&[turn], exemption);

    checker.assert_clean(
        "an exempted body on a Soul that imported nobody",
        redacted.as_str(),
    );
    assert!(
        redacted.as_str().contains(NAME_PLACEHOLDER),
        "{}",
        redacted.as_str()
    );
    assert!(
        redacted.as_str().contains("场地"),
        "the message the user confirmed did not travel: {}",
        redacted.as_str(),
    );
    assert!(redacted.carries_exempted_original());
}

/// The same corpus name, spelled the way a person spells it, and the Latin
/// label beside it — both inside the turn the user confirmed twice for.
///
/// The test above needs the space. This one has none to work with: `李雷` is
/// two of four Han characters in a row, and `Wang Xiao` is two capitalized
/// words in a language that capitalizes. What is left to recognize is the
/// position — both stand in front of a verb of saying, because that is what a
/// chat log is made of — and the checker below is asked about the name rather
/// than about the sentence, so a redactor that ate the whole turn would not
/// pass either: each case names the words that have to survive.
#[test]
fn a_name_in_front_of_a_verb_of_saying_is_placeheld_inside_an_exempted_turn() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for (id, name, paste, kept) in [
        (
            "name_li_lei_unspaced",
            "李雷",
            "李雷说周五的场地他已经订好了，你直接过来就行",
            "场地",
        ),
        (
            "name_wang_xiao_latin",
            "Wang Xiao",
            "Wang Xiao said Friday's venue is booked, come straight over",
            "venue",
        ),
    ] {
        let mut checker = LeakageChecker::new();
        checker.add_known_identifier(id, name);

        let turn = third_party(paste);
        let exemption = ExemptionRequest::for_turn(turn.turn_id)
            .confirm(true)
            .expect("the user confirmed twice");
        let redacted = redactor.redact_for_e1_with_exemption(&[turn], exemption);

        checker.assert_clean(
            &format!("an exempted body naming {name} on a Soul that imported nobody"),
            redacted.as_str(),
        );
        assert!(
            redacted.as_str().contains(NAME_PLACEHOLDER),
            "{}",
            redacted.as_str()
        );
        assert!(
            redacted.as_str().contains(kept),
            "the message the user confirmed did not travel: {}",
            redacted.as_str(),
        );
        assert!(redacted.carries_exempted_original());
    }
}

/// A short number is not an account. Placeholders that fire on everything are
/// as useless as placeholders that never fire.
#[test]
fn short_numbers_and_ordinary_words_are_left_alone() {
    let redactor = Redactor::new(KnownIdentifiers::new());
    let redacted = redactor.redact_for_e1(&[own("下午 3 点，第 2 会议室，预算 45000。")]);
    assert_eq!(redacted.as_str(), "下午 3 点，第 2 会议室，预算 45000。");
}

/// `mixed` is handled as third-party. PRODUCT_LOCK says so, and the part that
/// is not the user's is the part that matters.
#[test]
fn mixed_subject_prose_is_treated_as_third_party() {
    let fixture = fixture();
    let checker = checker(&fixture);
    let redactor = redactor(&fixture);

    let mixed = Turn::new(
        Uuid::now_v7(),
        SealedSubject::Mixed,
        "明天上午十点在公司门口见，我说好。",
    );
    let redacted = redactor.redact_for_e1(&[mixed]);

    assert_eq!(redacted.as_str(), THIRD_PARTY_PLACEHOLDER);
    assert_eq!(redacted.third_party_turns(), 1);
    checker.assert_clean("a mixed-subject turn", redacted.as_str());
}

/// The research path drops third-party turns rather than placeholding them,
/// because the research preview counts third-party rows and the count is zero.
#[test]
fn the_research_path_emits_no_third_party_line_at_all() {
    let fixture = fixture();
    let checker = checker(&fixture);
    let redactor = redactor(&fixture);

    let turns = vec![
        own("我今天写了周报。"),
        third_party("明天上午十点在公司门口见"),
        own("我回复说好。"),
    ];
    let redacted = redactor.redact_for_research(&turns);

    checker.assert_clean("the research preview body", redacted.as_str());
    assert!(
        !redacted.as_str().contains(THIRD_PARTY_PLACEHOLDER),
        "research output has no third-party row to placehold: {}",
        redacted.as_str(),
    );
    assert_eq!(redacted.third_party_turns(), 1);
    assert_eq!(redacted.as_str().lines().count(), 2);
}

/// Normalization applies on both sides, so a decomposed spelling in the input
/// cannot slip past a composed corpus entry.
#[test]
fn a_decomposed_spelling_is_caught_by_the_composed_corpus() {
    let fixture = fixture();
    let checker = checker(&fixture);
    let redactor = redactor(&fixture);

    // "café" written with a combining acute, in a third-party turn.
    let decomposed = "今晚在 cafe\u{0301} 见面聊一下";
    assert!(
        !checker.is_clean(decomposed),
        "the corpus must catch this before redaction, or the test proves nothing",
    );

    let redacted = redactor.redact_for_e1(&[third_party(decomposed)]);
    checker.assert_clean("a decomposed third-party body", redacted.as_str());
}
