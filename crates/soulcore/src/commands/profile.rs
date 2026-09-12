//! WP03's command surface: the questionnaire, corrections, and the read the
//! rest of the product does against the profile.
//!
//! Thin, like the rest of this module. `soul-profile` decides what an axis is
//! and what a correction locks; `soul-store` holds it. What is here is the
//! wiring, and one thing worth naming: every function takes the caller's clock
//! as `at_unix_seconds` rather than reading it, so a replay writes the same
//! audit entries as the run it is replaying.
//!
//! [`voice`] is the entry point WP10 uses. It is a separate function rather
//! than a field of the profile read because drafting needs the voice and not
//! the axes, and handing a draft prompt the whole profile would make it easy to
//! put a working hypothesis about the user into text the user is about to send
//! someone else.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use soul_profile::axes::{self, position_key};
use soul_profile::questionnaire::{Answer, QuestionTarget, QuestionnaireResponse, StatedField};
use soul_profile::{
    AxisProposal, InferenceOutcome, IntakeOutcome, ProfileError, ProfileView, VoiceField,
    VoiceProfile, VoiceSetting,
};
use soul_schema::profile::{AxisPosition, SoulProfile};
use soul_store::SqlCipherStore;

/// 工作假设，非临床结论, re-exported so the profile screen reads the same
/// constant `soul-policy`'s denylist tests hold.
pub use soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE;

/// The fixed questionnaire, for a UI that has to draw it: every question with
/// the profile field it moves and the options to offer.
pub fn questions() -> Vec<soul_profile::Question> {
    soul_profile::questionnaire()
}

/// Turn a completed questionnaire into a profile. AC-03.
///
/// This is the whole questionnaire, both ways in. A user who imported nothing
/// is asked the same questions as a user whose export turned out to be empty,
/// and each answer is written once — an event with the words sealed, a
/// `user_stated` evidence row, and the profile field it moves — because
/// `soul-profile` is the sink `soul-import`'s recorder hands answers to.
pub fn intake(
    store: &mut SqlCipherStore,
    profile_id: Uuid,
    response: &QuestionnaireResponse,
    at_unix_seconds: i64,
) -> Result<IntakeOutcome, ProfileError> {
    soul_profile::intake(store, profile_id, response, at_unix_seconds)
}

/// The profile as stored, or a blank one if the user has not started.
pub fn read(store: &SqlCipherStore, profile_id: Uuid) -> Result<SoulProfile, ProfileError> {
    soul_profile::read_profile(store, profile_id)
}

/// The profile with every cited evidence row resolved. AC-06.
///
/// Fails rather than returning a shorter list when an id does not come back: an
/// axis or an inference whose evidence has gone is making a claim nothing
/// supports, and the user asking "why do you think that" deserves an error over
/// a blank.
pub fn view(store: &SqlCipherStore, profile_id: Uuid) -> Result<ProfileView, ProfileError> {
    soul_profile::profile_view(store, profile_id)
}

/// The profile in prose, denylist-checked and carrying the working-hypothesis
/// notice.
pub fn render(store: &SqlCipherStore, profile_id: Uuid) -> Result<String, ProfileError> {
    soul_profile::render(&view(store, profile_id)?)
}

/// The voice WP10 drafts in: the user's values wherever they set one.
pub fn voice(store: &SqlCipherStore, profile_id: Uuid) -> Result<VoiceProfile, ProfileError> {
    soul_profile::read_voice(store, profile_id)
}

/// Record a correction and pin the axis against later inference. AC-07.
pub fn correct_axis(
    store: &mut SqlCipherStore,
    profile_id: Uuid,
    axis_id: Uuid,
    position: AxisPosition,
    at_unix_seconds: i64,
) -> Result<SoulProfile, ProfileError> {
    soul_profile::correct_axis(store, profile_id, axis_id, position, at_unix_seconds)
}

/// Set a voice field because the user said so.
pub fn set_voice(
    store: &mut SqlCipherStore,
    profile_id: Uuid,
    setting: VoiceSetting,
    at_unix_seconds: i64,
) -> Result<VoiceProfile, ProfileError> {
    soul_profile::set_voice(store, profile_id, setting, at_unix_seconds)
}

