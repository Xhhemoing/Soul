//! **A2 — statistical personnel summary.** The no-key path for WP10
//! (`PRODUCT_LOCK.md` 切片 6, D21: 人事分析是否门禁 — 是；无 key 则统计降级).
//!
//! # A2 is a renderer, and that is the whole design
//!
//! Round 1's A2 carried its own copy of the tie thresholds so that a bullet and
//! an edge would agree. That was the wrong fix for a real worry: two copies of
//! a rule are two rules, and the moment the graph algorithm changes — which is
//! exactly what the T-family ablation is for — the summary starts contradicting
//! the edge it is describing.
//!
//! So A2 has **no thresholds at all**. It takes a [`TieScore`], which already
//! carries the band the winning tie algorithm assigned, and turns the facts in
//! it into sentences. It does not compare a count to anything, it does not
//! decide whether a record is thick, and it cannot disagree with the graph
//! because it never forms a second opinion. `a2_defines_no_band_thresholds` in
//! `tests/a2_render.rs` reads this file and fails if the constants come back.
//!
//! The one number A2 owns is [`DORMANT_AFTER_DAYS`], and it is not a band: it
//! decides whether one extra **descriptive** sentence appears. The band on that
//! sentence is the edge's band, unchanged. See [`a2_render`].
//!
//! # Round 3: frozen, and widened by exactly two optional fields
//!
//! A2 is frozen alongside A0 as the rendering half of the retained pair. The
//! only change Round 3 makes is additive: [`TieScore::direct_count`] and
//! [`TieScore::group_count`] are `Option<u32>` fields that the T4D proposal
//! (`R2-SYNTHESIS.md` 边界风险 2) would supply. When both are present A2 says
//! what they are; when either is absent it says nothing. It does not derive
//! them, does not compare them to anything, and does not let them touch the
//! band. Widening the input this way is the alternative to the thing that must
//! not happen — A2 going back to the evidence store to work the split out for
//! itself, which would hand it back the second opinion Round 2 took away.
//!
//! # What A2 is allowed to talk about
//!
//! The record of contact: whether it goes both ways, how much of it there is,
//! how spread out it is, whether it has only ever happened in a group, and how
//! long ago the last one was. Everything else is out of scope on purpose:
//!
//! - **No message text.** The peer is a third party who never consented; their
//!   words are `local_only` and are not an input here at all, so there is
//!   nothing for a redactor to catch later.
//! - **No clinical word.** Screened by [`crate::denylist::diagnostic_hit`].
//! - **No rating.** Counts, not scores; screened by
//!   [`crate::denylist::numeric_rating_hit`].
//! - **No relationship or personality claim.** Screened by
//!   [`crate::denylist::peer_claim_hit`]. A tally of messages cannot know that
//!   two people are close, so a `Strong` band reads 「归在「强」一档」 and never
//!   「强关系」 — the band describes the evidence, not the friendship.
//!
//! Every bullet cites evidence. When there is nothing to cite there are no
//! bullets; see [`a2_render`] for why that is the decision rather than a
//! placeholder sentence.

use crate::types::Band;

/// Identifier A2 stamps on the summaries it produces.
///
/// `v3` because Round 3 added one statement key. The rendering rules did not
/// change, but the set of sentences a stored summary can contain did, and a
/// consumer reading old rows is entitled to tell the two vintages apart.
pub const A2_ALGORITHM_ID: &str = "a2.tie_summary_renderer.v3";

/// Every statement key [`a2_render`] can emit, in output order.
///
/// A2's whole output surface, as data. The screening tests walk it so that a
/// new template is screened when it is added rather than when somebody
/// remembers, and `every_statement_key_is_reachable` fails if a key is listed
/// here without a fixture that produces it.
pub const A2_STATEMENT_KEYS: [&str; 9] = [
    "personnel.tie.one_way_outgoing",
    "personnel.tie.one_way_incoming",
    "personnel.tie.two_way",
    "personnel.activity.counts",
    "personnel.venue.direct_and_group_counts",
    "personnel.venue.group_only",
    "personnel.recency.days_since_last",
    "personnel.recency.dormant",
    "personnel.tie.filed_band",
];

