//! The Soul people-graph tie-strength rule, and the rule it replaces.
//!
//! Round 3, slot `opus-a`. R2-SYNTHESIS kept exactly one candidate for the
//! people graph — T4, the Granovetter gate plus an integer demotion for
//! silence — and left one defect open against it: `any_direct` is a boolean,
//! so a group fan-out plus one private hello each way still clears every gate
//! (§潜在边界风险 2). This crate ships the answer to that, [`T4D`], as the
//! default, and keeps [`T4`] beside it so the decision can be reverted in one
//! line rather than reconstructed.
//!
//! ```
//! use soul_algo_tie::{score, Band, Interaction, TieAlgo};
//!
//! // A project-channel colleague: twenty group messages over ten days, both
//! // directions, plus exactly one private hello each way.
//! let mut log: Vec<Interaction> = (0..20)
//!     .map(|i| Interaction {
//!         peer_id: 7,
//!         outgoing: i % 2 == 0,
//!         occurred_at_unix: 86_400 * (i / 2),
//!         venue_direct: false,
//!         conversation_id: 1,
//!     })
//!     .collect();
//! log.push(Interaction { peer_id: 7, outgoing: true, occurred_at_unix: 86_400 * 10, venue_direct: true, conversation_id: 2 });
//! log.push(Interaction { peer_id: 7, outgoing: false, occurred_at_unix: 86_400 * 10 + 60, venue_direct: true, conversation_id: 2 });
//!
//! let as_of = 86_400 * 11;
//!
//! // The rollback rule counts every venue, so twenty-two exchanges over
//! // eleven days with somebody you did once message privately is a close tie.
//! assert_eq!(TieAlgo::T4.score(7, &log, as_of).band, Band::Strong);
//!
//! // The default rule counts the two private exchanges and nothing else.
//! let tie = score(7, &log, as_of);
//! assert_eq!(tie.band, Band::Weak);
//! assert_eq!(tie.direct_count(), 2);
//! assert_eq!(tie.group_count(), 20);
//! assert_eq!(tie.algorithm_id, "T4D");
//! ```
//!
//! ## What both rules hold to
//!
//! * **Reciprocity is a gate, not a term.** Neither rule can call a one-sided
//!   tie anything but Weak. A newsletter is not a friend.
//! * **A one-to-one channel is required for Strong.** T4 requires that one
//!   existed; T4D requires that the traffic went through it.
//! * **The output is a band plus the raw counts behind it.** No scores, no
//!   percentiles, nothing a user cannot recount by hand — and from Round 3 the
//!   counts are split by venue, so the user can recount the number the band
//!   was actually decided on.
//! * **Content is never read.** The input carries a peer id, a direction, a
//!   timestamp, a venue flag and a conversation id.
//! * **Time comes from the caller.** Every entry point takes `as_of_unix`;
//!   nothing here calls `SystemTime`, so the same evidence scores the same in
//!   Shanghai, in CI, and tomorrow. R2-SYNTHESIS freezes one `as_of` per
//!   rebuild for the whole store.
//! * **Integers only.** No floating point anywhere in the product path, pinned
//!   by `tests/ablation.rs::the_product_path_contains_no_floating_point`.
//! * **Orphanable.** A score is derived from a set of interactions and holds
//!   no state; forget the evidence and re-run, and the tie is gone or weaker.
//!
//! ## What is not here
//!
//! T0 — the rule Goal 1 ships — is in [`testing::oracle`] and is not a
//! [`TieAlgo`] variant. R1-SYNTHESIS retired it as a specification and forbade
//! falling back to it; it survives only so `tests/goal1_fidelity.rs` can prove
//! the shared tally still computes what the shipped rule computed. T1, T2, T3
//! and T3R are `#[cfg(test)]` tombstones in `src/tombstones.rs`, each with the
//! fixture that killed it.

#![forbid(unsafe_code)]

pub mod constants;
pub mod gate;
pub mod recency;
pub mod t4;
pub mod t4d;
pub mod testing;
#[cfg(test)]
mod tombstones;
pub mod types;

pub use t4::T4;
pub use t4d::T4D;
pub use types::{
    age_days, as_of_max, civil_from_epoch_day, epoch_day, tally_ego_network, zh_date, Band, Detail,
    Interaction, Tally, TieAlgorithm, TieScore, SECONDS_PER_DAY,
};