/// Propose a voice field from something observed. Returns `false` and changes
/// nothing when the user has already set that field.
pub fn suggest_voice(
    store: &mut SqlCipherStore,
    profile_id: Uuid,
    setting: VoiceSetting,
    at_unix_seconds: i64,
) -> Result<bool, ProfileError> {
    soul_profile::suggest_voice(store, profile_id, setting, at_unix_seconds)
}

/// Store an inference about an axis, and apply it unless the user has spoken.
///
/// A proposal with no evidence is refused before anything is written.
pub fn record_inference(
    store: &mut SqlCipherStore,
    profile_id: Uuid,
    proposal: AxisProposal,
    at_unix_seconds: i64,
) -> Result<InferenceOutcome, ProfileError> {
    soul_profile::record_axis_inference(store, profile_id, proposal, at_unix_seconds)
}

// ------------------------------------------------- what a screen may draw ---
//
// The types below are the same kind of value `graph.rs` hands the shell:
// identifiers, closed vocabulary words and counts. Two of them are worth
// naming. [`StatedRow`] carries no text — the user's own words about their
// boundaries are sealed in the event the recorder wrote, and this surface
// hands over the pointer rather than opening it. [`AxisRow::inferences`]
// keeps the inferences a lock refused, because AC-07 is only visible if the
// user can see the machine still disagrees.

/// One question, as the wizard draws it.
///
/// Each option carries both the recorder's token and the words for it. The
/// token is what goes back — `soul-import` declares the closed set so it can
/// refuse an option nobody offered — and the words come from the same place
/// the profile screen gets them, so the wizard is not a second spelling of
/// twelve phrases the denylist would have to be pointed at twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionView {
    pub question_id: String,
    pub prompt: String,
    /// `axis`, `voice`, `boundary` or `value`: which part of the profile an
    /// answer moves.
    pub moves: String,
    pub options: Vec<QuestionOption>,
    /// True when the answer is the user's own words rather than an option.
    /// Prose is sealed by the recorder and never comes back to a screen.
    pub prose: bool,
}

/// One answer a question offers: the token, and what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionOption {
    pub value: String,
    pub reading: String,
}

/// The eleven questions, in the order they are asked.
pub fn question_views() -> Vec<QuestionView> {
    questions()
        .into_iter()
        .map(|question| QuestionView {
            question_id: question.question_id.to_owned(),
            prompt: question.prompt.to_owned(),
            moves: target_word(question.target).to_owned(),
            options: question
                .options()
                .iter()
                .map(|option| QuestionOption {
                    value: (*option).to_owned(),
                    reading: option_reading(question.target, option),
                })
                .collect(),
            prose: question.options().is_empty(),
        })
        .collect()
}

/// What one option key says, in the vocabulary its own target owns.
///
/// An axis option is a direction on that axis and a voice option is one of
/// `soul-profile`'s twelve words; neither means anything as a bare token.
/// Falling back to the token would be a screen showing `leans_high`, which is
/// worse than an error and quieter, so an option that resolves to nothing
/// keeps its key and the questionnaire tests are what catch it.
fn option_reading(target: QuestionTarget, option: &str) -> String {
    let reading = match target {
        QuestionTarget::Axis(axis) => {
            axes::position_by_key(option).map(|position| axis.direction(position).to_owned())
        }
        QuestionTarget::Voice(field) => VoiceSetting::from_option(field, option)
            .map(|setting| soul_profile::setting_word(setting).to_owned()),
        QuestionTarget::Stated(_) => None,
    };
    reading.unwrap_or_else(|| option.to_owned())
}

fn target_word(target: QuestionTarget) -> &'static str {
    match target {
        QuestionTarget::Axis(_) => "axis",
        QuestionTarget::Voice(_) => "voice",
        QuestionTarget::Stated(StatedField::Boundary) => "boundary",
        QuestionTarget::Stated(StatedField::Value) => "value",
    }
}

/// What the user gave, against one question.
///
/// `given` is an option key for a choice question and the user's words for a
/// text box. Blank means the question was skipped, and a skipped question
/// leaves no row at all: not answering is how a user says they do not know,
/// and the axis stays `unknown` rather than being guessed at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GivenAnswer {
    pub question_id: String,
    pub given: String,
}

