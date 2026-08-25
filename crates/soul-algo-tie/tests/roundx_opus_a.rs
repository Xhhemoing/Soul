//! Round X, slot `opus-a`: adversarial re-verification of the frozen T4D.
//!
//! `docs/algorithms/DECISION.md` declares `ALGO_FROZEN` on T4D. This file is
//! the cross-check the freeze invites in §5, and it is written the way the rest
//! of the suite is written: a claim the product makes, and the evidence for or
//! against it.
//!
//! The band arithmetic survives. Everything in §4 below re-derives it —
//! including the two intersections the round's own tests never cross, the
//! 180/360 edges measured through a group row and the ladder measured across
//! the epoch — and finds it exact. **The band is not where the defects are.**
//!
//! Three findings, and they are all about what the score *says* rather than
//! what it decides:
//!
//! 1. §1 — `explain_zh` picks its wording from `score.band` and
//!    `score.detail` and never checks either against the counts it is about to
//!    print in the same sentence. Whenever `detail` is not the exact
//!    `Demoted { band_before }` that produced the band, the sentence asserts
//!    gates the evidence does not clear. `detail` is precisely the field
//!    `DECISION.md` §6.4 leaves out of the stored `TieScore`, so *every* score
//!    read back out of the graph is in that state — this is not only the
//!    foreign-rule path `src/lib.rs` documents. Observed on T4D's own scores:
//!    「你们一对一聊过 12 次，双方都发过，但不到 3 次，还只是打过招呼」 and
//!    「只分布在 6 天里，不到 3 天」. Observed on a stored Goal 1 band:
//!    「你们一对一聊过 0 次，双方都发过，不少于 10 次，而且分布在 0 天里，也不
//!    少于 3 天」. Eleven of the twenty-one fixtures are affected, over 26
//!    fixture/rule combinations: 11 on a stored Goal 1 band, 9 on T4's own
//!    scores, 6 on T4D's own.
//! 2. §2 — [`Tally`] uses `0` as the "no private exchange yet" sentinel for
//!    `last_direct_contact`, so a genuine one-to-one row at the epoch second is
//!    read as "never" and can be overwritten by an *older* private row.
//!    `tests/as_of_discipline.rs::a_last_direct_contact_before_the_epoch_is_still_reported`
//!    says in its comment that it covers "a real exchange at exactly the epoch
//!    second, or before it"; its body only covers "before it".
//! 3. §3 — the any-venue recency clock is priced in `DECISION.md` §4.3 against
//!    a private history that "stopped 200 days ago". Nothing bounds it. One
//!    *incoming* group message holds Strong over a private history of any age,
//!    and the explanation never names the age of that history, so the user
//!    cannot see it either.
//!
//! ## How to read the test names
//!
//! `todays_*` tests pin current behaviour, including where that behaviour is
//! wrong; they pass, and they go red when somebody fixes the defect, which is
//! the point. The `#[ignore]`d `bug_*` tests assert the behaviour the product
//! lock requires and fail today. `cargo test -p soul-algo-tie -- --ignored`
//! is the list of open defects.

use soul_algo_tie::constants::{
    MODERATE_MIN_INTERACTIONS, STRONG_MIN_ACTIVE_DAYS, STRONG_MIN_INTERACTIONS,
};
use soul_algo_tie::testing::oracle::T0;
use soul_algo_tie::testing::{self, direct, group, DAY};
use soul_algo_tie::{
    age_days, explain_zh, zh_date, Band, Detail, Interaction, Tally, TieAlgo, TieAlgorithm,
    TieScore, T4D,
};

// ---------------------------------------------------------------------------
// 1. An explanation may not contradict the counts printed beside it
// ---------------------------------------------------------------------------

