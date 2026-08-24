//! A2: what the counts allow us to say about another person, and everything
//! they do not.

use soul_algo_trait::a2::{a2_personnel_summary, PeerStats};
use soul_algo_trait::denylist::{assert_publishable_about_peer, peer_claim_hit};
use soul_algo_trait::fixtures::{peer_stats_matrix, FIXTURE_NOW_UNIX};
use soul_algo_trait::Band;

const DAY: i64 = 86_400;
const NOW: i64 = FIXTURE_NOW_UNIX;

fn stats_named(name: &str) -> PeerStats {
    peer_stats_matrix()
        .into_iter()
        .find(|(fixture, _)| *fixture == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

fn keys(summary: &soul_algo_trait::PersonnelSummary) -> Vec<String> {
    summary
        .bullets
        .iter()
        .map(|bullet| bullet.statement_key.clone())
        .collect()
}

/// The documented empty case: no bullets at all, because a bullet that cites
/// nothing would be the first claim in this crate without evidence. Saying
/// 「还看不出」 is the caller's copy, not an uncited bullet.
#[test]
fn empty_stats_produce_no_bullets() {
    let summary = a2_personnel_summary(&PeerStats::default(), NOW);

    assert!(summary.is_empty());
    assert!(summary.bullets.is_empty());
}

#[test]
fn counts_without_citable_evidence_produce_no_bullets() {
    let summary = a2_personnel_summary(&stats_named("counts_without_citable_evidence"), NOW);

    assert!(
        summary.is_empty(),
        "a bullet must cite evidence, so no evidence means no bullet"
    );
}

#[test]
fn every_bullet_cites_evidence() {
    for (name, stats) in peer_stats_matrix() {
        for bullet in a2_personnel_summary(&stats, NOW).bullets {
            assert!(
                !bullet.evidence_ids.is_empty(),
                "{name}/{} cited nothing",
                bullet.statement_key
            );
        }
    }
}

/// The group-only case. The summary says where the contact happened and says
/// nothing about one-to-one contact that was never observed.
#[test]
fn a_group_only_peer_is_never_described_as_a_direct_contact() {
    let summary = a2_personnel_summary(&stats_named("group_only_reciprocal"), NOW);

    assert!(keys(&summary).contains(&"personnel.venue.group_only".to_owned()));
    for text in summary.texts() {
        assert!(
            !text.contains("私聊"),
            "group-only summary said 私聊: {text}"
        );
        assert!(
            !text.contains("单聊"),
            "group-only summary said 单聊: {text}"
        );
        assert!(
            !text.contains("私下"),
            "group-only summary said 私下: {text}"
        );
    }
}

#[test]
fn no_group_only_bullet_when_direct_contact_was_seen() {
    let summary = a2_personnel_summary(&stats_named("direct_reciprocal_thick"), NOW);

    assert!(!keys(&summary).contains(&"personnel.venue.group_only".to_owned()));
}

#[test]
fn reciprocity_is_reported_with_both_counts() {
    let summary = a2_personnel_summary(&stats_named("direct_reciprocal_thick"), NOW);
    let bullet = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.tie.reciprocal")
        .expect("a reciprocal edge gets a reciprocity bullet");

    assert!(bullet.text_zh.contains("130"));
    assert!(bullet.text_zh.contains("110"));
    assert_eq!(bullet.band, Band::Strong);
}

/// Silence is thin evidence. A one-way record is never called strong.
#[test]
fn a_one_way_edge_is_never_strong() {
    for name in ["single_outgoing_message", "one_way_incoming_only"] {
        let summary = a2_personnel_summary(&stats_named(name), NOW);
        for bullet in &summary.bullets {
            assert_ne!(bullet.band, Band::Strong, "{name}/{}", bullet.statement_key);
        }
    }
}

#[test]
fn a_one_way_edge_says_which_way_it_goes() {
    let outgoing = a2_personnel_summary(&stats_named("single_outgoing_message"), NOW);
    assert!(keys(&outgoing).contains(&"personnel.tie.one_way_outgoing".to_owned()));

    let incoming = a2_personnel_summary(&stats_named("one_way_incoming_only"), NOW);
    assert!(keys(&incoming).contains(&"personnel.tie.one_way_incoming".to_owned()));
}

#[test]
fn two_way_traffic_the_graph_has_not_confirmed_is_reported_as_such() {
    let summary = a2_personnel_summary(&stats_named("two_way_but_not_confirmed_reciprocal"), NOW);

    assert!(keys(&summary).contains(&"personnel.tie.two_way_unconfirmed".to_owned()));
    assert!(!keys(&summary).contains(&"personnel.tie.reciprocal".to_owned()));
}

#[test]
fn days_since_last_contact_are_counted_from_the_given_now() {
    let summary = a2_personnel_summary(&stats_named("one_way_incoming_only"), NOW);
    let bullet = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.recency.days_since_last")
        .expect("a known last contact gets a recency bullet");

    assert!(bullet.text_zh.contains("11"), "{}", bullet.text_zh);
    assert_eq!(bullet.evidence_ids, vec![203]);
}

#[test]
fn contact_today_is_said_in_words_rather_than_as_zero_days() {
    let summary = a2_personnel_summary(&stats_named("direct_reciprocal_thick"), NOW);
    let bullet = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.recency.days_since_last")
        .expect("a known last contact gets a recency bullet");

    assert_eq!(bullet.text_zh, "最近一次往来就在今天。");
}

/// A timestamp ahead of `now` is a clock disagreement, not contact in the
/// future, and it must not produce a negative day count.
#[test]
fn a_last_contact_ahead_of_now_reads_as_today() {
    let summary = a2_personnel_summary(&stats_named("clock_ahead_of_last_contact"), NOW);
    let bullet = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.recency.days_since_last")
        .expect("a known last contact gets a recency bullet");

    assert_eq!(bullet.text_zh, "最近一次往来就在今天。");
    assert!(!bullet.text_zh.contains('-'));
}

#[test]
fn no_recency_bullet_without_a_last_contact() {
    let summary = a2_personnel_summary(&stats_named("two_way_but_not_confirmed_reciprocal"), NOW);

    assert!(!keys(&summary).contains(&"personnel.recency.days_since_last".to_owned()));
    assert!(!summary.is_empty(), "the other bullets still stand");
}

/// A thousand messages in one day is still one day, and the recency bullet
/// never claims more than the timestamp it rests on.
#[test]
fn a_single_day_burst_does_not_inflate_the_recency_bullet() {
    let summary = a2_personnel_summary(&stats_named("single_day_burst"), NOW);

    for bullet in &summary.bullets {
        if bullet.statement_key == "personnel.recency.days_since_last" {
            assert!(bullet.band.at_least(Band::Weak));
            assert_ne!(bullet.band, Band::Strong);
        }
    }

    let activity = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.activity.counts")
        .expect("counts are always reported");
    assert!(activity.text_zh.contains("1000"));
    assert!(activity.text_zh.contains('1'));
}

#[test]
fn a_long_cold_edge_reports_the_gap_without_judging_it() {
    let summary = a2_personnel_summary(&stats_named("long_cold_edge"), NOW);
    let bullet = summary
        .bullets
        .iter()
        .find(|bullet| bullet.statement_key == "personnel.recency.days_since_last")
        .expect("a known last contact gets a recency bullet");

    assert_eq!(bullet.text_zh, "最近一次往来距今 400 天。");
    assert_eq!(peer_claim_hit(&bullet.text_zh), None);
}

/// The order is fixed so a summary reads the same way twice, and so a
/// screenshot in a bug report means something.
#[test]
fn bullets_come_back_in_a_fixed_order() {
    let summary = a2_personnel_summary(&stats_named("group_only_reciprocal"), NOW);

    assert_eq!(
        keys(&summary),
        vec![
            "personnel.tie.reciprocal".to_owned(),
            "personnel.activity.counts".to_owned(),
            "personnel.recency.days_since_last".to_owned(),
            "personnel.venue.group_only".to_owned(),
        ]
    );
}

#[test]
fn the_same_stats_always_produce_the_same_summary() {
    for (name, stats) in peer_stats_matrix() {
        let first = a2_personnel_summary(&stats, NOW);
        let second = a2_personnel_summary(&stats, NOW);
        assert_eq!(first, second, "{name} is not deterministic");
    }
}

/// A2 never says who the peer is to the owner, and never says who the peer is.
#[test]
fn no_bullet_claims_friendship_intimacy_or_personality() {
    for (name, stats) in peer_stats_matrix() {
        for offset in [0, 3 * DAY, 400 * DAY] {
            for bullet in a2_personnel_summary(&stats, NOW + offset).bullets {
                assert_eq!(
                    assert_publishable_about_peer(&bullet.text_zh),
                    Ok(()),
                    "{name}/{} said something it may not: {}",
                    bullet.statement_key,
                    bullet.text_zh
                );
            }
        }
    }
}

/// Moving `now` forward changes the day count and nothing else about what is
/// claimed.
#[test]
fn moving_now_forward_only_changes_the_recency_bullet() {
    let stats = stats_named("long_cold_edge");
    let today = a2_personnel_summary(&stats, NOW);
    let later = a2_personnel_summary(&stats, NOW + 30 * DAY);

    assert_eq!(keys(&today), keys(&later));
    let recency = |summary: &soul_algo_trait::PersonnelSummary| {
        summary
            .bullets
            .iter()
            .find(|bullet| bullet.statement_key == "personnel.recency.days_since_last")
            .map(|bullet| bullet.text_zh.clone())
            .expect("a known last contact gets a recency bullet")
    };
    assert_eq!(recency(&today), "最近一次往来距今 400 天。");
    assert_eq!(recency(&later), "最近一次往来距今 430 天。");
}