/// Turn what a wizard collected into the response [`intake`] takes.
///
/// Blank answers are dropped here rather than refused: a partly filled
/// questionnaire is a legitimate thing for a user to hand in.
pub fn response_of(
    answers: &[GivenAnswer],
    answered_at: &str,
) -> Result<QuestionnaireResponse, ProfileError> {
    let mut checked = Vec::with_capacity(answers.len());
    for answer in answers {
        if answer.given.trim().is_empty() {
            continue;
        }
        checked.push(Answer::for_question(&answer.question_id, &answer.given)?);
    }
    Ok(QuestionnaireResponse::new(answered_at, checked))
}

/// One answer the intake recorded and did not apply, and why.
///
/// Two tokens and no content. The question is named because the user is
/// entitled to know which of their answers did not land, and the reason is a
/// machine word — `axis_locked_by_user` — rather than a sentence, because the
/// words a user reads are the interface's to choose and `soul-profile` already
/// owns the only spelling of the reason itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IgnoredAnswer {
    pub question_id: String,
    pub reason: String,
}

/// What one questionnaire run left behind.
///
/// Counts and identifiers. `axes_unknown` is the interesting one: it is how
/// many axes the user did not answer for, and they stay `unknown` rather than
/// being filled in from the answers that were given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntakeReceipt {
    /// Answers that moved something. An answer the user's own correction kept
    /// out is in [`IntakeReceipt::ignored`] and is not counted here: a receipt
    /// that said "recorded 3" about a run that changed two axes and refused
    /// one would be the polite version of not mentioning the refusal (D39).
    pub answered: usize,
    pub axes_known: usize,
    pub axes_unknown: usize,
    pub voice_fields_user_set: usize,
    pub stated_entries: usize,
    /// False once anything in the profile came from the user. AC-03 is this
    /// field being false after a questionnaire and no import.
    pub profile_is_empty: bool,
    /// One per recorded answer, refused ones included: the row exists either
    /// way, because the user answered the question. Every one of them is
    /// `user_stated`.
    pub evidence_ids: Vec<String>,
    /// The answers that reached an axis the user had already corrected, and
    /// were recorded without moving it. Empty on a first run.
    pub ignored: Vec<IgnoredAnswer>,
}

impl IntakeReceipt {
    fn of(outcome: &IntakeOutcome) -> IntakeReceipt {
        let voice = VoiceProfile::from_value(&outcome.profile.voice);
        let known = outcome
            .profile
            .trait_axes
            .iter()
            .filter(|axis| axis.position != AxisPosition::Unknown)
            .count();
        let stated = stated_count(&outcome.profile);
        let ignored: Vec<IgnoredAnswer> = outcome
            .ignored
            .iter()
            .map(|answer| IgnoredAnswer {
                question_id: answer.question_id.to_owned(),
                reason: answer.reason.as_str().to_owned(),
            })
            .collect();
        IntakeReceipt {
            answered: outcome.evidence_ids.len().saturating_sub(ignored.len()),
            axes_known: known,
            axes_unknown: outcome.profile.trait_axes.len() - known,
            voice_fields_user_set: voice.user_set.len(),
            stated_entries: stated,
            profile_is_empty: known == 0 && voice.user_set.is_empty() && stated == 0,
            evidence_ids: outcome.evidence_ids.iter().map(Uuid::to_string).collect(),
            ignored,
        }
    }
}

fn stated_count(profile: &SoulProfile) -> usize {
    profile.boundaries.as_ref().map_or(0, Vec::len) + profile.values.as_ref().map_or(0, Vec::len)
}

/// Record a completed questionnaire and describe what it produced.
pub fn intake_from(
    store: &mut SqlCipherStore,
    profile_id: Uuid,
    answers: &[GivenAnswer],
    answered_at: &str,
    at_unix_seconds: i64,
) -> Result<IntakeReceipt, ProfileError> {
    let response = response_of(answers, answered_at)?;
    let outcome = intake(store, profile_id, &response, at_unix_seconds)?;
    Ok(IntakeReceipt::of(&outcome))
}

/// Everything the 灵魂档案 screen shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileScreen {
    pub profile_id: String,
    pub axes: Vec<AxisRow>,
    pub voice: VoiceView,
    /// Boundaries and values, as pointers. No words.
    pub stated: Vec<StatedRow>,
    /// The profile in the core's own prose, denylist-checked before it left.
    pub reading: String,
    /// The positions a correction may choose from.
    pub positions: Vec<String>,
    pub notice: String,
}

