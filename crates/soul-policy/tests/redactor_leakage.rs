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

/// The label an export sealed and the spelling a person types.
///
/// Telegram joins `first_name` and `last_name` with a space, so the label
/// stored for 李雷 is `李 雷`, and nobody writing about him types the space.
/// `scrub_identifiers` is a `replace` over the strings in the set, so for as
/// long as the set held only the export's spelling the ordinary one went out
/// verbatim — inside the user's own words on the default path, and inside the
/// confirmed body itself on the exemption path.
#[test]
fn a_name_stored_with_a_space_is_placeheld_when_the_paste_leaves_it_out() {
    let redactor = Redactor::new(KnownIdentifiers::new().with_name("李 雷"));
    let mention = "李雷说周五的场地已经订好了";

    let default = redactor.redact_for_e1(&[own(mention)]);
    assert!(
        !default.as_str().contains("李雷"),
        "the ordinary spelling of a name Soul knows reached the body: {}",
        default.as_str(),
    );
    assert!(
        default.as_str().contains(NAME_PLACEHOLDER),
        "the name was dropped rather than placeheld: {}",
        default.as_str(),
    );
    assert!(
        default.as_str().contains("场地"),
        "the placeholder swallowed the sentence around it: {}",
        default.as_str(),
    );

    // The hardest case, because the whole turn travels: a placeholder in it
    // can only have come from the identifier set.
    let turn = third_party(mention);
    let exemption = ExemptionRequest::for_turn(turn.turn_id)
        .confirm(true)
        .expect("the user confirmed twice");
    let exempted = redactor.redact_for_e1_with_exemption(&[turn], exemption);
    assert!(
        !exempted.as_str().contains("李雷"),
        "an exemption is for one message's prose, not for a name: {}",
        exempted.as_str(),
    );
    assert!(exempted.as_str().contains(NAME_PLACEHOLDER));
    assert!(exempted.as_str().contains("场地"));

    // And the spelling the export sealed is still the one it was.
    let spaced = redactor.redact_for_e1(&[own("李 雷说周五的场地已经订好了")]);
    assert!(!spaced.as_str().contains("李 雷"), "{}", spaced.as_str());
    assert!(spaced.as_str().contains(NAME_PLACEHOLDER));
}

