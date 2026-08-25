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

/// The same promise for the same number, written in groups.
///
/// [`identifiers_stay_placeheld_inside_an_exempted_turn`] hands the redactor
/// `13800138000` and gets it back placeheld, which proves the identifier set is
/// consulted. A paste rarely spells a number that way: `138 0013 8000` is how
/// somebody writes one down for somebody else to read, it is not the string the
/// contact card holds, and it is eleven digits in three groups none of which is
/// long enough to be a number on its own.
///
/// The exempted turn is the one place a paste travels verbatim, so it is the
/// one place that difference reaches an endpoint. The redactor below knows the
/// number in its unspaced spelling only, so what covers the grouped one is the
/// shape and nothing else — and the message the user confirmed twice for still
/// has to come out the other side.
#[test]
fn a_grouped_phone_number_stays_placeheld_inside_an_exempted_turn() {
    let redactor = redactor();

    for grouped in ["138 0013 8000", "138-0013-8000"] {
        let turn_id = Uuid::now_v7();
        let turns = vec![Turn::new(
            turn_id,
            SealedSubject::ThirdParty,
            format!("{ORIGINAL}，电话 {grouped}"),
        )];

        let exemption = ExemptionRequest::for_turn(turn_id)
            .confirm(true)
            .expect("the user confirmed twice");
        let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

        assert_eq!(
            redacted.as_str(),
            format!("{ORIGINAL}，电话 {ACCOUNT_PLACEHOLDER}"),
            "`{grouped}` reached the body the user confirmed",
        );
        assert!(
            !redacted.as_str().contains("8000"),
            "the tail of the number travelled beside the placeholder: {}",
            redacted.as_str(),
        );
        assert!(redacted.carries_exempted_original());
    }
}

/// The same promise again, for the spellings a keyboard produces rather than
/// the ones a test author types.
///
/// The test above covers the ASCII hyphen and the ASCII space, which is what a
/// contact card and an English layout give. A Chinese IME in fullwidth mode
/// gives U+FF10–U+FF19 for the digits and U+FF0D for the dash; a paste out of
/// a document that has been autocorrected gives U+2013. None of them is the
/// string the contact card holds, so what covers them is the shape, and the
/// exempted turn is the one place a paste reaches an endpoint verbatim.
///
/// `138-0013–8000` is the case that says why the whole run has to go rather
/// than the first seven digits: with only the ASCII hyphen joining, the run
/// stopped at the en-dash and left `8000` standing beside the placeholder.
#[test]
fn a_phone_number_typed_on_an_ime_stays_placeheld_inside_an_exempted_turn() {
    let redactor = redactor();

    for typed in [
        "１３８００１３８０００",
        "１３８－００１３－８０００",
        "138\u{2013}0013\u{2013}8000",
        "138-0013\u{2013}8000",
    ] {
        let turn_id = Uuid::now_v7();
        let turns = vec![Turn::new(
            turn_id,
            SealedSubject::ThirdParty,
            format!("{ORIGINAL}，电话 {typed}"),
        )];

        let exemption = ExemptionRequest::for_turn(turn_id)
            .confirm(true)
            .expect("the user confirmed twice");
        let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

        assert_eq!(
            redacted.as_str(),
            format!("{ORIGINAL}，电话 {ACCOUNT_PLACEHOLDER}"),
            "`{typed}` reached the body the user confirmed",
        );
        assert!(
            !redacted.as_str().contains("8000") && !redacted.as_str().contains("８０００"),
            "the tail of the number travelled beside the placeholder: {}",
            redacted.as_str(),
        );
        assert!(redacted.carries_exempted_original());
    }
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

/// The same name written the way a person writes it, with no space to see.
///
/// [`an_exempted_turn_placeholds_a_display_label_nobody_registered`] covers the
/// spelling an export uses. This is the spelling everybody else uses: `李雷说…`
/// is four Han characters in a row and the shape scrub above cannot see a
/// boundary in it. What it can see is the position — a name stands in front of
/// a verb of saying — and on a Soul that has imported nobody that is the only
/// thing between the contact's name and the endpoint.
///
/// The placeholder has to be the name and not the clause: 场地 is what the
/// user confirmed twice to send.
#[test]
fn an_exempted_turn_placeholds_a_name_written_in_front_of_a_verb_of_saying() {
    let redactor = Redactor::new(KnownIdentifiers::new());
    assert!(redactor.identifiers().is_empty());

    let turn_id = Uuid::now_v7();
    let turns = vec![Turn::new(
        turn_id,
        SealedSubject::ThirdParty,
        "李雷说周五的场地他已经订好了，你直接过来就行",
    )];
    let exemption = ExemptionRequest::for_turn(turn_id)
        .confirm(true)
        .expect("the user confirmed twice");
    let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

    assert!(
        !redacted.as_str().contains(NAME),
        "a name the graph never learned reached the body the user confirmed: {}",
        redacted.as_str(),
    );
    assert_eq!(
        redacted.as_str(),
        format!("{NAME_PLACEHOLDER}说周五的场地他已经订好了，你直接过来就行"),
        "the placeholder is supposed to be the name and nothing either side of it",
    );
    assert!(redacted.carries_exempted_original());
}

/// The label in the script that spaces every word, in the same position.
///
/// `Wang Xiao` has no spelling to recognize — two capitalized words are how
/// English writes a good deal of a sentence — so the shape here is entirely
/// the position, and it is the same position in both scripts: a verb of saying
/// follows, whichever language the verb is in.
#[test]
fn an_exempted_turn_placeholds_a_latin_display_label_in_front_of_a_verb_of_saying() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for (paste, kept) in [
        (
            "Wang Xiao said Friday's venue is booked, just come straight over",
            "venue is booked",
        ),
        ("Wang Xiao 说这周先把方案定下来，别拖到下周", "方案"),
        ("Wang Xiao texted about the deposit this morning", "deposit"),
    ] {
        let turn_id = Uuid::now_v7();
        let turns = vec![Turn::new(turn_id, SealedSubject::ThirdParty, paste)];
        let exemption = ExemptionRequest::for_turn(turn_id)
            .confirm(true)
            .expect("the user confirmed twice");
        let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

        assert!(
            !redacted.as_str().contains("Wang Xiao"),
            "a display label nobody registered travelled verbatim: {}",
            redacted.as_str(),
        );
        assert!(
            redacted.as_str().contains(NAME_PLACEHOLDER),
            "the name was dropped rather than placeheld: {}",
            redacted.as_str(),
        );
        assert!(
            redacted.as_str().contains(kept),
            "the message the user confirmed did not travel: {}",
            redacted.as_str(),
        );
    }
}