/// One trait axis, with what it rests on and whether the user pinned it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisRow {
    pub axis_id: String,
    pub label: String,
    /// `leans_low`, `mixed`, `leans_high` or `unknown`.
    pub position: String,
    /// The axis in words, built on this side where the denylist could see it.
    pub reading: String,
    pub evidence_band: String,
    pub evidence_count: usize,
    pub locked_by_user: bool,
    /// The three positions a correction may choose from, each described for
    /// this axis. `leans_low` means nothing on its own, so the words come
    /// from `AxisDefinition` rather than from a screen guessing at them.
    pub choices: Vec<AxisChoice>,
    /// Every stored inference about this axis, including the ones the lock
    /// refused to apply.
    pub inferences: Vec<InferenceRow>,
}

/// One position an axis can be corrected to, in this axis's own words.
///
/// The direction alone, without the axis's name: the choice is drawn inside
/// the row that already names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisChoice {
    pub position: String,
    pub reading: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRow {
    pub inference_id: String,
    /// The position this inference argues for.
    pub position: String,
    pub band: String,
    /// Never zero: an inference whose evidence does not resolve fails the
    /// whole read rather than arriving with an empty list.
    pub evidence_count: usize,
    /// `live` or `orphaned`.
    pub state: String,
    pub falsifier: Option<String>,
}

/// The voice, field by field, with the lock the correction rule turns on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceView {
    pub fields: Vec<VoiceFieldRow>,
    /// The voice in words, from `soul-profile`.
    pub reading: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceFieldRow {
    /// `register`, `directness`, `warmth` or `emoji_use`.
    pub field: String,
    /// What the field is about, in words.
    pub label: String,
    pub value: String,
    /// True once the user set it by hand. Inference leaves these alone.
    pub locked_by_user: bool,
    pub options: Vec<VoiceOption>,
    /// The question that asks about this field, when the questionnaire asks
    /// about it at all. Warmth has none, and says so rather than pointing at
    /// a question that does not exist.
    pub question_id: Option<String>,
}

/// One value a voice field can take, and the word for it.
///
/// The words come from `soul-profile`, which is where `render_voice` gets
/// them: a second spelling in the interface is a second place for the scale
/// vocabulary D22 rules out to reappear where no Rust test is looking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceOption {
    pub value: String,
    pub reading: String,
}

/// One boundary or value the user stated, as the pointer the profile keeps.
///
/// The words are sealed in the event the recorder wrote. This surface has no
/// field that could hold them, which is why the screen cannot show them by
/// accident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatedRow {
    /// `boundary` or `value`.
    pub field: String,
    pub question_id: String,
    /// The question, so the row reads as something rather than as two ids.
    pub prompt: String,
    pub event_id: String,
    pub evidence_id: String,
}

/// The profile screen, with every citation resolved on the way.
pub fn screen(store: &SqlCipherStore, profile_id: Uuid) -> Result<ProfileScreen, ProfileError> {
    let stored = read(store, profile_id)?;
    let view = view(store, profile_id)?;
    Ok(ProfileScreen {
        profile_id: profile_id.to_string(),
        axes: view.axes.iter().map(axis_row).collect(),
        voice: voice_view(&view.voice),
        stated: stated_rows(&stored),
        reading: soul_profile::render(&view)?,
        positions: CORRECTABLE
            .into_iter()
            .map(|position| position_key(position).to_owned())
            .collect(),
        notice: view.notice.to_owned(),
    })
}

/// The positions a correction may choose from. `unknown` is not among them:
/// it is what an axis nobody answered for already says.
const CORRECTABLE: [AxisPosition; 3] = [
    AxisPosition::LeansLow,
    AxisPosition::Mixed,
    AxisPosition::LeansHigh,
];