/// The other side of that: a label is registered whole, never in pieces.
///
/// Registering `李` and `雷` separately would catch the unspaced spelling too,
/// and would placehold 李先生 and every other ordinary use of those characters
/// out of the user's own prose. That is a worse failure than the one above,
/// and it is the one this pins against.
#[test]
fn a_character_from_a_known_name_is_still_an_ordinary_word() {
    let redactor = Redactor::new(KnownIdentifiers::new().with_name("李 雷"));
    let text = "李先生来了，雷声也停了，我们照常开会。";

    let redacted = redactor.redact_for_e1(&[own(text)]);
    assert_eq!(
        redacted.as_str(),
        text,
        "a name's characters were placeheld out of the user's own sentence",
    );
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
///
/// The dash a number is grouped by is whichever one the keyboard produced.
/// An IME on a Chinese layout gives U+FF0D, a paste out of a document that has
/// been through an autocorrect gives U+2013 or U+2014, and none of them is the
/// ASCII hyphen the first version of this rule knew. They are all the same
/// number written down for somebody to read, so they are all listed here.
#[test]
fn a_phone_number_written_in_groups_is_placeheld_on_the_default_path() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for grouped in [
        "138 0013 8000",
        "138-0013-8000",
        "138.0013.8000",
        "138　0013　8000",
        "010-1234-5678",
        // The dash family a paste or an IME actually produces.
        "138\u{2010}0013\u{2010}8000",
        "138\u{2011}0013\u{2011}8000",
        "138\u{2012}0013\u{2012}8000",
        "138\u{2013}0013\u{2013}8000",
        "138\u{2014}0013\u{2014}8000",
        "138\u{2212}0013\u{2212}8000",
        "138\u{FF0D}0013\u{FF0D}8000",
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

/// The same number again, typed on the layout most of this product's users
/// have in front of them.
///
/// A Chinese IME in fullwidth mode gives U+FF10–U+FF19 rather than ASCII, and
/// `１３８００１３８０００` is the same eleven digits as `13800138000` to every
/// reader and to every phone. It is not the same string to `is_ascii_digit`,
/// which is what the phone shape was written against, so the number a contact
/// card would have had placeheld went out of the user's own turn verbatim —
/// the one turn that is never replaced wholesale.
///
/// The same mode gives U+FF0E for the dot key, so a number grouped by dots on
/// a fullwidth IME carries neither an ASCII digit nor an ASCII separator.
///
/// The identifier set is empty, so a placeholder here can only be the shape.
#[test]
fn a_phone_number_typed_in_fullwidth_digits_is_placeheld() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for typed in [
        // Contiguous, the spelling an IME produces when nobody groups it.
        "１３８００１３８０００",
        // And grouped, by each separator the ASCII spelling is grouped by.
        "１３８ ００１３ ８０００",
        "１３８　００１３　８０００",
        "１３８－００１３－８０００",
        "１３８-００１３-８０００",
        "１３８\u{2013}００１３\u{2013}８０００",
        "１３８.００１３.８０００",
        // The dot the same fullwidth mode gives for the same key: U+FF0E.
        "１３８．００１３．８０００",
        // Half typed in one mode and half in the other, which is what a
        // partly-corrected line looks like.
        "138００１３8000",
        "１３８-0013－8000",
        "138．0013．8000",
    ] {
        let redacted = redactor.redact_for_e1(&[own(&format!("回头打 {typed} 找他。"))]);
        assert_eq!(
            redacted.as_str(),
            format!("回头打 {ACCOUNT_PLACEHOLDER} 找他。"),
            "`{typed}` did not go whole",
        );
        assert!(
            !redacted.as_str().contains('８') && !redacted.as_str().contains('8'),
            "a digit of the number stood beside the placeholder: {}",
            redacted.as_str(),
        );
    }
}

/// One number, two kinds of dash, and the tail that used to survive.
///
/// `138-0013–8000` is what a line looks like after somebody retyped part of it
/// or a document autocorrected only the dash it thought was a range. When the
/// ASCII hyphen joined groups and the en-dash did not, the run stopped at the
/// en-dash: seven digits was already enough to place a placeholder, and
/// `8000` — the last four digits of the number — stayed on the wire beside it.
/// That is the exact failure [`soul_policy::redactor`]'s phone shape says it
/// exists to prevent, so it is pinned here by the tail rather than by the
/// whole string.
#[test]
fn a_number_grouped_by_two_different_dashes_leaves_no_tail() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for mixed in [
        "138-0013\u{2013}8000",
        "138\u{2013}0013-8000",
        "138 0013\u{FF0D}8000",
        "138\u{FF0D}0013 8000",
        // The dot in both widths: the ASCII one joined and the fullwidth one
        // did not, so the run stopped one group short of the tail.
        "138.0013\u{FF0E}8000",
        "138\u{FF0E}0013.8000",
    ] {
        let redacted = redactor.redact_for_e1(&[own(&format!("回头打 {mixed} 找他。"))]);
        assert!(
            !redacted.as_str().contains("8000"),
            "the tail of the number travelled beside the placeholder: {}",
            redacted.as_str(),
        );
        assert_eq!(
            redacted.as_str(),
            format!("回头打 {ACCOUNT_PLACEHOLDER} 找他。"),
            "`{mixed}` did not go whole",
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

/// The false positives the dash family, the fullwidth digits and the
/// fullwidth dot add, in the same place and on the same terms as the ISO date
/// above.
///
/// A year range is the one thing an en-dash is used for far more often than a
/// phone number, and `2019–2026` is eight digits in two groups joined by one,
/// so it reads as a number and goes. `２０２６－０８－２５` is the ISO date
/// again, typed on an IME, and `２０２６．０８．２５` is that date with the dot
/// the same IME gives — a fullwidth-dotted digit run adding to seven digits or
/// more is placeheld exactly as the ASCII-dotted `2026.08.25` already was.
/// Each is the price of one widening, and each is the same one-directional
/// trade `phone_shape_end` already documents: a placeholder too many is
/// something the user can see and work around, and the last four digits of
/// somebody's number on the wire are not.
///
/// Naming the year-range and date shapes to excuse them would excuse every
/// number punctuated the same way along with them, which is letting a number
/// through because of how it was written.
#[test]
fn a_year_range_and_a_fullwidth_date_are_placeheld_too_and_that_is_the_trade() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for (body, expected) in [
        (
            "这份材料覆盖 2019\u{2013}2026，别记错。",
            format!("这份材料覆盖 {ACCOUNT_PLACEHOLDER}，别记错。"),
        ),
        (
            "合同签在 ２０２６－０８－２５，别记错。",
            format!("合同签在 {ACCOUNT_PLACEHOLDER}，别记错。"),
        ),
        (
            "合同签在 ２０２６．０８．２５，别记错。",
            format!("合同签在 {ACCOUNT_PLACEHOLDER}，别记错。"),
        ),
    ] {
        assert_eq!(
            redactor.redact_for_e1(&[own(body)]).as_str(),
            expected,
            "if this now survives, the shape has been narrowed — say so here \
             and in `phone_shape_end`, and check the number spellings it was \
             widened for still go",
        );
    }

    // Seven digits is still the floor, whichever way the range is punctuated:
    // a two-group range of six digits is not a number.
    assert_eq!(
        redactor
            .redact_for_e1(&[own("这份材料覆盖 201\u{2013}206，别记错。")])
            .as_str(),
        "这份材料覆盖 201\u{2013}206，别记错。",
    );
}
