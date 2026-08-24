//! **A2 — statistical personnel summary.** The no-key path for WP10
//! (`PRODUCT_LOCK.md` 切片 6, D21: 人事分析是否门禁 — 是；无 key 则统计降级).
//!
//! A2 turns counts that are already in the graph into sentences a person can
//! check against those same counts. It never reads a message, never asks
//! whether the peer is a friend, and never says anything about who they are.
//! What it is allowed to talk about is the record of contact: whether it goes
//! both ways, how long ago the last one was, whether it has only ever happened
//! in a group, and how much of it there is.
//!
//! Everything else is out of scope on purpose, and the reasons are not
//! stylistic:
//!
//! - **No message text.** The peer is a third party who never consented; their
//!   words are `local_only` and are not an input here at all, so there is
//!   nothing for a redactor to catch later.
//! - **No clinical word.** Screened by [`crate::denylist::diagnostic_hit`].
//! - **No rating.** Counts, not scores; screened by
//!   [`crate::denylist::numeric_rating_hit`].
//! - **No relationship or personality claim.** Screened by
//!   [`crate::denylist::peer_claim_hit`]. "你们是朋友" is not something a tally
//!   of messages can know.
//!
//! Every bullet cites evidence. When there is nothing to cite there are no
//! bullets: see [`a2_personnel_summary`] for the empty case, which is a
//! decision this crate documents rather than a default that happened.

use crate::types::Band;

/// Identifier A2 stamps on the summaries it produces.
pub const A2_ALGORITHM_ID: &str = "a2.statistical_personnel_summary.v1";

/// Seconds in a day, for the recency bullet.
const SECONDS_PER_DAY: i64 = 86_400;

/// Interactions before the record is thick enough to call `Moderate`.
const MODERATE_MIN_INTERACTIONS: u32 = 3;

/// Interactions before the record is thick enough to call `Strong`.
const STRONG_MIN_INTERACTIONS: u32 = 10;

/// Separate days before the record is spread out enough to call `Strong`.
const STRONG_MIN_ACTIVE_DAYS: u32 = 3;

/// What the graph already knows about one peer. No names, no text.
///
/// The counts are the ones `graph_build.rs` computes for an edge, so A2 is a
/// second reader of numbers the user can already see, not a new measurement.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PeerStats {
    /// Interactions on this edge in either direction.
    pub interaction_count: u32,
    /// Interactions the owner sent.
    pub outgoing: u32,
    /// Interactions the peer sent.
    pub incoming: u32,
    /// Separate days on which any interaction happened.
    pub active_days: u32,
    /// When the most recent interaction happened, if there was one.
    pub last_contact_unix: Option<i64>,
    /// Whether this peer has ever been seen outside a group venue.
    pub venue_direct_seen: bool,
    /// Whether the edge has been observed in both directions.
    pub reciprocal: bool,
    /// The evidence rows these counts came from. A summary with nothing to
    /// cite produces no bullets.
    pub evidence_ids: Vec<u64>,
    /// The row the most recent interaction came from, when it is known
    /// separately. Falls back to [`PeerStats::evidence_ids`].
    pub last_contact_evidence_id: Option<u64>,
}

/// One checkable sentence about the record of contact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryBullet {
    /// Stable key for this kind of statement, e.g. `personnel.tie.reciprocal`.
    /// The UI may translate or restyle the sentence; the key is what the audit
    /// chain and any later review refer to.
    pub statement_key: String,
    /// The sentence itself, non-clinical and non-diagnostic.
    pub text_zh: String,
    /// The rows that back it. Never empty.
    pub evidence_ids: Vec<u64>,
    /// How well backed it is.
    pub band: Band,
}

/// Everything A2 is willing to say about one peer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersonnelSummary {
    /// The bullets, in a fixed order: direction of contact, how much of it,
    /// how long ago, then the group-only caveat.
    pub bullets: Vec<SummaryBullet>,
}

impl PersonnelSummary {
    /// Whether A2 found nothing it could say.
    pub fn is_empty(&self) -> bool {
        self.bullets.is_empty()
    }

    /// Every sentence in the summary, for screening.
    pub fn texts(&self) -> Vec<&str> {
        self.bullets
            .iter()
            .map(|bullet| bullet.text_zh.as_str())
            .collect()
    }
}

/// Summarise one peer as of `now_unix`.
///
/// # The empty case
///
/// With no interactions or no citable evidence the result has **no bullets at
/// all**. The alternative — a single 「还看不出」 bullet at `Band::None` — was
/// rejected because every bullet in this type promises a non-empty
/// `evidence_ids`, and a bullet that cites nothing to say nothing would be the
/// first exception to「无证据不得落库」. Saying "还看不出" is the caller's job
/// and the caller's copy; [`PersonnelSummary::is_empty`] is how it knows.
///
/// Nothing here reads a clock: `now_unix` is a parameter so the same stats
/// always produce the same sentence.
pub fn a2_personnel_summary(stats: &PeerStats, now_unix: i64) -> PersonnelSummary {
    if stats.interaction_count == 0 || stats.evidence_ids.is_empty() {
        return PersonnelSummary::default();
    }

    let mut bullets = Vec::new();
    let sample = sample_band(stats);

    if let Some(bullet) = direction_bullet(stats, sample) {
        bullets.push(bullet);
    }
    bullets.push(activity_bullet(stats, sample));
    if let Some(bullet) = recency_bullet(stats, now_unix, sample) {
        bullets.push(bullet);
    }
    if let Some(bullet) = group_only_bullet(stats, sample) {
        bullets.push(bullet);
    }

    PersonnelSummary { bullets }
}