/// After how many days without contact the summary adds a dormancy sentence.
///
/// **This is not a band rule.** It gates one descriptive line and nothing else;
/// [`TieScore::band`] passes through untouched, and
/// `dormancy_is_a_sentence_not_a_demotion` pins that. A relationship that has
/// been quiet for a year is still whatever the graph says it is — A2's job is
/// to make sure the user is told about the quiet, not to overrule the graph
/// about what it means.
pub const DORMANT_AFTER_DAYS: i64 = 180;

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// One edge, as the winning tie algorithm scored it.
///
/// Everything A2 is allowed to know about a peer. No name, no text, no
/// identifier that resolves to a person outside this machine.
///
/// `as_of_unix` lives on the struct rather than in the function signature so
/// that [`a2_render`] has exactly one input and cannot be handed a clock. It is
/// the largest timestamp in the store, per `R1-SYNTHESIS.md`
/// (「`as_of` = 数据内最大时间戳，禁读墙钟」), which is what makes a summary
/// reproducible from an export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TieScore {
    /// The band the tie algorithm assigned. A2 reads it and never checks it.
    pub band: Band,
    /// Interactions on this edge in either direction.
    pub interaction_count: u32,
    /// Interactions the owner sent.
    pub outgoing: u32,
    /// Interactions the peer sent.
    pub incoming: u32,
    /// Separate days on which any interaction happened.
    pub active_day_count: u32,
    /// Separate conversations the interactions were spread over.
    pub conversation_count: u32,
    /// Whether this peer has ever been seen outside a group venue.
    pub any_direct: bool,
    /// Interactions that happened one to one, when the tie algorithm splits its
    /// tally by venue. `None` when it does not.
    ///
    /// T4D (`R2-SYNTHESIS.md` 边界风险 2) moves the Strong and Moderate gates
    /// onto the direct count so that a group fan-out plus one private greeting
    /// each way cannot reach Strong. If that lands, the number the gate was
    /// applied to is the number the user needs to see in order to check the
    /// verdict, so A2 renders it. If it does not land the field stays `None`
    /// and no sentence appears — A2 does not reconstruct a venue split from
    /// anything, because reconstructing it would be a second opinion.
    pub direct_count: Option<u32>,
    /// Interactions that happened with other people present, when the tie
    /// algorithm splits its tally by venue. `None` when it does not.
    ///
    /// The counterpart to [`TieScore::direct_count`], and the reason the pair
    /// is worth rendering at all: 「你们一对一往来 X 次、群里同场 Y 次」 is
    /// checkable by a person, and 「往来 137 次」 on an edge that is 137 group
    /// messages is not.
    pub group_count: Option<u32>,
    /// When the most recent interaction happened, if there was one.
    pub last_contact_unix: Option<i64>,
    /// The largest timestamp in the store. Never a wall clock.
    pub as_of_unix: i64,
    /// The evidence rows behind these counts. **Empty means no bullets.**
    pub evidence_ids: Vec<u64>,
    /// The row the most recent interaction came from, when it is known
    /// separately. Falls back to [`TieScore::evidence_ids`].
    pub last_contact_evidence_id: Option<u64>,
}

impl Default for TieScore {
    fn default() -> Self {
        TieScore {
            band: Band::None,
            interaction_count: 0,
            outgoing: 0,
            incoming: 0,
            active_day_count: 0,
            conversation_count: 0,
            any_direct: false,
            direct_count: None,
            group_count: None,
            last_contact_unix: None,
            as_of_unix: 0,
            evidence_ids: Vec::new(),
            last_contact_evidence_id: None,
        }
    }
}

impl TieScore {
    /// Whole days between the last contact and `as_of`.
    ///
    /// `None` when there has been no contact. A last contact ahead of `as_of`
    /// means a clock disagreement upstream rather than contact in the future,
    /// so the gap is clamped to zero and reads as "today".
    pub fn days_since_last_contact(&self) -> Option<i64> {
        let last = self.last_contact_unix?;
        Some((self.as_of_unix - last).max(0) / SECONDS_PER_DAY)
    }

    /// Whether the dormancy sentence applies.
    pub fn is_dormant(&self) -> bool {
        matches!(self.days_since_last_contact(), Some(days) if days > DORMANT_AFTER_DAYS)
    }

