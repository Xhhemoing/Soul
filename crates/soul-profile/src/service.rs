//! Reading and writing the profile through the storage boundary.
//!
//! Three rules are implemented here rather than left to callers, because each
//! of them is a promise in `docs/PRODUCT_LOCK.md` that a caller could otherwise
//! forget:
//!
//! 1. **No inference without evidence.** [`record_axis_inference`] refuses an
//!    empty `evidence_ids` before it touches the store, and the store refuses
//!    one that cites evidence which does not resolve. Two nets, because the
//!    profile must never be left half-updated behind a rejected write.
//! 2. **A user correction wins.** [`correct_axis`] sets `locked_by_user`, and
//!    every later inference about that axis is stored but not applied. The
//!    inference is kept rather than dropped: the user is entitled to see that
//!    the machine still thinks otherwise, and the audit chain already records
//!    that it was written.
//! 3. **Nothing numeric, nothing clinical.** Every profile write goes through
//!    [`crate::numeric::reject_numeric_rating`] and every readable string
//!    through [`soul_policy::assert_non_clinical`].
//!
//! The functions are generic over the storage traits rather than over a
//! concrete store, so the same code runs against `FakeStore` and against
//! SQLCipher. Nothing here reads the clock: `now_unix_seconds` is a parameter
//! so a replay and a test both produce the same audit entries.

use serde_json::{json, Value};
use uuid::Uuid;

use soul_import::questionnaire as recorder;
use soul_policy::audit::{append_or_store_error, AuditContent, ReasonCode};
use soul_policy::clinical::assert_non_clinical;
use soul_schema::audit::{AuditAction, AuditDecision};
use soul_schema::common::{
    EvidenceBand, NotAClinicalClaim, Privacy, Purpose, SchemaVersion, Subject, SupportedBand,
    Timestamp,
};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::inference::{InferenceMethod, SoulInference, UserVerdict};
use soul_schema::profile::{AxisPosition, SoulProfile, TraitAxis};
use soul_store_api::types::StoreError;
use soul_store_api::{AuditLog, BlobStore, EventStore, ProfileStore};

use crate::axes::{self, blank_axes, AxisDefinition};
use crate::error::{ProfileError, ProfileResult};
use crate::numeric::reject_numeric_rating;
use crate::questionnaire::{self, QuestionTarget, QuestionnaireResponse, StatedField};
use crate::sink::{ProfileSink, StagedAnswer, StagedValue};
use crate::voice::{VoiceProfile, VoiceSetting};

/// What a questionnaire answer is worth as evidence.
///
/// Moderate, not strong: the user is describing themselves from memory, which
/// is better than a guess and weaker than a correction made while looking at
/// what the profile actually says. The recorder writes the evidence now, so
/// the constant lives there and is re-exported here under the name WP03 gave
/// it — one grade, not two.
pub const QUESTIONNAIRE_STRENGTH: SupportedBand = recorder::QUESTIONNAIRE_STRENGTH;

/// What a correction is worth. The user is looking at the claim and rejecting
/// it, which is the strongest signal this product can get.
pub const CORRECTION_STRENGTH: SupportedBand = SupportedBand::Strong;

/// What one completed questionnaire produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakeOutcome {
    pub profile: SoulProfile,
    /// One row per answer, in the order they were answered. Every axis the
    /// questionnaire moved cites one of these.
    pub evidence_ids: Vec<Uuid>,
    /// The event each answer was recorded as, same order. A prose answer's
    /// words are in the sealed body of one of these and nowhere else.
    pub event_ids: Vec<Uuid>,
}

/// Whether an inference reached the profile, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisUpdate {
    Applied,
    /// The user corrected this axis. The inference is stored; the axis is not
    /// touched.
    RefusedAxisLocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceOutcome {
    pub inference_id: Uuid,
    pub update: AxisUpdate,
}