/// Which rule to run.
///
/// Two variants, not four: Round 3 is a choice between the rule R2-SYNTHESIS
/// kept and the one that fixes its open defect. [`TieAlgo::T4D`] is the
/// default and is what [`score`] runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TieAlgo {
    /// Counts every venue, requires that a one-to-one channel existed. The
    /// Round 2 winner, kept for rollback.
    T4,
    /// Counts only the one-to-one rows. The default.
    #[default]
    T4D,
}

impl TieAlgo {
    /// Both rules, in table order.
    pub const ALL: [TieAlgo; 2] = [TieAlgo::T4, TieAlgo::T4D];

    /// What the product runs when nobody asked for anything else.
    pub const DEFAULT: TieAlgo = TieAlgo::T4D;

    pub const fn id(self) -> &'static str {
        match self {
            TieAlgo::T4 => T4::ID,
            TieAlgo::T4D => T4D::ID,
        }
    }

    /// The rule with this identifier, for reading a stored band back.
    pub fn from_id(id: &str) -> Option<TieAlgo> {
        TieAlgo::ALL.into_iter().find(|algo| algo.id() == id)
    }

    /// True when group traffic can move the band. Only T4's can.
    pub const fn counts_group_traffic(self) -> bool {
        matches!(self, TieAlgo::T4)
    }

    fn band_and_detail(self, tally: &Tally, as_of_unix: i64) -> (Band, Detail) {
        let (band, band_before) = match self {
            TieAlgo::T4 => T4::band_of(tally, as_of_unix),
            TieAlgo::T4D => T4D::band_of(tally, as_of_unix),
        };
        (band, Detail::Demoted { band_before })
    }

    pub fn score(self, peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let (band, detail) = self.band_and_detail(&tally, as_of_unix);
        tally.to_score(band, self.id(), as_of_unix, detail)
    }

    pub fn explain_zh(self, score: &TieScore) -> String {
        match self {
            TieAlgo::T4 => T4::explain_zh(score),
            TieAlgo::T4D => T4D::explain_zh(score),
        }
    }

    /// Score every peer in the log in one pass.
    ///
    /// Returned sorted by peer id, so two runs over the same evidence produce
    /// byte-identical output. Linear in the log, not in log times peers.
    pub fn score_ego_network(
        self,
        interactions: &[Interaction],
        as_of_unix: i64,
    ) -> Vec<(u64, TieScore)> {
        tally_ego_network(interactions)
            .into_iter()
            .map(|(peer_id, tally)| {
                let (band, detail) = self.band_and_detail(&tally, as_of_unix);
                let score = tally.to_score(band, self.id(), as_of_unix, detail);
                (peer_id, score)
            })
            .collect()
    }
}

/// Score one peer with the default rule.
///
/// This is the entry point the product calls. `as_of_unix` is supplied by the
/// caller and is one value for the whole rebuild.
pub fn score(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
    TieAlgo::DEFAULT.score(peer_id, interactions, as_of_unix)
}

/// The name Goal 1 uses: the product denylist forbids the word `score`
/// outside this crate, so the graph rebuild calls this and never names
/// [`score`] or [`TieScore`].
pub fn assess_tie(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
    score(peer_id, interactions, as_of_unix)
}

/// Score every peer in the log with the default rule, in one pass.
pub fn score_ego_network(interactions: &[Interaction], as_of_unix: i64) -> Vec<(u64, TieScore)> {
    TieAlgo::DEFAULT.score_ego_network(interactions, as_of_unix)
}

/// Explain a band in the language the user reads.
///
/// Dispatches on the rule that produced the score, so a stored T4 band is
/// never described in T4D's words. Anything this crate does not recognise —
/// the test-only oracle, or a band from an older schema — is explained by the
/// default rule, which is the only rule the product ships.
pub fn explain_zh(score: &TieScore) -> String {
    TieAlgo::from_id(score.algorithm_id)
        .unwrap_or(TieAlgo::DEFAULT)
        .explain_zh(score)
}