    /// The venue split, when the tie algorithm supplied one.
    ///
    /// Present only when **both** halves are, because half a split is not a
    /// split: "12 one to one" with no group figure invites the reader to
    /// subtract, and subtracting would make A2 the author of a number nobody
    /// gave it. A missing half is a missing sentence.
    pub fn venue_split(&self) -> Option<(u32, u32)> {
        Some((self.direct_count?, self.group_count?))
    }
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
    /// The edge's band, copied. A2 never adjusts it per bullet, because a
    /// per-bullet adjustment is a second opinion with extra steps.
    pub band: Band,
}

/// Everything A2 is willing to say about one peer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersonnelSummary {
    /// The bullets, in a fixed order: direction of contact, how much of it, how
    /// it splits between one-to-one and group when that is known, whether it
    /// has only ever been in a group, how long ago the last one was, whether it
    /// has gone quiet, and finally which band the graph filed it under.
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

    /// Whether a statement of this kind was emitted.
    pub fn has(&self, statement_key: &str) -> bool {
        self.bullets
            .iter()
            .any(|bullet| bullet.statement_key == statement_key)
    }

    /// The copy a caller shows when [`PersonnelSummary::is_empty`] is true.
    ///
    /// A string constant rather than a bullet, because a bullet in this crate
    /// promises non-empty `evidence_ids`, and「无证据不得落库」has no exceptions.
    pub fn nothing_to_say_zh() -> &'static str {
        "还看不出什么，这个人目前没有可以引用的往来记录。"
    }
}

/// Render one scored edge.
///
/// # The empty case
///
/// With no citable evidence the result has **no bullets at all**. The
/// alternative the brief allows — one 「还看不出」 bullet at [`Band::None`] —
/// was rejected because every bullet in this type promises a non-empty
/// `evidence_ids`, and a bullet that cites nothing in order to say nothing
/// would be the first exception to「无证据不得落库」. The sentence still exists,
/// as [`PersonnelSummary::nothing_to_say_zh`]; it is the caller's copy, and
/// [`PersonnelSummary::is_empty`] is how the caller knows to show it. The
/// difference matters at the write boundary: an empty summary is not stored,
/// and a placeholder bullet would have been.
///
/// A [`Band::None`] score with real evidence behind it is a different case and
/// does render: the counts are facts, and the filing line says the band is not
/// settled yet.
///
/// Nothing here reads a clock, compares a count to a threshold, or looks at a
/// message.
pub fn a2_render(score: &TieScore) -> PersonnelSummary {
    if score.evidence_ids.is_empty() {
        return PersonnelSummary::default();
    }

    let mut bullets = Vec::new();
    if let Some(bullet) = direction_bullet(score) {
        bullets.push(bullet);
    }
    bullets.push(activity_bullet(score));
    if let Some(bullet) = venue_split_bullet(score) {
        bullets.push(bullet);
    }
    if let Some(bullet) = venue_bullet(score) {
        bullets.push(bullet);
    }
    if let Some(bullet) = recency_bullet(score) {
        bullets.push(bullet);
    }
    if let Some(bullet) = dormancy_bullet(score) {
        bullets.push(bullet);
    }
    bullets.push(filing_bullet(score));

    PersonnelSummary { bullets }
}

// ------------------------------------------------------------- internals ---

fn bullet(key: &str, text: String, evidence_ids: Vec<u64>, band: Band) -> SummaryBullet {
    SummaryBullet {
        statement_key: key.to_owned(),
        text_zh: text,
        evidence_ids,
        band,
    }
}

/// Which way the contact runs. Counts only; no inference about why.
fn direction_bullet(score: &TieScore) -> Option<SummaryBullet> {
    let (key, text) = match (score.outgoing, score.incoming) {
        (0, 0) => return None,
        (out, 0) => (
            "personnel.tie.one_way_outgoing",
            format!("目前只看到你发出的往来，共 {out} 次，没有对方发来的记录。"),
        ),
        (0, incoming) => (
            "personnel.tie.one_way_incoming",
            format!("目前只看到对方发来的往来，共 {incoming} 次，没有你发出的记录。"),
        ),
        (out, incoming) => (
            "personnel.tie.two_way",
            format!("往来是双向的：你发出过 {out} 次，对方发来过 {incoming} 次。"),
        ),
    };

    Some(bullet(key, text, score.evidence_ids.clone(), score.band))
}

