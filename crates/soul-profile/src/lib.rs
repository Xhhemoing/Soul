//! Soul's soul profile: what the product believes about the user, and what the
//! user has told it to believe instead.
//!
//! Five direction axes ([`axes`]), a voice the agent layer drafts with
//! ([`voice`]), the no-import questionnaire that fills both in
//! ([`questionnaire`]), the write path that enforces the promises
//! ([`service`]), and the read model that traces every claim back to evidence
//! ([`view`]).
//!
//! Four things this crate refuses to do, each one a line in
//! `docs/PRODUCT_LOCK.md`:
//!
//! * store an inference with no evidence, or with evidence that does not
//!   resolve;
//! * overwrite an axis the user has corrected, or a voice field the user has
//!   set;
//! * put a number on a trait axis;
//! * emit a sentence carrying diagnostic vocabulary.
//!
//! There is no database here and no network. Everything goes through the
//! `soul-store-api` traits, so the same code runs against `FakeStore` in a unit
//! test and against SQLCipher in `soulcore`.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod axes;
pub mod error;
pub mod numeric;
pub mod questionnaire;
pub mod service;
pub mod view;
pub mod voice;

pub use axes::{
    axis_by_id, axis_by_key, blank_axes, AxisDefinition, ACCOMMODATION, CURIOSITY, DEFAULT_AXES,
    EMOTIONAL_STEADINESS, ORDERLINESS, SOCIAL_ENERGY,
};
pub use error::{ProfileError, ProfileResult};
pub use numeric::{reject_numeric_rating, reject_numeric_rating_value, NumericRating};
pub use questionnaire::{
    questionnaire, Answer, CheckedAnswer, Question, QuestionTarget, QuestionnaireResponse,
};
pub use service::{
    axis_is_locked, blank_profile, correct_axis, intake, read_profile, read_voice,
    record_axis_inference, set_voice, suggest_voice, AxisProposal, AxisUpdate, InferenceOutcome,
    IntakeOutcome,
};
pub use view::{profile_view, render, render_voice, AxisView, InferenceView, ProfileView};
pub use voice::{
    EmojiUse, VoiceDirectness, VoiceField, VoiceProfile, VoiceRegister, VoiceSetting, VoiceWarmth,
};