/// Every factual claim T4D's wording can make about the evidence, checked
/// against the score the wording was rendered from.
///
/// This is not a spell-check. Each entry is a sentence fragment that only
/// appears when a specific gate was cleared or missed, paired with the
/// arithmetic that fragment asserts. A user recounting their own messages is
/// doing exactly this comparison, which is what C3 in the shared brief asks
/// for; if it fails, the explanation told them something untrue.
fn contradictions(score: &TieScore, text: &str) -> Vec<String> {
    let direct = score.direct_count();
    let days = score.direct_active_day_count;
    let claims: [(String, bool); 9] = [
        (
            format!("不少于 {STRONG_MIN_INTERACTIONS} 次"),
            direct >= STRONG_MIN_INTERACTIONS,
        ),
        (
            format!("不到 {STRONG_MIN_INTERACTIONS} 次"),
            direct < STRONG_MIN_INTERACTIONS,
        ),
        (
            format!("也不少于 {STRONG_MIN_ACTIVE_DAYS} 天"),
            days >= STRONG_MIN_ACTIVE_DAYS,
        ),
        (
            format!("不到 {STRONG_MIN_ACTIVE_DAYS} 天，看不出是长期习惯"),
            days < STRONG_MIN_ACTIVE_DAYS,
        ),
        (
            format!("不到 {MODERATE_MIN_INTERACTIONS} 次，还只是打过招呼"),
            direct < MODERATE_MIN_INTERACTIONS,
        ),
        ("双方都发过".to_string(), score.is_direct_reciprocal()),
        ("从来没有单独聊过".to_string(), direct == 0),
        (
            "对方一次也没有单独回过你".to_string(),
            score.direct_in_count == 0,
        ),
        (
            "本来就已经是最弱的一档".to_string(),
            score.band == Band::Weak,
        ),
    ];
    claims
        .into_iter()
        .filter(|(fragment, holds)| text.contains(fragment.as_str()) && !holds)
        .map(|(fragment, _)| fragment)
        .collect()
}

#[test]
fn every_t4d_explanation_of_a_t4d_score_is_consistent_with_its_own_counts() {
    // The baseline that makes the next three tests mean something: on scores
    // T4D produced itself, the wording and the counts agree everywhere. The
    // defect is not in the templates, it is in what selects between them.
    for f in testing::all() {
        let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
        let text = explain_zh(&score);
        assert_eq!(
            contradictions(&score, &text),
            Vec::<String>::new(),
            "{}: {text}",
            f.name
        );
    }
}

#[test]
fn a_rehydrated_score_is_still_explained_from_the_counts() {
    // `DECISION.md` §6.4 fixes what a stored `TieScore` carries: band, raw
    // counts, the one-to-one/group split, `last_contact`, silent days and
    // `as_of`. `detail` is not on that list — there is nowhere to put it and
    // nothing to recompute it from — so a score read back out of the graph has
    // to pick a `Detail`, and `RawCounts` is the only honest choice for "these
    // are the counts, I did not run the recency step".
    //
    // The band here is T4D's own, correct, Moderate. Only the working is
    // missing, and the explanation invents a replacement.
    let f = testing::quiet_200_days();
    let mut score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(score.band, Band::Moderate);
    assert_eq!(score.direct_count(), 12);
    assert_eq!(score.direct_active_day_count, 6);
    assert_eq!(
        score.detail,
        Detail::Demoted {
            band_before: Band::Strong
        }
    );

    let truthful = explain_zh(&score);
    assert!(truthful.contains("本来可以算强联系"), "{truthful}");
    assert!(truthful.contains("往下降到中等联系"), "{truthful}");
    assert_eq!(contradictions(&score, &truthful), Vec::<String>::new());

    score.detail = Detail::RawCounts;
    let rehydrated = explain_zh(&score);
    assert!(rehydrated.contains("本来可以算强联系"), "{rehydrated}");
    assert!(rehydrated.contains("往下降到中等联系"), "{rehydrated}");
    assert!(
        contradictions(&score, &rehydrated).is_empty(),
        "{rehydrated}"
    );
}

#[test]
fn a_stored_goal1_band_is_explained_from_the_counts_not_the_stored_label() {
    assert!(TieAlgo::from_id(T0::ID).is_none());

    let f = testing::group_only_50();
    let stored = T0::score(f.peer_id, &f.log, f.as_of);
    assert_eq!(stored.band, Band::Strong);
    assert_eq!(stored.direct_count(), 0);

    let text = explain_zh(&stored);
    assert!(
        text.contains("一对一 0 次") || text.contains("从来没有单独聊过"),
        "{text}"
    );
    assert!(
        !text.contains("不少于 10 次"),
        "must not claim a Strong gate the counts fail: {text}"
    );
    assert!(contradictions(&stored, &text).is_empty(), "{text}");
}

