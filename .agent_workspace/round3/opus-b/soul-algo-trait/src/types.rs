//! The vocabulary every algorithm in this crate shares.
//!
//! It is deliberately small and deliberately non-numeric. `PRODUCT_LOCK.md`
//! ("心理模型" row) and `DECISIONS.md` D22 allow exactly two things to be said
//! about an axis: a direction (`leans_low | mixed | leans_high | unknown`) and
//! an evidence band (`weak | moderate | strong`). There is no score, no
//! percentile and no scale anywhere in this module, and [`Band`] deliberately
//! does not implement `Ord` so that no caller can quietly start doing
//! arithmetic on it.
//!
//! Round 2 adds two fields to [`EvidenceRef`]: [`EvidenceRef::kind`] and
//! [`EvidenceRef::recorded_at_unix`]. They exist only so that A1 can compute
//! the independence key `(kind, utc_day)` that `CANDIDATE_SPEC.md` froze. A0
//! ignores both.

/// How well supported a claim is. Not a magnitude of the claim itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// Nothing supports this yet. Pairs with [`Position::Unknown`].
    None,
    /// One thin observation.
    Weak,
    /// What a questionnaire answer is worth: the user describing themselves
    /// from memory.
    Moderate,
    /// What a correction is worth, or what several independent agreeing groups
    /// of observations are worth under A1.
    Strong,
}

impl Band {
    /// Rung on the none/weak/moderate/strong ladder.
    ///
    /// Exposed as a method rather than an `Ord` impl so that comparing bands
    /// is always a visible, deliberate call and never an accidental sort key.
    pub fn rank(self) -> u8 {
        match self {
            Band::None => 0,
            Band::Weak => 1,
            Band::Moderate => 2,
            Band::Strong => 3,
        }
    }

    /// Whether this band sits at or above `floor`.
    pub fn at_least(self, floor: Band) -> bool {
        self.rank() >= floor.rank()
    }

    /// The higher of two bands.
    pub fn max_of(left: Band, right: Band) -> Band {
        if left.rank() >= right.rank() {
            left
        } else {
            right
        }
    }

    /// This band, lowered to `ceiling` if it sits above it.
    pub fn capped_at(self, ceiling: Band) -> Band {
        if self.rank() <= ceiling.rank() {
            self
        } else {
            ceiling
        }
    }

    /// The wire spelling, matching `evidence_band` in the Goal 1 schemas.
    pub fn as_str(self) -> &'static str {
        match self {
            Band::None => "none",
            Band::Weak => "weak",
            Band::Moderate => "moderate",
            Band::Strong => "strong",
        }
    }

    /// The one word a user reads for this band.
    ///
    /// `Band::Strong` reads 「强」 and nothing longer: the phrase 「强关系」 is
    /// a claim about the relationship, which nothing in this crate is entitled
    /// to make. See `a2::a2_render`.
    pub fn label_zh(self) -> &'static str {
        match self {
            Band::None => "还看不出",
            Band::Weak => "弱",
            Band::Moderate => "中",
            Band::Strong => "强",
        }
    }
}

/// Which way an axis leans. The whole vocabulary; there is no fifth option and
/// no number behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Leans toward the low pole of the axis.
    LeansLow,
    /// Both ends show up; it depends on the situation.
    Mixed,
    /// Leans toward the high pole of the axis.
    LeansHigh,
    /// No direction can be told yet.
    Unknown,
}

impl Position {
    /// The wire spelling, matching `profile.schema.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Position::LeansLow => "leans_low",
            Position::Mixed => "mixed",
            Position::LeansHigh => "leans_high",
            Position::Unknown => "unknown",
        }
    }

    /// Whether this position names one end of the axis.
    pub fn is_directional(self) -> bool {
        matches!(self, Position::LeansLow | Position::LeansHigh)
    }
}

/// Where an observation came from.
///
/// Mirrors `EvidenceKind` in `crates/soul-schema/src/evidence.rs`, because A1's
/// independence key is `(kind, utc_day)` and the key has to mean the same thing
/// here as it does in the store. v0.1 only ever writes
/// [`EvidenceKind::Questionnaire`] and [`EvidenceKind::UserCorrection`] against
/// a trait axis; the other four exist so that the key is total and so that the
/// "what would make A1 fire" question has a testable answer rather than a
/// prose one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    /// An imported message observation.
    Message,
    /// Observed application usage.
    AppUsage,
    /// An answer to a Soul questionnaire question.
    Questionnaire,
    /// Something the user said about themselves outside the questionnaire.
    UserStatement,
    /// The user looking at a claim and rejecting it.
    UserCorrection,
    /// A derived roll-up of other rows.
    Aggregate,
}