/// What the two shapes together still cannot see, written down.
///
/// The screens promise 「姓名与账号两种情况下都占位」 without a condition, and
/// on a Soul that has imported nobody these strings are the distance between
/// that sentence and this file. Each one is a name that keeps its bytes: not
/// because doing so is right, but because no rule here can tell it from prose,
/// and a rule that tried would take away the message the user confirmed twice
/// to send.
///
/// The test asserts the current answer so that the hole is a fact somebody has
/// to change a test to move, rather than something to rediscover. What closes
/// it is the contact graph — [`KnownIdentifiers`], which `soulcore` fills from
/// the contact rows — or a step the user sees before the request leaves.
///
/// The second half of the test is that graph, and it registers `李 雷` with the
/// space in it, because that is the string a Telegram export seals and the
/// only spelling of that name `soulcore` ever hands the redactor. Handing it
/// `李雷` instead would have proved the set is consulted and nothing about the
/// product: the unspaced spelling is covered because `add_name` folds a spaced
/// label, not because anybody registered it.
#[test]
fn the_names_the_shapes_still_cannot_see_are_written_down_here() {
    let redactor = Redactor::new(KnownIdentifiers::new());

    for (paste, still_there) in [
        // Not in front of a verb of saying: nothing marks it as a name.
        ("周五的方案我下周交给李雷，你不用管", "李雷"),
        // Not a surname on the list, which is how friends write a name.
        ("小王说周五的场地他已经订好了", "小王"),
        // A second name inside the same run of Han characters.
        ("李雷说张伟明天也过来", "张伟"),
        // A Latin label anywhere but in front of the verb.
        ("The deposit is with Wang Xiao until Friday", "Wang Xiao"),
    ] {
        let turn_id = Uuid::now_v7();
        let turns = vec![Turn::new(turn_id, SealedSubject::ThirdParty, paste)];
        let exemption = ExemptionRequest::for_turn(turn_id)
            .confirm(true)
            .expect("confirmed");
        let redacted = redactor.redact_for_e1_with_exemption(&turns, exemption);

        assert!(
            redacted.as_str().contains(still_there),
            "`{still_there}` is now placeheld, which is better than this test \
             describes — move the case up into the tests above: {}",
            redacted.as_str(),
        );
    }

    // And the same names, once anything at all has been imported.
    let knowing = Redactor::new(
        KnownIdentifiers::new()
            .with_name(SPACED_LABEL)
            .with_name("小王")
            .with_name("张伟")
            .with_name("Wang Xiao"),
    );
    for paste in [
        "周五的方案我下周交给李雷，你不用管",
        "小王说周五的场地他已经订好了",
        "李雷说张伟明天也过来",
        "The deposit is with Wang Xiao until Friday",
    ] {
        let turn_id = Uuid::now_v7();
        let turns = vec![Turn::new(turn_id, SealedSubject::ThirdParty, paste)];
        let exemption = ExemptionRequest::for_turn(turn_id)
            .confirm(true)
            .expect("confirmed");
        let redacted = knowing.redact_for_e1_with_exemption(&turns, exemption);

        for name in ["李雷", "小王", "张伟", "Wang Xiao"] {
            assert!(
                !redacted.as_str().contains(name),
                "the contact graph is what covers these, and it did not: {}",
                redacted.as_str(),
            );
        }
    }
}