/// How much contact there is and how spread out. Counts only.
fn activity_bullet(score: &TieScore) -> SummaryBullet {
    let text = match (score.active_day_count, score.conversation_count) {
        (0, _) => format!("有记录的往来 {} 次。", score.interaction_count),
        (days, 0) => format!(
            "有记录的往来 {} 次，出现在 {days} 个不同的日子。",
            score.interaction_count
        ),
        (days, conversations) => format!(
            "有记录的往来 {} 次，出现在 {days} 个不同的日子、{conversations} 个会话里。",
            score.interaction_count
        ),
    };

    bullet(
        "personnel.activity.counts",
        text,
        score.evidence_ids.clone(),
        score.band,
    )
}

/// The venue split, restated. Two numbers the tie algorithm already computed,
/// in a sentence, so the user can check the count the band was decided on.
///
/// It renders whatever it is handed. A `direct_count` of zero next to an
/// `any_direct` of true is a disagreement inside the score, and A2 reports the
/// numbers rather than picking a winner: adjudicating between two fields of its
/// own input is exactly the second opinion this module does not hold.
fn venue_split_bullet(score: &TieScore) -> Option<SummaryBullet> {
    let (direct, group) = score.venue_split()?;

    Some(bullet(
        "personnel.venue.direct_and_group_counts",
        format!("其中一对一往来 {direct} 次，群里同场 {group} 次。"),
        score.evidence_ids.clone(),
        score.band,
    ))
}

/// Only emitted when the record really is group-only. There is no matching
/// bullet for the other case: "we have also seen you two one to one" is a step
/// toward describing a relationship, which A2 does not do.
fn venue_bullet(score: &TieScore) -> Option<SummaryBullet> {
    if score.any_direct {
        return None;
    }
    Some(bullet(
        "personnel.venue.group_only",
        "到目前为止只在群聊里见过往来，没有一对一的记录。".to_owned(),
        score.evidence_ids.clone(),
        score.band,
    ))
}

/// How long ago the last contact was, counted from `as_of` rather than a clock.
fn recency_bullet(score: &TieScore) -> Option<SummaryBullet> {
    let days = score.days_since_last_contact()?;
    let text = if days == 0 {
        "最近一次往来就在最新的记录当天。".to_owned()
    } else {
        format!("最近一次往来距最新的记录 {days} 天。")
    };

    Some(bullet(
        "personnel.recency.days_since_last",
        text,
        recency_evidence(score),
        score.band,
    ))
}

/// The dormancy line. A description of the gap, not a demotion: the band on it
/// is the edge's band, exactly as it arrived.
fn dormancy_bullet(score: &TieScore) -> Option<SummaryBullet> {
    if !score.is_dormant() {
        return None;
    }
    let days = score.days_since_last_contact()?;

    Some(bullet(
        "personnel.recency.dormant",
        format!("已经 {days} 天没有新的往来了；下面的归档说的是过去的记录，不是现在的联系频率。"),
        recency_evidence(score),
        score.band,
    ))
}

/// Which band the graph filed this edge under, said out loud so the user can
/// argue with it. A2 copies the word; it does not choose it.
fn filing_bullet(score: &TieScore) -> SummaryBullet {
    let text = match score.band {
        Band::None => {
            "按上面的计数，还看不出该归在哪一档。这是对记录的归档，不是对这个人的评价。".to_owned()
        }
        band => format!(
            "按上面的计数，这条往来归在「{}」一档。这是对记录的归档，不是对这个人的评价。",
            band.label_zh()
        ),
    };

    bullet(
        "personnel.tie.filed_band",
        text,
        score.evidence_ids.clone(),
        score.band,
    )
}

fn recency_evidence(score: &TieScore) -> Vec<u64> {
    match score.last_contact_evidence_id {
        Some(id) => vec![id],
        None => score.evidence_ids.clone(),
    }
}