impl EvidenceKind {
    /// Every kind, in schema order.
    pub const ALL: [EvidenceKind; 6] = [
        EvidenceKind::Message,
        EvidenceKind::AppUsage,
        EvidenceKind::Questionnaire,
        EvidenceKind::UserStatement,
        EvidenceKind::UserCorrection,
        EvidenceKind::Aggregate,
    ];

    /// The wire spelling, matching `evidence.schema.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceKind::Message => "message",
            EvidenceKind::AppUsage => "app_usage",
            EvidenceKind::Questionnaire => "questionnaire",
            EvidenceKind::UserStatement => "user_statement",
            EvidenceKind::UserCorrection => "user_correction",
            EvidenceKind::Aggregate => "aggregate",
        }
    }

    /// Whether a v0.1 install can produce this kind against a trait axis.
    ///
    /// Only the questionnaire and a correction can. This is the fact the
    /// `EMPTY_ON_V01` question in `REPORT.md` turns on, so it is a function
    /// with a test rather than a sentence in a document.
    pub fn reachable_for_axis_in_v01(self) -> bool {
        matches!(
            self,
            EvidenceKind::Questionnaire | EvidenceKind::UserCorrection
        )
    }
}

/// Seconds in a day.
pub const SECONDS_PER_DAY: i64 = 86_400;

/// The UTC day a timestamp falls in, as a day number counted from the epoch.
///
/// Euclidean division rather than truncating division, so that a timestamp
/// before 1970 lands in the day it actually belongs to instead of rounding
/// toward the epoch. Two observations on the same UTC day are one group under
/// A1, and that has to stay true either side of 1970 or the key is not a key.
pub const fn utc_day(unix_seconds: i64) -> i64 {
    unix_seconds.div_euclid(SECONDS_PER_DAY)
}

/// The five default direction axes, mirroring `crates/soul-profile/src/axes.rs`
/// in Goal 1 (`profile_axes.rs` in the context snapshot).
///
/// The identifiers are fixed rather than generated: a generated id would fork
/// the user's history the first time a profile was rebuilt. The UUIDs Goal 1
/// uses are not repeated here because this crate has no `uuid` dependency;
/// [`AxisId::key`] is the stable join key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisId {
    /// Openness-like: how much pull the unfamiliar has.
    Curiosity,
    /// Conscientiousness-like: plan first, or start and adjust.
    Orderliness,
    /// Extraversion-like: where the energy comes back from.
    SocialEnergy,
    /// Agreeableness-like: hold the line, or make room.
    Accommodation,
    /// Day-to-day emotional variability, framed so that no clinician would
    /// recognise it as a construct. D5: this product makes no medical claim.
    EmotionalSteadiness,
}

impl AxisId {
    /// Every axis, in the order the questionnaire asks about them.
    pub const ALL: [AxisId; 5] = [
        AxisId::Curiosity,
        AxisId::Orderliness,
        AxisId::SocialEnergy,
        AxisId::Accommodation,
        AxisId::EmotionalSteadiness,
    ];