/// One peer under both rules, in [`TieAlgo::ALL`] order, from one pass over
/// the evidence.
pub fn score_both(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> [TieScore; 2] {
    let tally = Tally::of(peer_id, interactions);
    TieAlgo::ALL.map(|algo| {
        let (band, detail) = algo.band_and_detail(&tally, as_of_unix);
        tally.to_score(band, algo.id(), as_of_unix, detail)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, lilei_12, DAY};

    #[test]
    fn the_default_rule_is_t4d() {
        assert_eq!(TieAlgo::default(), TieAlgo::T4D);
        assert_eq!(TieAlgo::DEFAULT, TieAlgo::T4D);
        assert_eq!(TieAlgo::DEFAULT.id(), "T4D");
        let f = lilei_12();
        assert_eq!(
            score(f.peer_id, &f.log, f.as_of),
            TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of)
        );
        assert_eq!(
            assess_tie(f.peer_id, &f.log, f.as_of),
            score(f.peer_id, &f.log, f.as_of)
        );
        assert_eq!(
            score_ego_network(&f.log, f.as_of),
            TieAlgo::T4D.score_ego_network(&f.log, f.as_of)
        );
    }

    #[test]
    fn enum_dispatch_agrees_with_the_trait() {
        let f = lilei_12();
        assert_eq!(
            TieAlgo::T4.score(f.peer_id, &f.log, f.as_of),
            T4::score(f.peer_id, &f.log, f.as_of)
        );
        assert_eq!(
            TieAlgo::T4D.score(f.peer_id, &f.log, f.as_of),
            T4D::score(f.peer_id, &f.log, f.as_of)
        );
    }

    #[test]
    fn the_free_explanation_follows_the_rule_that_made_the_score() {
        for f in testing::all() {
            for algo in TieAlgo::ALL {
                let s = algo.score(f.peer_id, &f.log, f.as_of);
                assert_eq!(explain_zh(&s), algo.explain_zh(&s), "{}", f.name);
            }
        }
    }

    #[test]
    fn identifiers_round_trip() {
        for algo in TieAlgo::ALL {
            assert_eq!(TieAlgo::from_id(algo.id()), Some(algo));
        }
        assert_eq!(TieAlgo::from_id("T0"), None);
        assert_eq!(TieAlgo::from_id("T3R"), None);
    }

    #[test]
    fn score_both_tags_each_result_with_its_own_rule() {
        let f = lilei_12();
        let scores = score_both(f.peer_id, &f.log, f.as_of);
        let ids: Vec<&str> = scores.iter().map(|score| score.algorithm_id).collect();
        assert_eq!(ids, vec!["T4", "T4D"]);
        // The counts are a property of the evidence, not of the rule, so both
        // of them have to agree whatever they decide about the band.
        for score in &scores {
            assert_eq!(score.interaction_count, 12);
            assert_eq!(score.direct_count(), 12);
            assert_eq!(score.group_count(), 0);
            assert_eq!(score.active_day_count, 6);
            assert_eq!(score.direct_active_day_count, 6);
            assert_eq!(score.silent_days, 3);
        }
    }

    #[test]
    fn ego_network_scoring_matches_per_peer_scoring() {
        let mut log = Vec::new();
        for peer in 1..=5u64 {
            for i in 0..(peer as i64 * 3) {
                log.push(direct(peer, i % 2 == 0, DAY * i, peer));
            }
        }
        let as_of = DAY * 20;
        for algo in TieAlgo::ALL {
            let whole = algo.score_ego_network(&log, as_of);
            assert_eq!(whole.len(), 5);
            for (peer_id, score) in whole {
                assert_eq!(score, algo.score(peer_id, &log, as_of));
            }
        }
    }

    #[test]
    fn unknown_peers_score_as_nothing_observed() {
        let f = lilei_12();
        for algo in TieAlgo::ALL {
            let score = algo.score(999, &f.log, f.as_of);
            assert_eq!(score.band, Band::Weak);
            assert_eq!(score.interaction_count, 0);
            assert_eq!(score.first_contact_unix, 0);
            assert_eq!(score.last_contact_unix, 0);
            assert_eq!(score.last_direct_contact_unix, 0);
            assert_eq!(score.silent_days, 0);
            assert!(!score.any_direct());
        }
    }

    #[test]
    fn every_score_carries_the_split_counts_and_the_rule_that_made_it() {
        for fixture in testing::all() {
            let rows: Vec<_> = fixture
                .log
                .iter()
                .filter(|row| row.peer_id == fixture.peer_id)
                .collect();
            for algo in TieAlgo::ALL {
                let score = algo.score(fixture.peer_id, &fixture.log, fixture.as_of);
                assert_eq!(score.algorithm_id, algo.id());
                assert_eq!(
                    score.direct_count(),
                    rows.iter().filter(|row| row.venue_direct).count() as u64,
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    score.group_count(),
                    rows.iter().filter(|row| !row.venue_direct).count() as u64,
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    score.interaction_count,
                    score.direct_count() + score.group_count(),
                    "{}",
                    fixture.name
                );
                assert_eq!(
                    score.outgoing_count,
                    score.direct_out_count + score.group_out_count
                );
                assert_eq!(
                    score.incoming_count,
                    score.direct_in_count + score.group_in_count
                );
            }
        }
    }
}
