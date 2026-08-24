//! The three surviving tie-strength candidates, on one set of evidence.
//!
//! Round 2, slot `opus-a`. R1-SYNTHESIS killed T2, A3 and T1-as-a-product, and
//! retired T0 as a specification while keeping it as the thing every port must
//! still agree with. What is left to decide is which of **T3**, **T3R** and
//! **T4** becomes the single people-graph algorithm, and that is decided by
//! the ablation in `tests/ablation.rs`, not by prose.
//!
//! ```
//! use soul_algo_tie::{Band, Interaction, TieAlgo, TieAlgorithm, T3, T4};
//!
//! // Twelve reciprocal one-to-one exchanges over six days — and then silence.
//! let log: Vec<Interaction> = (0..12)
//!     .map(|i| Interaction {
//!         peer_id: 7,
//!         outgoing: i % 2 == 0,
//!         occurred_at_unix: 86_400 * (i / 2),
//!         venue_direct: true,
//!         conversation_id: 1,
//!     })
//!     .collect();
//!
//! // Judged the week it happened, both rules say the same thing.
//! let as_of = 86_400 * 12;
//! assert_eq!(T3::score(7, &log, as_of).band, Band::Strong);
//! assert_eq!(T4::score(7, &log, as_of).band, Band::Strong);
//!
//! // Judged two years later, only the rule that can see time passing moves.
//! let as_of = 86_400 * 730;
//! assert_eq!(T3::score(7, &log, as_of).band, Band::Strong);
//! assert_eq!(T4::score(7, &log, as_of).band, Band::Weak);
//! assert_eq!(TieAlgo::T4.score(7, &log, as_of).silent_days, 725);
//! ```
//!
//! ## What every candidate has in common
//!
//! * **Reciprocity is a gate, not a term.** No candidate can call a one-sided
//!   tie anything but Weak. A newsletter is not a friend.
//! * **A one-to-one channel is required for Strong.** A tie only ever seen in
//!   a group chat tops out at Moderate, however loud it is. This is the fix
//!   for R1-SYNTHESIS P0-3.
//! * **The output is a band plus the raw counts behind it.** No scores, no
//!   percentiles, nothing a user cannot recount by hand.
//! * **Content is never read.** The input carries a peer id, a direction, a
//!   timestamp, a venue flag and a conversation id.
//! * **Time is UTC and comes from the data.** Every entry point takes
//!   `as_of_unix`; nothing here calls `SystemTime`, so the same evidence
//!   scores the same in Shanghai, in CI, and tomorrow.
//! * **Integers only.** There is no floating point anywhere in the product
//!   path — T3R's decay is done in quarter-interaction units. Pinned by
//!   `tests/ablation.rs::the_product_path_contains_no_floating_point`.
//! * **Orphanable.** A score is derived from a set of interactions and holds
//!   no state; forget the evidence and re-run, and the tie is gone or weaker.

#![forbid(unsafe_code)]

pub mod constants;
pub mod gate;
pub mod t0;
pub mod t3;
pub mod t3r;
pub mod t4;
pub mod testing;
#[cfg(test)]
mod tombstones;
pub mod types;

pub use t0::T0;
pub use t3::T3;
pub use t3r::T3R;
pub use t4::T4;
pub use types::{
    age_days, as_of_max, civil_from_epoch_day, epoch_day, tally_ego_network, zh_date, Band,
    Decayed, Detail, Interaction, Tally, TieAlgorithm, TieScore, SECONDS_PER_DAY,
};

/// Runtime choice of rule, for callers that want to compare them.
///
/// The trait is the compile-time form; this is the same rules behind one
/// value, so the ablation table does not need a generic parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieAlgo {
    /// The shipped rule. Kept as the regression oracle, **not** a candidate:
    /// R1-SYNTHESIS retired it as a specification.
    T0,
    /// Granovetter gate over raw counts.
    T3,
    /// Granovetter gate over bucketed, decayed counts.
    T3R,
    /// Granovetter gate over raw counts, then one integer step down for
    /// silence.
    T4,
}

impl TieAlgo {
    /// Everything this crate can run, in table order.
    pub const ALL: [TieAlgo; 4] = [TieAlgo::T0, TieAlgo::T3, TieAlgo::T3R, TieAlgo::T4];

    /// The rules still competing for the single people-graph slot.
    pub const CANDIDATES: [TieAlgo; 3] = [TieAlgo::T3, TieAlgo::T3R, TieAlgo::T4];

