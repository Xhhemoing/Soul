//! The user's voice, as a deterministic template and as one line of material.
//!
//! Two things happen here and they are deliberately the same four words:
//!
//! * [`template_draft`] is what the product produces when no endpoint is
//!   configured. It is the default shape of a draft, not a fallback branch —
//!   nothing about it is degraded, and nothing about it reaches the network.
//! * [`tone_turn`] is how the voice reaches a request body when an endpoint
//!   *is* configured. It is a turn of the user's own, built from this module's
//!   constants and the four closed enums, because the instruction slot of an
//!   E1 request is a constant in `soul_policy::e1` and this crate does not get
//!   to append to it.
//!
//! What is not here is any state. The voice is read from the profile on every
//! draft, so a value the user set is in effect on the next one; a cached
//! `VoiceProfile` is the bug AC-07 is written to catch.

use uuid::Uuid;

use soul_policy::clinical::assert_non_clinical;
use soul_policy::redactor::{RedactedBody, Turn};
use soul_profile::voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth};
use soul_schema::common::SealedSubject;

use crate::error::{DraftError, DraftResult};

/// Where the text of a draft came from.
///
/// Recorded so a local template is never presented as a model's answer. There
/// is no third variant for "half of each".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftRoute {
    /// One generation request against the endpoint the user configured.
    E1,
    /// The deterministic template. No bytes left the machine.
    Template,
}

impl DraftRoute {
    pub const fn as_str(self) -> &'static str {
        match self {
            DraftRoute::E1 => "e1",
            DraftRoute::Template => "template",
        }
    }
}

/// The counting side of one draft: how much material there was, and how much
/// of it was placeheld.
///
/// Taken from the [`RedactedBody`] before it is moved into the request, so the
/// numbers describe the body that actually went rather than a second redaction
/// nobody checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraftStats {
    turns: usize,
    third_party_turns: usize,
    placeheld_turns: usize,
    carries_exempted_original: bool,
}

impl DraftStats {
    pub fn of(body: &RedactedBody, turns: usize) -> DraftStats {
        DraftStats {
            turns,
            third_party_turns: body.third_party_turns(),
            placeheld_turns: body.placeheld_turns(),
            carries_exempted_original: body.carries_exempted_original(),
        }
    }

    pub fn turns(&self) -> usize {
        self.turns
    }

    pub fn third_party_turns(&self) -> usize {
        self.third_party_turns
    }

    pub fn placeheld_turns(&self) -> usize {
        self.placeheld_turns
    }

    pub fn carries_exempted_original(&self) -> bool {
        self.carries_exempted_original
    }
}

/// A draft, and nothing that could act on it.
///
/// The public surface is text plus counts. There is no method that hands the
/// text to a transport, and no field that says a draft is ready to go out:
/// this product drafts, and the person decides what happens next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftOutcome {
    text: String,
    route: DraftRoute,
    stats: DraftStats,
}

impl DraftOutcome {
    /// Build the outcome, refusing text that makes a medical claim.
    ///
    /// The check runs on both routes. A template that trips it is a bug in
    /// this module; an endpoint answer that trips it is a bug in whatever
    /// answered, and neither is something to strip a word out of and keep.
    pub fn new(text: String, route: DraftRoute, stats: DraftStats) -> DraftResult<DraftOutcome> {
        assert_non_clinical(&text)?;
        Ok(DraftOutcome { text, route, stats })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn route(&self) -> DraftRoute {
        self.route
    }

    pub fn stats(&self) -> DraftStats {
        self.stats
    }

    pub fn placeheld_turns(&self) -> usize {
        self.stats.placeheld_turns
    }

    pub fn carries_exempted_original(&self) -> bool {
        self.stats.carries_exempted_original
    }
}

/// Opening line, by register.
const OPENING_CASUAL: &str = "随手回一句：";
const OPENING_PLAIN: &str = "回复如下：";
const OPENING_FORMAL: &str = "谨拟回复如下：";

/// How the draft orders what it says, by directness.
const ORDER_RESERVED: &str = "先确认几处细节，再给答复。";
const ORDER_BALANCED: &str = "先说我的理解，再说下一步。";
const ORDER_DIRECT: &str = "先给结论，再补背景。";

/// How the draft closes, by warmth.
const CLOSING_COOL: &str = "就事论事，不多寒暄。";
const CLOSING_EVEN: &str = "有需要随时说。";
const CLOSING_WARM: &str = "辛苦你了，有事随时找我。";

/// What a draft appends, by emoji preference.
const MARK_NEVER: &str = "";
const MARK_SPARING: &str = "🙂";
const MARK_FREQUENT: &str = "🙂✨🙂";

/// Heading above the material the draft was written against.
const MATERIAL_HEADING: &str = "参考素材（第三人正文与标识已按规则占位）：";
/// First line of every template draft.
const DRAFT_HEADING: &str = "以下是按你的语气拟好的回复草稿，用不用由你决定：";
/// Prefix of the one line that carries the voice into a request body.
const TONE_PREFIX: &str = "我的语气偏好：";

const fn register_word(register: VoiceRegister) -> &'static str {
    match register {
        VoiceRegister::Casual => "随意",
        VoiceRegister::Plain => "平实",
        VoiceRegister::Formal => "正式",
    }
}

