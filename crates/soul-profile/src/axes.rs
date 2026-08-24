//! The five default direction axes.
//!
//! `docs/DECISIONS.md` D22 settles what these are and what they are not: a
//! direction plus a weak/moderate/strong evidence band, never a rating, a
//! percentage or a place on a scale. The words "leans low" and "leans high"
//! are the whole vocabulary, and [`crate::numeric::reject_numeric_rating`]
//! refuses to let anything else reach the store.
//!
//! `axis_id` is a fixed constant rather than something generated at first run.
//! WP01 narrowed every `*_id` in the contracts to UUIDv7 (see the WP01 notes in
//! `docs/STATUS.md`), which left the axes with machine identifiers nobody can
//! read; the readable name lives in `label`. Fixing the constants is what makes
//! an axis the *same* axis across installs, across a re-run of the
//! questionnaire, and across an export — a generated id would silently fork the
//! user's history the first time a profile was rebuilt.

use uuid::Uuid;

use soul_schema::common::{EvidenceBand, NotAClinicalClaim};
use soul_schema::profile::{AxisPosition, TraitAxis};

/// One direction axis: an identity, a readable name, and what each end means.
///
/// The pole descriptions are part of the definition rather than UI copy
/// because "leans_high" is meaningless on its own, and because they go through
/// the same non-clinical check as everything else this crate emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisDefinition {
    pub axis_id: Uuid,
    /// Stable machine key, used to build `statement_key`. Never shown.
    pub key: &'static str,
    pub label: &'static str,
    pub leans_low: &'static str,
    pub leans_high: &'static str,
    /// Id of the questionnaire question that asks about this axis. It lives
    /// here so a question and its axis cannot drift apart. The wording lives
    /// in `soul_import::questionnaire::QUESTIONS`, which is the one list both
    /// crates ask from; [`crate::questionnaire::questionnaire`] pairs the two.
    pub question_id: &'static str,
}

impl AxisDefinition {
    /// `trait_axis.<key>.<position>`, the `statement_key` an inference about
    /// this axis carries.
    pub fn statement_key(&self, position: AxisPosition) -> String {
        format!("trait_axis.{}.{}", self.key, position_key(position))
    }

    /// How this axis reads to a person, given a position.
    pub fn describe(&self, position: AxisPosition) -> String {
        let direction = match position {
            AxisPosition::LeansLow => self.leans_low,
            AxisPosition::LeansHigh => self.leans_high,
            AxisPosition::Mixed => "两端都有，看场合",
            AxisPosition::Unknown => "还看不出方向",
        };
        format!("{}：{direction}", self.label)
    }

    /// A fresh axis with no observations behind it.
    pub fn blank(&self) -> TraitAxis {
        TraitAxis {
            axis_id: self.axis_id,
            label: Some(self.label.to_owned()),
            position: AxisPosition::Unknown,
            evidence_band: EvidenceBand::None,
            evidence_ids: None,
            locked_by_user: Some(false),
            clinical_claim: NotAClinicalClaim,
        }
    }
}

/// Openness-like axis: how much pull the unfamiliar has.
pub const CURIOSITY: AxisDefinition = AxisDefinition {
    axis_id: Uuid::from_u128(0x0192b0c0_5001_7a01_8b01_000000000001),
    key: "curiosity",
    label: "好奇与开放",
    leans_low: "偏向熟悉稳妥的做法",
    leans_high: "偏向尝试新的做法",
    question_id: "q.axis.curiosity",
};

/// Conscientiousness-like axis: plan first, or start and adjust.
pub const ORDERLINESS: AxisDefinition = AxisDefinition {
    axis_id: Uuid::from_u128(0x0192b0c0_5001_7a02_8b02_000000000002),
    key: "orderliness",
    label: "条理与执行",
    leans_low: "偏向随性推进",
    leans_high: "偏向先规划再动手",
    question_id: "q.axis.orderliness",
};

/// Extraversion-like axis: where the energy comes back from.
pub const SOCIAL_ENERGY: AxisDefinition = AxisDefinition {
    axis_id: Uuid::from_u128(0x0192b0c0_5001_7a03_8b03_000000000003),
    key: "social_energy",
    label: "社交能量",
    leans_low: "偏向独处时回血",
    leans_high: "偏向人群里回血",
    question_id: "q.axis.social_energy",
};

/// Agreeableness-like axis: hold the line, or make room.
pub const ACCOMMODATION: AxisDefinition = AxisDefinition {
    axis_id: Uuid::from_u128(0x0192b0c0_5001_7a04_8b04_000000000004),
    key: "accommodation",
    label: "协作与体谅",
    leans_low: "偏向直说与坚持己见",
    leans_high: "偏向迁就与照顾对方",
    question_id: "q.axis.accommodation",
};

/// Emotional-steadiness axis. Deliberately framed as day-to-day variability
/// rather than as anything a clinician would recognise: PRODUCT_LOCK rules out
/// medical claims, and this is the axis where that is easiest to get wrong.
pub const EMOTIONAL_STEADINESS: AxisDefinition = AxisDefinition {
    axis_id: Uuid::from_u128(0x0192b0c0_5001_7a05_8b05_000000000005),
    key: "emotional_steadiness",
    label: "情绪起伏",
    leans_low: "日常起伏比较平缓",
    leans_high: "日常起伏比较明显",
    question_id: "q.axis.emotional_steadiness",
};

/// The five axes a profile starts with, in the order the questionnaire asks
/// about them.
pub const DEFAULT_AXES: [AxisDefinition; 5] = [
    CURIOSITY,
    ORDERLINESS,
    SOCIAL_ENERGY,
    ACCOMMODATION,
    EMOTIONAL_STEADINESS,
];

pub fn axis_by_id(axis_id: Uuid) -> Option<AxisDefinition> {
    DEFAULT_AXES.into_iter().find(|a| a.axis_id == axis_id)
}

pub fn axis_by_key(key: &str) -> Option<AxisDefinition> {
    DEFAULT_AXES.into_iter().find(|a| a.key == key)
}

/// The five axes with no observations behind them yet.
pub fn blank_axes() -> Vec<TraitAxis> {
    DEFAULT_AXES.iter().map(AxisDefinition::blank).collect()
}

/// The wire spelling of a position, matching `profile.schema.json`.
pub fn position_key(position: AxisPosition) -> &'static str {
    match position {
        AxisPosition::LeansLow => "leans_low",
        AxisPosition::Mixed => "mixed",
        AxisPosition::LeansHigh => "leans_high",
        AxisPosition::Unknown => "unknown",
    }
}

/// The position a questionnaire option key names.
///
/// This is the other half of [`position_key`], and it is what turns the
/// opaque option token an answer crosses the seam with back into a position.
/// `unknown` is not offered by the questionnaire, but it round-trips here so
/// the two functions stay inverses of each other.
pub fn position_by_key(key: &str) -> Option<AxisPosition> {
    [
        AxisPosition::LeansLow,
        AxisPosition::Mixed,
        AxisPosition::LeansHigh,
        AxisPosition::Unknown,
    ]
    .into_iter()
    .find(|position| position_key(*position) == key)
}