fn axis_row(axis: &soul_profile::AxisView) -> AxisRow {
    let definition = axes::axis_by_id(axis.axis_id);
    AxisRow {
        axis_id: axis.axis_id.to_string(),
        label: axis.label.clone(),
        position: position_key(axis.position).to_owned(),
        reading: axis.reading.clone(),
        evidence_band: word(&axis.evidence_band),
        evidence_count: axis.evidence.len(),
        locked_by_user: axis.locked_by_user,
        choices: CORRECTABLE
            .into_iter()
            .map(|position| AxisChoice {
                position: position_key(position).to_owned(),
                reading: match definition {
                    Some(definition) => definition.direction(position).to_owned(),
                    None => position_key(position).to_owned(),
                },
            })
            .collect(),
        inferences: axis
            .inferences
            .iter()
            .map(|held| InferenceRow {
                inference_id: held.inference.inference_id.to_string(),
                position: statement_position(&held.inference.statement_key),
                band: word(&held.inference.evidence_band),
                evidence_count: held.evidence.len(),
                state: word(&held.state),
                falsifier: held.inference.falsifier.clone(),
            })
            .collect(),
    }
}

/// The position out of `trait_axis.<key>.<position>`.
///
/// The statement key is built by `AxisDefinition::statement_key`, so the tail
/// is one of the four positions; anything else came from a row this build did
/// not write, and reporting it as `unknown` is the honest reading.
fn statement_position(statement_key: &str) -> String {
    statement_key
        .rsplit('.')
        .next()
        .and_then(axes::position_by_key)
        .map(|position| position_key(position).to_owned())
        .unwrap_or_else(|| position_key(AxisPosition::Unknown).to_owned())
}

fn voice_view(voice: &VoiceProfile) -> VoiceView {
    VoiceView {
        fields: VoiceField::ALL
            .iter()
            .map(|field| VoiceFieldRow {
                field: field.as_str().to_owned(),
                label: soul_profile::field_word(*field).to_owned(),
                value: voice.get(*field).option_key(),
                locked_by_user: voice.is_locked(*field),
                options: VoiceSetting::all_of(*field)
                    .into_iter()
                    .map(|setting| VoiceOption {
                        value: setting.option_key(),
                        reading: soul_profile::setting_word(setting).to_owned(),
                    })
                    .collect(),
                question_id: soul_profile::questionnaire::voice_question_id(*field)
                    .map(str::to_owned),
            })
            .collect(),
        reading: soul_profile::render_voice(voice),
    }
}

fn stated_rows(profile: &SoulProfile) -> Vec<StatedRow> {
    let mut rows = Vec::new();
    for (field, held) in [
        (StatedField::Boundary, profile.boundaries.as_ref()),
        (StatedField::Value, profile.values.as_ref()),
    ] {
        for entry in held.into_iter().flatten() {
            let question_id = text(entry, "question_id");
            rows.push(StatedRow {
                field: field.as_str().to_owned(),
                prompt: soul_profile::questionnaire::question(&question_id)
                    .map(|question| question.prompt.to_owned())
                    .unwrap_or_default(),
                question_id,
                event_id: text(entry, "event_id"),
                evidence_id: text(entry, "evidence_id"),
            });
        }
    }
    rows
}

fn text(entry: &Value, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The contract's own spelling of a small enum, taken from serde rather than
/// written out a second time. Every one of them serializes to a string, so
/// the fallback is unreachable.
fn word<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

// --------------------------------------- what a screen may name, resolved ---
//
// A screen addresses an axis and a voice field by the strings this surface
// handed it. Resolving one is a lookup against a closed set, and each of these
// returns `None` rather than a refusal: the sentence a user reads belongs to
// the session, which is the layer that knows how to say no.

/// The axis with this identifier, if it is one of the five.
pub fn axis_named(axis_id: &str) -> Option<Uuid> {
    Uuid::parse_str(axis_id)
        .ok()
        .and_then(axes::axis_by_id)
        .map(|axis| axis.axis_id)
}

/// The position an option key names.
///
/// `unknown` is not one a user may choose. It is what an axis nobody answered
/// for already says, and offering it as a correction would turn "I did not
/// answer" into something the user asserted.
pub fn position_named(key: &str) -> Option<AxisPosition> {
    axes::position_by_key(key).filter(|position| *position != AxisPosition::Unknown)
}

/// The voice setting a field name and an option key name together.
pub fn voice_setting_named(field: &str, option: &str) -> Option<VoiceSetting> {
    let field = VoiceField::ALL
        .iter()
        .copied()
        .find(|candidate| candidate.as_str() == field)?;
    VoiceSetting::from_option(field, option)
}
