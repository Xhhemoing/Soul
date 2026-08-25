//! Four candidate rules for how strong a tie in the Soul graph is.
//!
//! Round 1, slot `opus-a`. This crate exists so the candidates can be compared
//! on the same fixtures instead of on prose: T0 is the rule Goal 1 already
//! ships, ported faithfully, and T1/T2/T3 are the three alternatives named in
//! the shared brief. Nothing here writes anything, asks anything of a model,
//! or reads a message body.
//!
//! ```
//! use soul_algo_tie::{Band, TieAlgo, TieAlgorithm, Interaction, T0};
//!
//! let log = vec![
//!     Interaction { peer_id: 7, outgoing: true,  occurred_at_unix: 0,         venue_direct: true, conversation_id: 1 },
//!     Interaction { peer_id: 7, outgoing: false, occurred_at_unix: 86_400,    venue_direct: true, conversation_id: 1 },
//!     Interaction { peer_id: 7, outgoing: true,  occurred_at_unix: 86_400 * 2, venue_direct: true, conversation_id: 1 },
//! ];
//! let score = T0::score(7, &log, 86_400 * 3);
//! assert_eq!(score.band, Band::Moderate);
//! assert_eq!(score.interaction_count, 3);
//! assert_eq!(TieAlgo::T0.score(7, &log, 0), score);
//! ```
//!
//! ## What every candidate has in common
//!
//! * **Reciprocity is a gate, not a term.** No candidate can call a one-sided
//!   tie anything but Weak. A newsletter is not a friend.
//! * **The output is a band plus the raw counts behind it.** No scores, no
//!   percentiles, nothing a user cannot recount by hand — `DECISIONS D22` and
//!   the PRODUCT_LOCK rule that inferences must be checkable.
//! * **Content is never read.** The input carries a peer id, a direction, a
//!   timestamp, a venue flag and a conversation id. No text, so third-party
//!   message bodies cannot leak through a tie strength.
//! * **Time is UTC only.** Active days are epoch days, so the same evidence
//!   scores the same in Shanghai and in CI.
//! * **Pure functions.** Same evidence and same `now` give the same score, in
//!   any input order, with no clock read inside the crate.
//! * **Orphanable.** A score is derived from a set of interactions and holds
//!   no state; forget the evidence and re-run, and the tie is gone or weaker.
//!   Nothing has to be migrated.

#![forbid(unsafe_code)]

pub mod t0;
pub mod t1;
pub mod t2;
pub mod t3;
pub mod testing;
pub mod types;

pub use t0::T0;
pub use t1::T1;
pub use t2::T2;
pub use t3::T3;
pub use types::{
    age_days_exact, age_days_floor, civil_from_epoch_day, epoch_day, tally_ego_network, zh_date,
    Band, Interaction, Tally, TieAlgorithm, TieScore, SECONDS_PER_DAY,
};

/// Runtime choice of candidate, for callers that want to compare them.
///
/// The trait is the compile-time form; this is the same four rules behind one
/// value, so a benchmark or a UI toggle does not need a generic parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieAlgo {
    /// Count + reciprocity + span. The shipped baseline.
    T0,
    /// Recency-weighted exponential decay.
    T1,
    /// RFM band.
    T2,
    /// Granovetter-span.
    T3,
}

impl TieAlgo {
    /// Every candidate, in comparison order.
    pub const ALL: [TieAlgo; 4] = [TieAlgo::T0, TieAlgo::T1, TieAlgo::T2, TieAlgo::T3];