/// One inference about one axis, before it has been written anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisProposal {
    pub axis_id: Uuid,
    pub position: AxisPosition,
    pub band: SupportedBand,
    /// Must be non-empty and must resolve. This is the whole of AC-06.
    pub evidence_ids: Vec<Uuid>,
    pub method: InferenceMethod,
    /// What observation would overturn this.
    pub falsifier: Option<String>,
}

impl AxisProposal {
    pub fn new(
        axis_id: Uuid,
        position: AxisPosition,
        band: SupportedBand,
        evidence_ids: Vec<Uuid>,
    ) -> Self {
        AxisProposal {
            axis_id,
            position,
            band,
            evidence_ids,
            method: InferenceMethod::Rule,
            falsifier: None,
        }
    }

    pub fn by(mut self, method: InferenceMethod) -> Self {
        self.method = method;
        self
    }

    pub fn falsified_by(mut self, falsifier: impl Into<String>) -> Self {
        self.falsifier = Some(falsifier.into());
        self
    }
}

/// The profile as stored, or a blank one if the user has not started yet.
///
/// A blank profile is five axes at `unknown` with no evidence, which is a
/// truthful thing to show. Returning `NotFound` would push every caller into
/// writing the same fallback.
pub fn read_profile<S: ProfileStore>(store: &S, profile_id: Uuid) -> ProfileResult<SoulProfile> {
    match store.get_profile(profile_id) {
        Ok(profile) => Ok(profile),
        Err(StoreError::NotFound { .. }) => Ok(blank_profile(profile_id)),
        Err(error) => Err(error.into()),
    }
}

/// The voice WP10 drafts with: the user's values wherever they set one.
pub fn read_voice<S: ProfileStore>(store: &S, profile_id: Uuid) -> ProfileResult<VoiceProfile> {
    Ok(VoiceProfile::from_value(
        &read_profile(store, profile_id)?.voice,
    ))
}

/// Five axes at `unknown`, a neutral voice, and nothing claimed.
pub fn blank_profile(profile_id: Uuid) -> SoulProfile {
    SoulProfile {
        schema_version: SchemaVersion,
        profile_id,
        voice: VoiceProfile::default().to_value(),
        trait_axes: blank_axes(),
        values: None,
        boundaries: None,
        clinical_claim: NotAClinicalClaim,
    }
}