#[test]
fn a_rehydrated_weak_band_does_not_call_twelve_exchanges_fewer_than_three() {
    let f = testing::dormant_360_days();
    let mut score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    assert_eq!(score.band, Band::Weak);
    assert_eq!(score.direct_count(), 12);

    score.detail = Detail::RawCounts;
    let text = explain_zh(&score);
    assert!(
        !text.contains("不到 3 次"),
        "counts-stage Strong must not be narrated as a greeting: {text}"
    );
    assert!(contradictions(&score, &text).is_empty(), "{text}");
}

#[test]
fn an_explanation_never_contradicts_the_counts_it_prints() {
    // The property the product lock requires («算法必须可向用户解释») and the
    // fix this file is asking for: the wording must be selected from the
    // evidence, not from a band and a `detail` that arrived alongside it.
    // Recomputing the counts stage inside `explain_zh` — or refusing to explain
    // a score whose band does not follow from its own counts — would satisfy
    // this, and neither touches a band.
    //
    // The failure list is the blast radius: eleven fixtures over 26
    // fixture/rule combinations, both product rules included, not just the
    // stored-Goal-1-band path.
    let mut offenders = Vec::new();
    for f in testing::all() {
        let mut candidates = vec![T0::score(f.peer_id, &f.log, f.as_of)];
        let fresh = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
        let mut rehydrated = fresh.clone();
        rehydrated.detail = Detail::RawCounts;
        candidates.push(fresh);
        candidates.push(rehydrated);
        for score in candidates {
            let text = explain_zh(&score);
            let found = contradictions(&score, &text);
            if !found.is_empty() {
                offenders.push(format!("{} / {}: {found:?}", f.name, score.algorithm_id));
            }
        }
    }
    assert_eq!(offenders, Vec::<String>::new());
}

// ---------------------------------------------------------------------------
// 2. `last_direct_contact` uses 0 for two different things
// ---------------------------------------------------------------------------

#[test]
fn a_private_exchange_at_the_epoch_second_is_kept() {
    let log = vec![direct(1, true, 0, 1), direct(1, false, -DAY, 1)];
    let tally = Tally::of(1, &log);
    assert_eq!(tally.direct_count(), 2);
    assert_eq!(tally.last_contact, 0, "the any-venue clock is unaffected");
    assert_eq!(tally.last_direct_contact, 0, "newest private row is at 0");

    let reversed = vec![direct(1, false, -DAY, 1), direct(1, true, 0, 1)];
    assert_eq!(Tally::of(1, &reversed).last_direct_contact, 0);

    let as_of = DAY * 30;
    assert_eq!(
        T4D::score(1, &log, as_of).last_direct_contact_unix,
        T4D::score(1, &reversed, as_of).last_direct_contact_unix,
    );
}

#[test]
fn last_direct_contact_is_the_newest_private_row() {
    // Exhaustive over two-row logs straddling the epoch. The fix is an
    // `Option<i64>`, or tracking "has a private row" separately from the
    // timestamp; the sentinel cannot be repaired by moving it, because every
    // i64 is a legal Unix second.
    let times = [-2 * DAY, -DAY, 0, DAY];
    let mut offenders = Vec::new();
    for &first in &times {
        for &second in &times {
            for &first_direct in &[true, false] {
                for &second_direct in &[true, false] {
                    let log = vec![
                        Interaction {
                            peer_id: 1,
                            outgoing: true,
                            occurred_at_unix: first,
                            venue_direct: first_direct,
                            conversation_id: 1,
                        },
                        Interaction {
                            peer_id: 1,
                            outgoing: false,
                            occurred_at_unix: second,
                            venue_direct: second_direct,
                            conversation_id: 1,
                        },
                    ];
                    let Some(newest) = log
                        .iter()
                        .filter(|row| row.venue_direct)
                        .map(|row| row.occurred_at_unix)
                        .max()
                    else {
                        continue;
                    };
                    let got = Tally::of(1, &log).last_direct_contact;
                    if got != newest {
                        offenders.push(format!("{log:?}: expected {newest}, got {got}"));
                    }
                }
            }
        }
    }
    assert_eq!(offenders, Vec::<String>::new());
}