    pub const fn id(self) -> &'static str {
        match self {
            TieAlgo::T0 => T0::ID,
            TieAlgo::T1 => T1::ID,
            TieAlgo::T2 => T2::ID,
            TieAlgo::T3 => T3::ID,
        }
    }

    /// True when the rule looks at `now_unix`. T0 does not, which is exactly
    /// what the recency candidates were proposed to fix.
    pub const fn uses_recency(self) -> bool {
        match self {
            TieAlgo::T0 | TieAlgo::T3 => false,
            TieAlgo::T1 | TieAlgo::T2 => true,
        }
    }

    /// True when the rule distinguishes a one-to-one exchange from a group
    /// one. T0 does not, which is what T3 was proposed to fix.
    pub const fn uses_venue(self) -> bool {
        match self {
            TieAlgo::T0 | TieAlgo::T2 => false,
            TieAlgo::T1 | TieAlgo::T3 => true,
        }
    }

    fn band_of(self, tally: &Tally, now_unix: i64) -> Band {
        match self {
            TieAlgo::T0 => T0::band_of(tally, now_unix),
            TieAlgo::T1 => T1::band_of(tally, now_unix),
            TieAlgo::T2 => T2::band_of(tally, now_unix),
            TieAlgo::T3 => T3::band_of(tally, now_unix),
        }
    }

    pub fn score(self, peer_id: u64, interactions: &[Interaction], now_unix: i64) -> TieScore {
        let tally = Tally::of(peer_id, interactions);
        let band = self.band_of(&tally, now_unix);
        tally.into_score(band, self.id())
    }

    pub fn explain_zh(self, score: &TieScore) -> String {
        match self {
            TieAlgo::T0 => T0::explain_zh(score),
            TieAlgo::T1 => T1::explain_zh(score),
            TieAlgo::T2 => T2::explain_zh(score),
            TieAlgo::T3 => T3::explain_zh(score),
        }
    }

    /// Score every peer in the log in one pass.
    ///
    /// Returned sorted by peer id, so two runs over the same evidence produce
    /// byte-identical output. Linear in the log, not in log times peers.
    pub fn score_ego_network(
        self,
        interactions: &[Interaction],
        now_unix: i64,
    ) -> Vec<(u64, TieScore)> {
        tally_ego_network(interactions)
            .into_iter()
            .map(|(peer_id, tally)| {
                let band = self.band_of(&tally, now_unix);
                (peer_id, tally.into_score(band, self.id()))
            })
            .collect()
    }
}

/// One peer under all four rules, in [`TieAlgo::ALL`] order.
///
/// The comparison helper the round is actually for: same evidence, same `now`,
/// four verdicts side by side.
pub fn score_all(peer_id: u64, interactions: &[Interaction], now_unix: i64) -> [TieScore; 4] {
    let tally = Tally::of(peer_id, interactions);
    TieAlgo::ALL.map(|algo| {
        let band = algo.band_of(&tally, now_unix);
        tally.clone().into_score(band, algo.id())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{direct, lilei_12_over_6_days, DAY};

    #[test]
    fn enum_dispatch_agrees_with_the_trait() {
        let (log, now) = lilei_12_over_6_days();
        assert_eq!(TieAlgo::T0.score(1, &log, now), T0::score(1, &log, now));
        assert_eq!(TieAlgo::T1.score(1, &log, now), T1::score(1, &log, now));
        assert_eq!(TieAlgo::T2.score(1, &log, now), T2::score(1, &log, now));
        assert_eq!(TieAlgo::T3.score(1, &log, now), T3::score(1, &log, now));
    }

    #[test]
    fn score_all_tags_each_result_with_its_own_rule() {
        let (log, now) = lilei_12_over_6_days();
        let scores = score_all(1, &log, now);
        let ids: Vec<&str> = scores.iter().map(|score| score.algorithm_id).collect();
        assert_eq!(ids, vec!["T0", "T1", "T2", "T3"]);
        // The counts are a property of the evidence, not of the rule, so all
        // four have to agree on them whatever they decide about the band.
        for score in &scores {
            assert_eq!(score.interaction_count, 12);
            assert_eq!(score.active_day_count, 6);
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
        let now = DAY * 20;
        for algo in TieAlgo::ALL {
            let whole = algo.score_ego_network(&log, now);
            assert_eq!(whole.len(), 5);
            for (peer_id, score) in whole {
                assert_eq!(score, algo.score(peer_id, &log, now));
            }
        }
    }

    #[test]
    fn unknown_peers_score_as_nothing_observed() {
        let (log, now) = lilei_12_over_6_days();
        for algo in TieAlgo::ALL {
            let score = algo.score(999, &log, now);
            assert_eq!(score.band, Band::Weak);
            assert_eq!(score.interaction_count, 0);
            assert_eq!(score.first_contact_unix, 0);
            assert_eq!(score.last_contact_unix, 0);
        }
    }
}