/// Turn a completed questionnaire into a profile backed by evidence.
///
/// This is the whole of the v0.1 questionnaire, both paths: the user who
/// imported nothing and the user who imported a file with nothing in it end up
/// here, answering the same eleven questions once.
/// [`soul_import::questionnaire::record`] writes each answer as an event with
/// the words sealed and a `user_stated` evidence row, hands the ids to
/// [`ProfileSink`], and this function applies what the sink made of them. The
/// axes cite the recorder's evidence rather than a second copy of it, which is
/// what stops the two crates from holding two accounts of the same answer.
///
/// Voice answers are treated as the user speaking and pin the field; axis
/// answers are not locked, because an axis is a working hypothesis that later
/// evidence is allowed to refine. Locking an axis is what [`correct_axis`] is
/// for, and the difference is deliberate: voice is an instruction the agent
/// layer obeys, an axis is a belief the soul layer holds.
pub fn intake<S>(
    store: &mut S,
    profile_id: Uuid,
    response: &QuestionnaireResponse,
    now_unix_seconds: i64,
) -> ProfileResult<IntakeOutcome>
where
    S: ProfileStore + AuditLog + EventStore + BlobStore,
{
    let answers = questionnaire::check(response)?;
    let to_record: Vec<recorder::Answer> = answers.iter().map(|a| a.to_recorded()).collect();

    let mut sink = ProfileSink::new();
    let receipt = recorder::record(
        store,
        &to_record,
        &Timestamp::new(&response.answered_at),
        &mut sink,
    )?;

    // Every question left blank. Nothing was written, so there is nothing to
    // apply and nothing to audit — and a profile of five `unknown` axes is not
    // a completed questionnaire.
    if sink.is_empty() {
        return Err(ProfileError::EmptyQuestionnaire);
    }

    let mut profile = read_profile(store, profile_id)?;
    let mut voice = VoiceProfile::from_value(&profile.voice);
    let mut evidence_ids = Vec::with_capacity(sink.accepted().len());
    let mut event_ids = Vec::with_capacity(sink.accepted().len());

    for staged in sink.accepted() {
        evidence_ids.push(staged.evidence_id);
        event_ids.push(staged.event_id);

        match (staged.target, staged.value) {
            (QuestionTarget::Axis(axis), StagedValue::Position(position)) => place_axis(
                &mut profile,
                &axis,
                position,
                band_of(QUESTIONNAIRE_STRENGTH),
                vec![staged.evidence_id],
                None,
            ),
            (QuestionTarget::Voice(_), StagedValue::Voice(setting)) => voice.set_by_user(setting),
            (QuestionTarget::Stated(field), StagedValue::Stated) => {
                place_stated(&mut profile, field, staged)
            }
            // The sink builds these pairs and refuses the ones that do not
            // match, so this arm is unreachable by construction. Skipping is
            // the safe reading of an impossible answer: it leaves the field
            // alone rather than putting a guess in the profile.
            _ => continue,
        }
    }

    profile.voice = voice.to_value();
    let profile = write_profile(store, profile)?;

    // The questionnaire is the no-file branch of the same intake the importer
    // takes, so it records under the same action, and the entry the recorder
    // built is the one that lands — with the profile named on it, because this
    // is the run that made a profile out of the answers. `counts.items` is the
    // number of answers, which is a count and not a word of what was answered.
    for content in receipt.audit {
        append_or_store_error(store, content.about(&[profile_id]), now_unix_seconds)?;
    }

    Ok(IntakeOutcome {
        profile,
        evidence_ids,
        event_ids,
    })
}

/// Record the user's correction and pin the axis against later inference.
pub fn correct_axis<S>(
    store: &mut S,
    profile_id: Uuid,
    axis_id: Uuid,
    position: AxisPosition,
    now_unix_seconds: i64,
) -> ProfileResult<SoulProfile>
where
    S: ProfileStore + AuditLog,
{
    let axis = axes::axis_by_id(axis_id).ok_or(ProfileError::UnknownAxis(axis_id))?;

    let evidence_id = Uuid::now_v7();
    store.put_evidence(SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id,
        kind: EvidenceKind::UserCorrection,
        subject: Subject::Owner,
        source_refs: vec![json!({
            "origin": "profile_correction",
            "axis_id": axis_id,
            "position": axes::position_key(position),
        })],
        strength: CORRECTION_STRENGTH,
        method: Some(EvidenceMethod::UserStated),
        exportable_to_research: Some(false),
        privacy: Some(owner_privacy()),
    })?;

    let mut profile = read_profile(store, profile_id)?;
    place_axis(
        &mut profile,
        &axis,
        position,
        band_of(CORRECTION_STRENGTH),
        vec![evidence_id],
        Some(true),
    );
    let profile = write_profile(store, profile)?;

    append_or_store_error(
        store,
        AuditContent::new(AuditAction::ProfileCorrect, AuditDecision::Allowed)
            .because(ReasonCode::Routine)
            .about(&[profile_id, axis_id, evidence_id]),
        now_unix_seconds,
    )?;

    Ok(profile)
}

/// Set a voice field because the user said so. WP10 reads this back.
pub fn set_voice<S>(
    store: &mut S,
    profile_id: Uuid,
    setting: VoiceSetting,
    now_unix_seconds: i64,
) -> ProfileResult<VoiceProfile>
where
    S: ProfileStore + AuditLog,
{
    let mut profile = read_profile(store, profile_id)?;
    let mut voice = VoiceProfile::from_value(&profile.voice);
    voice.set_by_user(setting);
    profile.voice = voice.to_value();
    write_profile(store, profile)?;

    append_or_store_error(
        store,
        AuditContent::new(AuditAction::ProfileCorrect, AuditDecision::Allowed)
            .because(ReasonCode::Routine)
            .about(&[profile_id]),
        now_unix_seconds,
    )?;

    Ok(voice)
}