#[test]
fn the_never_sentinel_is_not_safe_to_read_as_an_age() {
    // The other half of the same overload, pinned because the crate's own
    // `ablation.rs` reads `age_days(score.last_direct_contact_unix, ..)` and
    // would get this on any group-only tie. Not a wrong band — a field no
    // caller can render without a null check the type does not force.
    let f = testing::group_only_50();
    let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    assert!(score.is_group_only());
    assert_eq!(score.last_direct_contact_unix, 0);
    assert_eq!(
        age_days(score.last_direct_contact_unix, score.as_of_unix),
        20_689
    );
    assert_eq!(zh_date(score.last_direct_contact_unix), "1970 年 1 月 1 日");
}

// ---------------------------------------------------------------------------
// 3. The any-venue clock is priced at 200 days and is not bounded at all
// ---------------------------------------------------------------------------

/// Twelve reciprocal one-to-one exchanges over six days ending `stale` days
/// ago, plus one *incoming* group message yesterday.
///
/// Incoming on purpose: the user did nothing, and the peer did nothing
/// addressed to the user. Somebody posted in a channel they both happen to be
/// in.
fn private_history_plus_one_group_ping(stale: i64) -> Vec<Interaction> {
    let mut log = Vec::new();
    for day in 0..6i64 {
        log.push(direct(1, true, testing::at(stale + day, 9), 1));
        log.push(direct(1, false, testing::at(stale + day, 10), 1));
    }
    log.push(group(1, false, testing::at(1, 10), 2));
    log
}

#[test]
fn one_incoming_group_message_holds_strong_over_a_private_history_of_any_age() {
    // `DECISION.md` §4.3 states the cost of the any-venue clock as «一对一历史
    // 停在 200 天前、群聊昨天还活跃 → 不降档», and prices it: «该关系的 Strong
    // 资格来自真实的一对一历史». The price is quoted against one number and
    // the mechanism has no number in it — the demotion step never reads
    // `last_direct_contact`, so there is no age at which the private history
    // stops qualifying.
    //
    // §4.2 records the neighbouring limitation, «七年休眠 + 昨天一来一回 →
    // Strong», as a rollback trigger — but that one at least requires a
    // private word. This one does not require the two of them to have spoken
    // privately since 2007.
    for stale in [200i64, 300, 1_000, 3_000, 7_000] {
        let log = private_history_plus_one_group_ping(stale);
        let score = TieAlgo::T4D.score(1, &log, testing::AS_OF_2026_08_24);
        assert_eq!(score.silent_days, 1, "{stale}");
        assert_eq!(
            age_days(score.last_direct_contact_unix, score.as_of_unix),
            stale
        );
        assert_eq!(score.direct_count(), 12);
        assert_eq!(score.group_count(), 1);
        assert_eq!(score.group_in_count, 1, "the ping is incoming only");
        assert_eq!(score.band, Band::Strong, "private history {stale} days old");
    }

    // Without the single group row the ladder falls, which locates the whole
    // effect in that one message.
    let mut without = private_history_plus_one_group_ping(7_000);
    without.pop();
    assert_eq!(
        TieAlgo::T4D
            .score(1, &without, testing::AS_OF_2026_08_24)
            .band,
        Band::Weak
    );
}

#[test]
fn a_split_clock_names_when_the_private_conversation_ended() {
    let f = testing::dormant_direct_group_ping_yesterday();
    let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
    let text = explain_zh(&score);

    assert_eq!(score.band, Band::Strong);
    assert_eq!(
        age_days(score.last_direct_contact_unix, score.as_of_unix),
        300
    );
    assert!(text.contains("一对一 12 次"), "{text}");
    assert!(text.contains("距今 1 天"), "{text}");
    assert!(
        text.contains(&zh_date(score.last_direct_contact_unix)),
        "private date must appear: {text}"
    );
}

#[test]
fn a_band_decided_on_private_rows_says_when_those_rows_happened() {
    // The C3 requirement, applied to the number the band was actually decided
    // on. Wherever the two clocks disagree, the split-count sentence has to
    // carry the one-to-one date as well as the any-venue one — the score
    // already holds it in `last_direct_contact_unix`, so this is a template
    // change, not a rule change, and it does not touch a band.
    let mut offenders = Vec::new();
    for f in testing::all() {
        let score = TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of);
        if score.is_empty() || !score.any_direct() {
            continue;
        }
        if score.last_direct_contact_unix == score.last_contact_unix {
            continue;
        }
        let text = explain_zh(&score);
        if !text.contains(&zh_date(score.last_direct_contact_unix)) {
            offenders.push(f.name);
        }
    }
    assert_eq!(offenders, Vec::<&str>::new());
}

