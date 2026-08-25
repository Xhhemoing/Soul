//! Exercises the leakage checker against the Unicode fixture.
//!
//! The fixture carries its own expectations, so adding a nasty new string
//! there is enough to extend the coverage.

use std::collections::BTreeSet;

use soul_testkit::fixtures;
use soul_testkit::leakage::{LeakageChecker, LeakageKind, DEFAULT_MIN_NGRAM};

#[test]
fn every_fixture_case_matches_its_expectation() {
    let fixture = fixtures::leakage_fixture().expect("leakage fixture loads");
    let checker = LeakageChecker::from_fixture(&fixture);
    assert_eq!(checker.min_ngram(), DEFAULT_MIN_NGRAM);
    assert!(
        fixture.cases.len() >= 10,
        "the fixture should keep covering the awkward cases, not shrink",
    );

    for case in &fixture.cases {
        let findings = checker.inspect(&case.text);
        assert_eq!(
            !findings.is_empty(),
            case.leaks,
            "case `{}` disagreed with the checker.\nnote: {}\nfindings: {findings:#?}",
            case.id,
            case.note.as_deref().unwrap_or("(none)"),
        );

        if !case.expect_kinds.is_empty() {
            let found: BTreeSet<LeakageKind> = findings.iter().map(|f| f.kind).collect();
            let expected: BTreeSet<LeakageKind> = case.expect_kinds.iter().copied().collect();
            assert_eq!(
                found, expected,
                "case `{}` produced the wrong kinds of finding",
                case.id,
            );
        }
    }
}

/// A two-scalar name is far shorter than the n-gram floor and must still hit.
#[test]
fn short_names_are_caught_regardless_of_length() {
    let mut checker = LeakageChecker::new();
    checker.add_known_identifier("name_li_lei", "李雷");
    let findings = checker.inspect("我准备回复李雷。");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, LeakageKind::KnownIdentifier);
    assert_eq!(findings[0].matched, "李雷");
}

#[test]
fn the_eight_scalar_boundary_is_exact() {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("plan", "明天上午十点在公司门口见");

    assert!(
        !checker.is_clean("摘要：明天上午十点在公"),
        "eight consecutive scalars is a leak",
    );
    assert!(
        checker.is_clean("摘要：明天上午十点在"),
        "seven consecutive scalars is below the threshold",
    );
}

/// The `≥8` floor is a policy choice, not a limit of the checker.
#[test]
fn lowering_the_threshold_catches_short_replies() {
    let fixture = fixtures::leakage_fixture().expect("fixture loads");
    let strict = LeakageChecker::from_fixture(&fixture).with_min_ngram(4);
    let candidate = "对方回了一句：好的没问题";

    assert!(
        LeakageChecker::from_fixture(&fixture).is_clean(candidate),
        "the default threshold deliberately misses a five-scalar reply",
    );
    assert!(
        !strict.is_clean(candidate),
        "at a threshold of four the same reply is caught",
    );
}

#[test]
fn normalization_works_in_both_directions() {
    let composed = "café 的午后";
    let decomposed = "cafe\u{0301} 的午后";
    assert_ne!(composed, decomposed, "the two spellings differ byte-wise");

    let mut from_nfc = LeakageChecker::new().with_min_ngram(4);
    from_nfc.add_third_party_body("nfc", composed);
    assert!(!from_nfc.is_clean(decomposed));

    let mut from_nfd = LeakageChecker::new().with_min_ngram(4);
    from_nfd.add_third_party_body("nfd", decomposed);
    assert!(!from_nfd.is_clean(composed));
}

/// A window must be able to span a ZWJ sequence instead of stopping at it.
#[test]
fn zwj_emoji_sequences_do_not_split_the_window() {
    let line = "团队里那位 \u{1F469}\u{200D}\u{1F4BB} 很靠谱";
    assert_eq!(line.chars().count(), 13, "three scalars for the ZWJ emoji");

    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("zwj", line);

    assert!(!checker.is_clean(&format!("复述：{line}。")));
    assert!(
        checker.is_clean("复述：团队里那位很靠谱。"),
        "dropping the emoji also drops the eight-scalar run",
    );
}

#[test]
fn an_empty_corpus_finds_nothing() {
    let checker = LeakageChecker::new();
    assert!(checker.is_clean("任意文本"));
    checker.assert_clean("empty corpus", "任意文本");
}