    pub const fn id(self) -> &'static str {
        match self {
            TieAlgo::T0 => T0::ID,
            TieAlgo::T3 => T3::ID,
            TieAlgo::T3R => T3R::ID,
            TieAlgo::T4 => T4::ID,
        }
    }

    /// True when the rule looks at `as_of` to decide a band. T0 and T3 do not,
    /// which is exactly what the other two were proposed to fix.
    pub const fn uses_recency(self) -> bool {
        match self {
            TieAlgo::T0 | TieAlgo::T3 => false,
            TieAlgo::T3R | TieAlgo::T4 => true,
        }
    }

    /// True when the rule distinguishes a one-to-one exchange from a group
    /// one. Only T0 does not.
    pub const fn uses_venue(self) -> bool {
        !matches!(self, TieAlgo::T0)
    }

    fn band_and_detail(self, tally: &Tally, as_of_unix: i64) -> (Band, Detail) {
        match self {
            TieAlgo::T0 => (T0::band_of(tally, as_of_unix), Detail::RawCounts),
            TieAlgo::T3 => (T3::band_of(tally, as_of_unix), Detail::RawCounts),
            TieAlgo::T3R => {
                let (band, decayed) = T3R::band_of(tally, as_of_unix);
                (band, Detail::Decayed(decayed))
            }
            TieAlgo::T4 => {
                let (band, band_before) = T4::band_of(tally, as_of_unix);
                (band, Detail::Demoted { band_before })
            }
        }
    }

    pub fn score(self, peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let (band, detail) = self.band_and_detail(&tally, as_of_unix);
        tally.to_score(band, self.id(), as_of_unix, detail)
    }

    pub fn explain_zh(self, score: &TieScore) -> String {
        match self {
            TieAlgo::T0 => T0::explain_zh(score),
            TieAlgo::T3 => T3::explain_zh(score),
            TieAlgo::T3R => T3R::explain_zh(score),
            TieAlgo::T4 => T4::explain_zh(score),
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

/// One peer under every rule, in [`TieAlgo::ALL`] order.
///
/// The comparison helper the round is for: same evidence, same `as_of`, four
/// verdicts side by side, one pass over the log.
pub fn score_all(peer_id: u64, interactions: &[Interaction], as_of_unix: i64) -> [TieScore; 4] {
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
    fn enum_dispatch_agrees_with_the_trait() {
        let f = lilei_12();
        assert_eq!(
            TieAlgo::T0.score(f.peer_id, &f.log, f.as_of),
            T0::score(f.peer_id, &f.log, f.as_of)
        );
        assert_eq!(
            TieAlgo::T3.score(f.peer_id, &f.log, f.as_of),
            T3::score(f.peer_id, &f.log, f.as_of)
        );
        assert_eq!(
            TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of),
            T3R::score(f.peer_id, &f.log, f.as_of)
        );
        assert_eq!(
            TieAlgo::T4.score(f.peer_id, &f.log, f.as_of),
            T4::score(f.peer_id, &f.log, f.as_of)
        );
    }

    #[test]
    fn score_all_tags_each_result_with_its_own_rule() {
        let f = lilei_12();
        let scores = score_all(f.peer_id, &f.log, f.as_of);
        let ids: Vec<&str> = scores.iter().map(|score| score.algorithm_id).collect();
        assert_eq!(ids, vec!["T0", "T3", "T3R", "T4"]);
        // The counts are a property of the evidence, not of the rule, so all
        // of them have to agree whatever they decide about the band.
        for score in &scores {
            assert_eq!(score.interaction_count, 12);
            assert_eq!(score.active_day_count, 6);
            assert!(score.any_direct);
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
            assert_eq!(score.silent_days, 0);
            assert!(!score.any_direct);
        }
    }

    #[test]
    fn every_score_carries_the_venue_and_the_rule_that_made_it() {
        // The Round 2 addition to TieScore, asserted as a property rather than
        // as a field that happens to exist.
        for fixture in testing::all() {
            for algo in TieAlgo::ALL {
                let score = algo.score(fixture.peer_id, &fixture.log, fixture.as_of);
                assert_eq!(score.algorithm_id, algo.id());
                assert_eq!(
                    score.any_direct,
                    fixture
                        .log
                        .iter()
                        .any(|row| row.peer_id == fixture.peer_id && row.venue_direct),
                    "{}",
                    fixture.name
                );
            }
        }
    }
}