/// Propose a voice field from something observed rather than stated.
///
/// Returns `false` and changes nothing when the user has already set the
/// field. The refusal is the point of the function.
pub fn suggest_voice<S>(
    store: &mut S,
    profile_id: Uuid,
    setting: VoiceSetting,
    now_unix_seconds: i64,
) -> ProfileResult<bool>
where
    S: ProfileStore + AuditLog,
{
    let mut profile = read_profile(store, profile_id)?;
    let mut voice = VoiceProfile::from_value(&profile.voice);
    if !voice.suggest(setting) {
        return Ok(false);
    }
    profile.voice = voice.to_value();
    write_profile(store, profile)?;

    append_or_store_error(
        store,
        AuditContent::new(AuditAction::InferenceWrite, AuditDecision::Allowed)
            .because(ReasonCode::Routine)
            .about(&[profile_id]),
        now_unix_seconds,
    )?;

    Ok(true)
}

/// Store an inference about an axis, and apply it unless the user has spoken.
pub fn record_axis_inference<S>(
    store: &mut S,
    profile_id: Uuid,
    proposal: AxisProposal,
    now_unix_seconds: i64,
) -> ProfileResult<InferenceOutcome>
where
    S: ProfileStore + AuditLog,
{
    let axis =
        axes::axis_by_id(proposal.axis_id).ok_or(ProfileError::UnknownAxis(proposal.axis_id))?;

    // Refused before anything is written. The store refuses this too, but by
    // then the caller has already been told the write began.
    if proposal.evidence_ids.is_empty() {
        return Err(ProfileError::NoEvidence);
    }

    let statement_key = axis.statement_key(proposal.position);
    assert_non_clinical(&statement_key)?;
    if let Some(falsifier) = proposal.falsifier.as_deref() {
        assert_non_clinical(falsifier)?;
    }

    let inference_id = Uuid::now_v7();
    store.put_inference(SoulInference {
        schema_version: SchemaVersion,
        inference_id,
        target: json!({ "kind": "trait_axis", "axis_id": proposal.axis_id }),
        statement_key,
        evidence_ids: proposal.evidence_ids.clone(),
        evidence_band: proposal.band,
        method: Some(proposal.method),
        user_verdict: Some(UserVerdict::Unreviewed),
        clinical_claim: NotAClinicalClaim,
        falsifier: proposal.falsifier.clone(),
    })?;

    let mut profile = read_profile(store, profile_id)?;
    let update = match axis_is_locked(&profile, proposal.axis_id) {
        true => AxisUpdate::RefusedAxisLocked,
        false => {
            place_axis(
                &mut profile,
                &axis,
                proposal.position,
                band_of(proposal.band),
                proposal.evidence_ids.clone(),
                Some(false),
            );
            write_profile(store, profile)?;
            AxisUpdate::Applied
        }
    };

    append_or_store_error(
        store,
        AuditContent::new(AuditAction::InferenceWrite, AuditDecision::Allowed)
            .because(ReasonCode::Routine)
            .about(&[profile_id, proposal.axis_id, inference_id]),
        now_unix_seconds,
    )?;

    Ok(InferenceOutcome {
        inference_id,
        update,
    })
}

/// Whether the user has pinned this axis.
pub fn axis_is_locked(profile: &SoulProfile, axis_id: Uuid) -> bool {
    profile
        .trait_axes
        .iter()
        .find(|axis| axis.axis_id == axis_id)
        .and_then(|axis| axis.locked_by_user)
        .unwrap_or(false)
}

// ------------------------------------------------------------- internals ---