// ---------------------------------------------------------------------------
// 4. What re-verification did not break
// ---------------------------------------------------------------------------

#[test]
fn the_silence_edges_are_exact_when_the_newest_row_is_a_group_row() {
    // The intersection of the two things Round 3 changed, which neither
    // `recency.rs` nor `ablation.rs` crosses: the 180/360 edges are unit-tested
    // on the band ladder alone, and the fixtures that sit on them are all
    // private-only. Here the counts come from a private history far past the
    // cut-off and the clock is a single group row, so an off-by-one in either
    // half would show up as a band moving one day early or late.
    for (group_age, expected) in [
        (0i64, Band::Strong),
        (179, Band::Strong),
        (180, Band::Moderate),
        (359, Band::Moderate),
        (360, Band::Weak),
    ] {
        let mut log = Vec::new();
        for day in 0..6i64 {
            log.push(direct(1, true, testing::at(700 + day, 9), 1));
            log.push(direct(1, false, testing::at(700 + day, 10), 1));
        }
        log.push(group(1, false, testing::at(group_age, 9), 2));
        let score = TieAlgo::T4D.score(1, &log, testing::AS_OF_2026_08_24);
        assert_eq!(score.silent_days, group_age);
        assert_eq!(score.band, expected, "group row {group_age} days old");
    }
}

#[test]
fn the_ladder_and_the_ordering_survive_evidence_that_straddles_the_epoch() {
    // `tests/direct_gate.rs` generates every timestamp from `testing::at`,
    // which is anchored in 2026, so its four property tests never see a
    // negative Unix second. Imported archives with broken clocks do. Same three
    // claims, re-run on evidence spread from 1969 to 1970.
    let mut state = 0xE90C_1970u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 11
    };
    for case in 0..4_000 {
        let rows = next() % 30;
        let log: Vec<Interaction> = (0..rows)
            .map(|_| Interaction {
                peer_id: 1,
                outgoing: next() % 2 == 0,
                occurred_at_unix: (next() % 40) as i64 * DAY - 20 * DAY + (next() % 86_400) as i64,
                venue_direct: next() % 3 == 0,
                conversation_id: next() % 3,
            })
            .collect();
        for as_of in [-20 * DAY, 0, 20 * DAY, 400 * DAY] {
            let t4 = TieAlgo::T4.score(1, &log, as_of);
            let t4d = TieAlgo::T4D.score(1, &log, as_of);
            assert!(
                t4d.band.rank() <= t4.band.rank(),
                "case {case} at {as_of}: T4D {:?} over T4 {:?}: {log:?}",
                t4d.band,
                t4.band
            );
            assert!(t4d.silent_days >= 0, "case {case}: {log:?}");
            assert!(
                t4d.direct_active_day_count <= t4d.active_day_count,
                "case {case}: {log:?}"
            );
            if t4d.band == Band::Strong {
                assert!(t4d.direct_count() >= STRONG_MIN_INTERACTIONS, "{log:?}");
                assert!(
                    t4d.direct_active_day_count >= STRONG_MIN_ACTIVE_DAYS,
                    "{log:?}"
                );
                assert!(t4d.is_direct_reciprocal(), "{log:?}");
            }
        }
    }
}

#[test]
fn group_rows_still_cannot_reach_the_counts_stage_at_any_volume_or_shape() {
    // The latch, re-derived rather than re-read. Every combination of a small
    // private history with a large group one: the counts band is a function of
    // the private rows alone, whatever the group rows do.
    for direct_out in 0..5u64 {
        for direct_in in 0..5u64 {
            for group_rows in [0u64, 1, 200] {
                let mut log = Vec::new();
                for i in 0..direct_out {
                    log.push(direct(1, true, testing::at(i as i64 % 4, 9), 1));
                }
                for i in 0..direct_in {
                    log.push(direct(1, false, testing::at(i as i64 % 4, 10), 1));
                }
                let private_only = Tally::of(1, &log);
                for i in 0..group_rows {
                    log.push(group(1, i % 2 == 0, testing::at(i as i64 % 300, 11), 2));
                }
                assert_eq!(
                    T4D::counts_band(&Tally::of(1, &log)),
                    T4D::counts_band(&private_only),
                    "{direct_out} out / {direct_in} in / {group_rows} group"
                );
            }
        }
    }
}
