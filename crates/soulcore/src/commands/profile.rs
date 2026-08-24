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

use uuid::Uuid;

use soul_profile::questionnaire::QuestionnaireResponse;
use soul_profile::{
    AxisProposal, InferenceOutcome, IntakeOutcome, ProfileError, ProfileView, VoiceProfile,
    VoiceSetting,
};
use soul_schema::profile::{AxisPosition, SoulProfile};
use soul_store::SqlCipherStore;

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