fn owner_privacy() -> Privacy {
    Privacy::local_only(Subject::Owner, vec![Purpose::SoulProfile])
}

fn band_of(band: SupportedBand) -> EvidenceBand {
    match band {
        SupportedBand::Weak => EvidenceBand::Weak,
        SupportedBand::Moderate => EvidenceBand::Moderate,
        SupportedBand::Strong => EvidenceBand::Strong,
    }
}

/// Record that the user stated a boundary or a value, as a pointer.
///
/// `profile.values` and `profile.boundaries` are free-form arrays in
/// `profile.schema.json`, which is exactly why nothing readable goes in them:
/// the words are sealed in the event the recorder wrote, and what lands here
/// is the question, the event and the evidence. One entry per question —
/// answering the same question again replaces the old pointer rather than
/// stacking a second one, the same rule the axes follow.
fn place_stated(profile: &mut SoulProfile, field: StatedField, staged: &StagedAnswer) {
    let entry = json!({
        "origin": "questionnaire",
        "question_id": staged.question_id,
        "event_id": staged.event_id,
        "evidence_id": staged.evidence_id,
    });

    let held = match field {
        StatedField::Boundary => &mut profile.boundaries,
        StatedField::Value => &mut profile.values,
    };
    let entries = held.get_or_insert_with(Vec::new);
    match entries
        .iter_mut()
        .find(|held| names_question(held, staged.question_id))
    {
        Some(existing) => *existing = entry,
        None => entries.push(entry),
    }
}

fn names_question(entry: &Value, question_id: &str) -> bool {
    entry.get("question_id").and_then(Value::as_str) == Some(question_id)
}

/// Move one axis, leaving the others alone.
///
/// `evidence_ids` replaces rather than accumulates: the list says what supports
/// the position the axis is in *now*. Superseded answers keep their rows in the
/// evidence table and their entries in the audit chain, so nothing is lost — it
/// simply stops being cited as support for a claim it no longer supports.
fn place_axis(
    profile: &mut SoulProfile,
    axis: &AxisDefinition,
    position: AxisPosition,
    band: EvidenceBand,
    evidence_ids: Vec<Uuid>,
    locked_by_user: Option<bool>,
) {
    if !profile
        .trait_axes
        .iter()
        .any(|candidate| candidate.axis_id == axis.axis_id)
    {
        profile.trait_axes.push(axis.blank());
    }

    let Some(target) = profile
        .trait_axes
        .iter_mut()
        .find(|candidate| candidate.axis_id == axis.axis_id)
    else {
        return;
    };

    target.label = Some(axis.label.to_owned());
    target.position = position;
    target.evidence_band = band;
    target.evidence_ids = Some(evidence_ids);
    if let Some(locked) = locked_by_user {
        target.locked_by_user = Some(locked);
    }
    target.clinical_claim = NotAClinicalClaim;
}

/// The one write path. Everything a profile promises is checked here.
fn write_profile<S: ProfileStore>(
    store: &mut S,
    profile: SoulProfile,
) -> ProfileResult<SoulProfile> {
    reject_numeric_rating(&profile.trait_axes)?;
    for axis in &profile.trait_axes {
        if let Some(label) = axis.label.as_deref() {
            assert_non_clinical(label)?;
        }
    }
    store.put_profile(profile.clone())?;
    Ok(profile)
}

/// The axes of a profile, in the fixed order the defaults are declared in.
pub fn ordered_axes(profile: &SoulProfile) -> Vec<TraitAxis> {
    let mut ordered: Vec<TraitAxis> = axes::DEFAULT_AXES
        .iter()
        .filter_map(|definition| {
            profile
                .trait_axes
                .iter()
                .find(|axis| axis.axis_id == definition.axis_id)
                .cloned()
        })
        .collect();
    for axis in &profile.trait_axes {
        if axes::axis_by_id(axis.axis_id).is_none() {
            ordered.push(axis.clone());
        }
    }
    ordered
}
