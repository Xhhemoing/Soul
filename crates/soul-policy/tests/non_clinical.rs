//! The runtime half of the non-clinical guarantee.
//!
//! `xtask denylist-audit` keeps the forbidden vocabulary out of the crate
//! sources, which covers what a developer typed. Anything a model returns or a
//! template assembles at runtime is not covered by that, and AC-16 asks for a
//! summary with no diagnostic vocabulary in it. Both halves read the same
//! file, so a word added to `fixtures/denylist/diagnostic_terms.txt` starts
//! being refused at runtime without another edit.

use soul_policy::clinical::{
    assert_non_clinical, denied_terms_in, term_count, DENYLIST_SOURCE, WORKING_HYPOTHESIS_NOTICE,
};

#[test]
fn the_embedded_denylist_matches_the_file_on_disk() {
    let on_disk = soul_testkit::fixtures::read_text("denylist/diagnostic_terms.txt")
        .expect("the denylist file loads");
    assert_eq!(
        DENYLIST_SOURCE, on_disk,
        "the embedded copy must be the same bytes xtask reads",
    );

    let from_fixture = soul_testkit::fixtures::denylist_terms().expect("terms parse");
    assert_eq!(term_count(), from_fixture.len());
    assert!(term_count() > 40, "only {} terms loaded", term_count());
}

#[test]
fn a_summary_that_makes_a_medical_claim_is_refused() {
    for claim in [
        "对方可能有抑郁倾向，建议就医",
        "This reads like a classic anxiety disorder presentation.",
        "他的症状符合双相的描述",
        "Consider a differential diagnosis before drafting.",
    ] {
        assert!(
            assert_non_clinical(claim).is_err(),
            "should have been refused: {claim}",
        );
    }
}

#[test]
fn an_ordinary_summary_passes() {
    for wording in [
        "最近两周你和对方的互动集中在工作日晚上，语气偏简短。",
        "证据档为中等：三条来自导入的消息，一条来自你的纠正。",
        "You reply faster to people you have met in person.",
        WORKING_HYPOTHESIS_NOTICE,
    ] {
        assert_non_clinical(wording).unwrap_or_else(|error| {
            panic!("ordinary wording was refused: {wording} ({error})");
        });
    }
}

/// The same word-boundary rule `xtask` uses, so the two halves agree about
/// what a hit is. A rating word inside a longer word is not a hit.
#[test]
fn matching_respects_word_boundaries_for_ascii_terms() {
    assert!(assert_non_clinical("underscore separated").is_ok());
    assert!(assert_non_clinical("Psychotherapy is a word this file names").is_err());
    assert!(assert_non_clinical("the underscore is fine").is_ok());
}

#[test]
fn the_reported_term_is_the_one_that_matched() {
    let hits = denied_terms_in("他的症状很像抑郁症，也可能是 bipolar");
    assert!(hits.contains(&"症状".to_owned()), "{hits:?}");
    assert!(hits.contains(&"抑郁症".to_owned()), "{hits:?}");
    assert!(hits.contains(&"bipolar".to_owned()), "{hits:?}");
}