    /// Stable machine key. Never shown to the user.
    pub fn key(self) -> &'static str {
        match self {
            AxisId::Curiosity => "curiosity",
            AxisId::Orderliness => "orderliness",
            AxisId::SocialEnergy => "social_energy",
            AxisId::Accommodation => "accommodation",
            AxisId::EmotionalSteadiness => "emotional_steadiness",
        }
    }

    /// The readable name of the axis.
    pub fn label(self) -> &'static str {
        match self {
            AxisId::Curiosity => "好奇与开放",
            AxisId::Orderliness => "条理与执行",
            AxisId::SocialEnergy => "社交能量",
            AxisId::Accommodation => "协作与体谅",
            AxisId::EmotionalSteadiness => "情绪起伏",
        }
    }

    /// What the low end of this axis means in words.
    pub fn leans_low(self) -> &'static str {
        match self {
            AxisId::Curiosity => "偏向熟悉稳妥的做法",
            AxisId::Orderliness => "偏向随性推进",
            AxisId::SocialEnergy => "偏向独处时回血",
            AxisId::Accommodation => "偏向直说与坚持己见",
            AxisId::EmotionalSteadiness => "日常起伏比较平缓",
        }
    }

    /// What the high end of this axis means in words.
    pub fn leans_high(self) -> &'static str {
        match self {
            AxisId::Curiosity => "偏向尝试新的做法",
            AxisId::Orderliness => "偏向先规划再动手",
            AxisId::SocialEnergy => "偏向人群里回血",
            AxisId::Accommodation => "偏向迁就与照顾对方",
            AxisId::EmotionalSteadiness => "日常起伏比较明显",
        }
    }

    /// `trait_axis.<key>.<position>`, the statement key an inference carries.
    pub fn statement_key(self, position: Position) -> String {
        format!("trait_axis.{}.{}", self.key(), position.as_str())
    }

    /// How this axis reads to a person, given a position.
    pub fn describe(self, position: Position) -> String {
        let direction = match position {
            Position::LeansLow => self.leans_low(),
            Position::LeansHigh => self.leans_high(),
            Position::Mixed => "两端都有，看场合",
            Position::Unknown => "还看不出方向",
        };
        format!("{}：{direction}", self.label())
    }
}

/// One stored observation about one axis.
///
/// `locked_by_user` marks an explicit correction: the user looked at the claim
/// and rejected it, which is the strongest signal this product can get and the
/// only thing that pins an axis. `forgotten` marks a row whose content key has
/// been destroyed; every algorithm here drops those before it aggregates, so a
/// derived state can always be recomputed after a forget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    /// Local id of the evidence row.
    pub evidence_id: u64,
    /// Which axis the observation is about.
    pub axis: AxisId,
    /// What this single piece of evidence claims.
    pub position: Position,
    /// How strong this single piece is on its own.
    pub band: Band,
    /// Where it came from. Half of A1's independence key.
    pub kind: EvidenceKind,
    /// When it was recorded. The UTC day of this is the other half of A1's
    /// independence key. A0 never reads it.
    pub recorded_at_unix: i64,
    /// True only for an explicit user correction.
    pub locked_by_user: bool,
    /// True once the user has forgotten the underlying row.
    pub forgotten: bool,
}

impl EvidenceRef {
    /// A questionnaire answer: Moderate, never locking.
    ///
    /// Moderate rather than Strong because the user is describing themselves
    /// from memory, which is better than a guess and weaker than a correction
    /// made while looking at what the profile actually says.
    pub fn questionnaire(
        evidence_id: u64,
        axis: AxisId,
        position: Position,
        recorded_at_unix: i64,
    ) -> Self {
        EvidenceRef {
            evidence_id,
            axis,
            position,
            band: Band::Moderate,
            kind: EvidenceKind::Questionnaire,
            recorded_at_unix,
            locked_by_user: false,
            forgotten: false,
        }
    }

    /// An explicit user correction: Strong, and it locks the axis.
    pub fn correction(
        evidence_id: u64,
        axis: AxisId,
        position: Position,
        recorded_at_unix: i64,
    ) -> Self {
        EvidenceRef {
            evidence_id,
            axis,
            position,
            band: Band::Strong,
            kind: EvidenceKind::UserCorrection,
            recorded_at_unix,
            locked_by_user: true,
            forgotten: false,
        }
    }

    /// A machine inference. Never locks, whatever band it carries.
    ///
    /// `kind` is a parameter because the kind is what makes two inferences on
    /// the same day independent of each other under A1.
    pub fn inference(
        evidence_id: u64,
        axis: AxisId,
        position: Position,
        band: Band,
        kind: EvidenceKind,
        recorded_at_unix: i64,
    ) -> Self {
        EvidenceRef {
            evidence_id,
            axis,
            position,
            band,
            kind,
            recorded_at_unix,
            locked_by_user: false,
            forgotten: false,
        }
    }

    /// The same row after the user forgot it.
    pub fn into_forgotten(mut self) -> Self {
        self.forgotten = true;
        self
    }

    /// The band this row is worth once the correction rule is applied.
    ///
    /// A correction is Strong by definition, so a caller cannot weaken one by
    /// mislabelling the row; a position of `Unknown` claims nothing, so it is
    /// worth no band at all.
    pub fn effective_band(&self) -> Band {
        if self.position == Position::Unknown {
            Band::None
        } else if self.locked_by_user {
            Band::Strong
        } else {
            self.band
        }
    }