/// How thick the record is, on the same thresholds the graph uses for tie
/// strength, so a bullet and an edge never disagree about the same counts.
fn sample_band(stats: &PeerStats) -> Band {
    if stats.interaction_count == 0 {
        Band::None
    } else if stats.reciprocal
        && stats.interaction_count >= STRONG_MIN_INTERACTIONS
        && stats.active_days >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if stats.interaction_count >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    }
}

/// Whether the contact goes both ways. A one-way record is never `Strong`:
/// absence of a reply is thin evidence of anything.
fn direction_bullet(stats: &PeerStats, sample: Band) -> Option<SummaryBullet> {
    let both_ways = stats.outgoing > 0 && stats.incoming > 0;

    let (key, text, band) = if stats.reciprocal && both_ways {
        (
            "personnel.tie.reciprocal",
            format!(
                "往来是双向的：你发出过 {} 次，对方发来过 {} 次。",
                stats.outgoing, stats.incoming
            ),
            sample,
        )
    } else if both_ways {
        (
            "personnel.tie.two_way_unconfirmed",
            format!(
                "两边都有记录：你发出过 {} 次，对方发来过 {} 次，但还没算作稳定的双向往来。",
                stats.outgoing, stats.incoming
            ),
            sample.capped_at(Band::Moderate),
        )
    } else if stats.outgoing > 0 {
        (
            "personnel.tie.one_way_outgoing",
            format!(
                "目前只看到你发出的往来，共 {} 次，没有对方发来的记录。",
                stats.outgoing
            ),
            sample.capped_at(Band::Moderate),
        )
    } else if stats.incoming > 0 {
        (
            "personnel.tie.one_way_incoming",
            format!(
                "目前只看到对方发来的往来，共 {} 次，没有你发出的记录。",
                stats.incoming
            ),
            sample.capped_at(Band::Moderate),
        )
    } else {
        return None;
    };

    Some(SummaryBullet {
        statement_key: key.to_owned(),
        text_zh: text,
        evidence_ids: stats.evidence_ids.clone(),
        band,
    })
}

/// How much contact there is, and how spread out. Counts only.
fn activity_bullet(stats: &PeerStats, sample: Band) -> SummaryBullet {
    let text = if stats.active_days > 0 {
        format!(
            "有记录的往来 {} 次，出现在 {} 个不同的日子。",
            stats.interaction_count, stats.active_days
        )
    } else {
        format!("有记录的往来 {} 次。", stats.interaction_count)
    };

    SummaryBullet {
        statement_key: "personnel.activity.counts".to_owned(),
        text_zh: text,
        evidence_ids: stats.evidence_ids.clone(),
        band: sample,
    }
}

/// How long ago the last contact was. Capped at `Moderate`: it rests on one
/// timestamp however thick the rest of the record is.
fn recency_bullet(stats: &PeerStats, now_unix: i64, sample: Band) -> Option<SummaryBullet> {
    let last = stats.last_contact_unix?;
    // A timestamp ahead of `now_unix` means a clock disagreement upstream, not
    // contact in the future; the honest reading is "just now".
    let days = ((now_unix - last).max(0)) / SECONDS_PER_DAY;

    let text = if days == 0 {
        "最近一次往来就在今天。".to_owned()
    } else {
        format!("最近一次往来距今 {days} 天。")
    };

    let evidence_ids = match stats.last_contact_evidence_id {
        Some(id) => vec![id],
        None => stats.evidence_ids.clone(),
    };

    Some(SummaryBullet {
        statement_key: "personnel.recency.days_since_last".to_owned(),
        text_zh: text,
        evidence_ids,
        band: sample.capped_at(Band::Moderate),
    })
}

/// Only emitted when the record really is group-only. There is no matching
/// bullet for the other case: "we have also seen you two one to one" is a step
/// toward describing a relationship, which A2 does not do.
fn group_only_bullet(stats: &PeerStats, sample: Band) -> Option<SummaryBullet> {
    if stats.venue_direct_seen {
        return None;
    }

    Some(SummaryBullet {
        statement_key: "personnel.venue.group_only".to_owned(),
        text_zh: "到目前为止只在群聊里见过往来，没有一对一的记录。".to_owned(),
        evidence_ids: stats.evidence_ids.clone(),
        band: sample.capped_at(Band::Moderate),
    })
}