/// A label the graph learned is placeheld in the spelling a person writes, not
/// only the spelling the export sealed.
///
/// This is the product's own path and it has no shape in it: the turn below is
/// the user's own, so neither of the two shape rules runs, and a placeholder in
/// these bytes can only have come from the identifier set. `李 雷` is what
/// `soulcore` registers, because it is what the file said; `李雷` is what the
/// paste says, because that is how the name is written everywhere that is not
/// a contact card. Before the fold those were two different strings to a
/// `String::replace`, and the second one travelled.
#[test]
fn a_spaced_label_the_graph_learned_covers_the_unspaced_spelling_too() {
    let knowing = Redactor::new(KnownIdentifiers::new().with_name(SPACED_LABEL));

    let turns = vec![Turn::new(
        Uuid::now_v7(),
        SealedSubject::Owner,
        "周五的方案我下周交给李雷，你不用管",
    )];
    let redacted = knowing.redact_for_e1(&turns);

    assert!(
        !redacted.as_str().contains(NAME),
        "the name this Soul imported travelled in the spelling everybody uses: {}",
        redacted.as_str(),
    );
    assert!(
        redacted.as_str().contains(NAME_PLACEHOLDER),
        "the name was dropped rather than placeheld: {}",
        redacted.as_str(),
    );
    assert!(
        redacted.as_str().contains("方案"),
        "the placeholder is supposed to be the name and not the sentence: {}",
        redacted.as_str(),
    );

    // The spelling that was registered is of course still covered.
    let spaced = vec![Turn::new(
        Uuid::now_v7(),
        SealedSubject::Owner,
        format!("联系人卡片上写的是 {SPACED_LABEL}，别改"),
    )];
    assert!(
        !knowing
            .redact_for_e1(&spaced)
            .as_str()
            .contains(SPACED_LABEL),
        "folding a label may not cost it the spelling it was registered in",
    );
}

/// The fold stops where the label shape stops.
///
/// Taking the spaces out of a string is only safe while the string is shaped
/// like a name, because what comes out is matched literally against every draft
/// afterwards. Two things are deliberately outside the shape: a run longer than
/// a person's name, which is what a group title looks like; and a label in a
/// script that spaces its words anyway, where `WangXiao` is a spelling nobody
/// has ever typed and matching it would buy nothing.
///
/// Both are asserted through the user's own turn, where no shape rule runs, so
/// the answer is the identifier set's and nothing else's.
#[test]
fn a_label_that_is_not_shaped_like_a_name_is_registered_as_written_only() {
    for (label, registered_in_prose, folded_in_prose) in [
        // Four groups and seven characters: past the end of a name.
        (
            "项目 组 周会 通知",
            "群名片上写着项目 组 周会 通知，别动",
            "这次项目组周会通知发得有点晚",
        ),
        // A script that spaces every word carries no signal in the space.
        (
            "Wang Xiao",
            "The card still says Wang Xiao, leave it",
            "The WangXiao line in the sheet is a typo",
        ),
    ] {
        let knowing = Redactor::new(KnownIdentifiers::new().with_name(label));
        let folded: String = label.chars().filter(|c| *c != ' ').collect();

        let as_written = knowing.redact_for_e1(&[Turn::new(
            Uuid::now_v7(),
            SealedSubject::Owner,
            registered_in_prose,
        )]);
        assert!(
            !as_written.as_str().contains(label),
            "`{label}` was registered and did not travel as a placeholder: {}",
            as_written.as_str(),
        );

        let as_folded = knowing.redact_for_e1(&[Turn::new(
            Uuid::now_v7(),
            SealedSubject::Owner,
            folded_in_prose,
        )]);
        assert!(
            as_folded.as_str().contains(folded.as_str()),
            "`{folded}` is now placeheld, so the fold has grown past the label \
             shape — check that `add_name` and the spaced-label scrub still \
             agree on what a name looks like: {}",
            as_folded.as_str(),
        );
    }
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
        // An adverb standing where a name stands. 于 and 马 are surnames and
        // 说 is a verb of saying, so both signals the attribution shape reads
        // are present and the answer is still no.
        "于是说好了周五在会议室碰头",
        "马上说定，我这边没问题",
        // The user's own name in front of the same verb: one capitalized word
        // is not a display label, and the owner is not the third party.
        "Roy said the venue is booked already",
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

    // Both extra rules belong to the exempted turn and nowhere else. The
    // user's own words are not placeheld as a body, so a rule that had started
    // reading them would show up here as prose the writer never sent.
    let mine = vec![Turn::new(
        Uuid::now_v7(),
        SealedSubject::Owner,
        "李雷说周五的场地他已经订好了，Wang Xiao said the same thing",
    )];
    assert_eq!(
        redactor.redact_for_e1(&mine).as_str(),
        "李雷说周五的场地他已经订好了，Wang Xiao said the same thing",
        "the name shapes reached a turn that is not the exempted one",
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