    /// The UTC day this row was recorded on.
    pub fn utc_day(&self) -> i64 {
        utc_day(self.recorded_at_unix)
    }
}

/// Where one axis currently stands, and what holds it up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisState {
    /// Which axis this is.
    pub axis: AxisId,
    /// The direction, or `Unknown`.
    pub position: Position,
    /// How well supported the direction is.
    pub band: Band,
    /// True once the user has corrected this axis.
    pub locked_by_user: bool,
    /// The rows that support the position the axis is in *now*. Never invented
    /// and, apart from the `Unknown` starting state, never empty.
    pub evidence_ids: Vec<u64>,
    /// Which algorithm produced this state, so two candidates can be compared
    /// row by row.
    pub algorithm_id: &'static str,
}

impl AxisState {
    /// A fresh axis with nothing behind it: `Unknown`, `Band::None`, no
    /// evidence. This is the truthful thing to show before intake.
    pub fn unknown(axis: AxisId, algorithm_id: &'static str) -> Self {
        AxisState {
            axis,
            position: Position::Unknown,
            band: Band::None,
            locked_by_user: false,
            evidence_ids: Vec::new(),
            algorithm_id,
        }
    }

    /// How this state reads to a person, without the band.
    pub fn describe(&self) -> String {
        self.axis.describe(self.position)
    }

    /// `trait_axis.<key>.<position>` for this state.
    pub fn statement_key(&self) -> String {
        self.axis.statement_key(self.position)
    }
}

/// Whether an observation reached the axis, and if not, why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyResult {
    /// The observation is part of what holds the current state up.
    Applied,
    /// The user corrected this axis. The observation is kept in the shadow and
    /// the axis is not touched. This is the only refusal A0's
    /// [`crate::a0::WriteMode::LastWriteWins`] can produce.
    RefusedLocked,
    /// The observation disagreed with the standing position and was worth less
    /// than what already held the axis up, so
    /// [`crate::a0::WriteMode::NoDowngrade`] did not let it clobber. Never
    /// produced under `LastWriteWins`.
    RefusedWeaker,
}

/// One observation as the algorithm saw it, kept whether or not it was applied.
///
/// The refused ones are the point: the user is entitled to see that the machine
/// still thinks otherwise, and the audit chain already records that the
/// inference was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowInference {
    /// Which evidence row this was.
    pub evidence_id: u64,
    /// Which axis it spoke about.
    pub axis: AxisId,
    /// What it claimed.
    pub position: Position,
    /// What it was worth on its own, after [`EvidenceRef::effective_band`].
    pub band: Band,
    /// Whether the row reached the axis, and if not, why not.
    pub result: ApplyResult,
    /// Whether the row ended up in [`AxisState::evidence_ids`]. A row can be
    /// applied without being cited: it was counted, but the position the axis
    /// settled on is not the one it supports.
    pub cited: bool,
}

/// What an axis algorithm returns: the state, plus every observation it read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisOutcome {
    /// The axis as it now stands.
    pub state: AxisState,
    /// Every non-forgotten, non-duplicate observation the run considered, in
    /// input order, each marked applied or refused.
    pub shadow: Vec<ShadowInference>,
}

impl AxisOutcome {
    /// The observations that did not reach the axis, for any reason.
    pub fn refused(&self) -> impl Iterator<Item = &ShadowInference> {
        self.shadow
            .iter()
            .filter(|entry| entry.result != ApplyResult::Applied)
    }

    /// The observations the user's lock kept out, specifically.
    pub fn refused_by_lock(&self) -> impl Iterator<Item = &ShadowInference> {
        self.shadow
            .iter()
            .filter(|entry| entry.result == ApplyResult::RefusedLocked)
    }

    /// Ids of the observations that did not reach the axis.
    pub fn refused_ids(&self) -> Vec<u64> {
        self.refused().map(|entry| entry.evidence_id).collect()
    }

    /// Ids of the observations the user's lock kept out.
    pub fn refused_by_lock_ids(&self) -> Vec<u64> {
        self.refused_by_lock()
            .map(|entry| entry.evidence_id)
            .collect()
    }

    /// Whether any observation was kept out.
    pub fn refused_any(&self) -> bool {
        self.refused().next().is_some()
    }
}