const fn directness_word(directness: VoiceDirectness) -> &'static str {
    match directness {
        VoiceDirectness::Reserved => "含蓄",
        VoiceDirectness::Balanced => "适中",
        VoiceDirectness::Direct => "直接",
    }
}

const fn warmth_word(warmth: VoiceWarmth) -> &'static str {
    match warmth {
        VoiceWarmth::Cool => "克制",
        VoiceWarmth::Even => "平和",
        VoiceWarmth::Warm => "热络",
    }
}

const fn emoji_word(emoji: EmojiUse) -> &'static str {
    match emoji {
        EmojiUse::Never => "不用表情",
        EmojiUse::Sparing => "偶尔用表情",
        EmojiUse::Frequent => "经常用表情",
    }
}

const fn opening(register: VoiceRegister) -> &'static str {
    match register {
        VoiceRegister::Casual => OPENING_CASUAL,
        VoiceRegister::Plain => OPENING_PLAIN,
        VoiceRegister::Formal => OPENING_FORMAL,
    }
}

const fn order(directness: VoiceDirectness) -> &'static str {
    match directness {
        VoiceDirectness::Reserved => ORDER_RESERVED,
        VoiceDirectness::Balanced => ORDER_BALANCED,
        VoiceDirectness::Direct => ORDER_DIRECT,
    }
}

const fn closing(warmth: VoiceWarmth) -> &'static str {
    match warmth {
        VoiceWarmth::Cool => CLOSING_COOL,
        VoiceWarmth::Even => CLOSING_EVEN,
        VoiceWarmth::Warm => CLOSING_WARM,
    }
}

const fn mark(emoji: EmojiUse) -> &'static str {
    match emoji {
        EmojiUse::Never => MARK_NEVER,
        EmojiUse::Sparing => MARK_SPARING,
        EmojiUse::Frequent => MARK_FREQUENT,
    }
}

/// The voice in words, from the four closed enums and nothing else.
///
/// Not `VoiceProfile`'s JSON: that carries `user_set`, which is an internal
/// lock list and has no business in text the product shows or sends.
pub fn tone_directive(voice: &VoiceProfile) -> String {
    format!(
        "{TONE_PREFIX}{}、{}、{}、{}",
        register_word(voice.register),
        directness_word(voice.directness),
        warmth_word(voice.warmth),
        emoji_word(voice.emoji_use),
    )
}

/// The voice as a turn of the user's own.
///
/// `SealedSubject::Owner`, because it is the user describing how they write.
/// This is the only way the voice reaches a request body: the instruction slot
/// belongs to `soul_policy::e1::DRAFTING_INSTRUCTION`, byte for byte, and
/// widening that is a change to `soul-policy`, not something to work around
/// from here.
pub fn tone_turn(voice: &VoiceProfile) -> Turn {
    Turn::new(Uuid::now_v7(), SealedSubject::Owner, tone_directive(voice))
}

/// The voice as plan fields, for the hash the user's approval is taken over.
///
/// Four strings from four closed enums. `user_set` is absent by construction.
pub fn voice_plan(voice: &VoiceProfile) -> serde_json::Value {
    serde_json::json!({
        "register": register_word(voice.register),
        "directness": directness_word(voice.directness),
        "warmth": warmth_word(voice.warmth),
        "emoji_use": emoji_word(voice.emoji_use),
    })
}

/// A draft written on this machine, in the user's voice.
///
/// Deterministic: the same voice and the same redacted material produce the
/// same bytes. Every one of the four fields changes the output, so a constant
/// string cannot pass for a template.
pub fn template_draft(voice: &VoiceProfile, body: &RedactedBody) -> String {
    format!(
        "{DRAFT_HEADING}\n{}\n{}\n{MATERIAL_HEADING}\n{}\n{}{}",
        opening(voice.register),
        order(voice.directness),
        body.as_str(),
        closing(voice.warmth),
        mark(voice.emoji_use),
    )
}

/// Check one rendered draft the way the outcome constructor does.
///
/// Exposed so a caller can assert over a template it rendered directly,
/// without having to build a [`DraftOutcome`] to find out.
pub fn check_draft(text: &str) -> Result<(), DraftError> {
    Ok(assert_non_clinical(text)?)
}
