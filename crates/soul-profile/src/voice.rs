//! How the user sounds, and who is allowed to change it.
//!
//! Voice is the half of the profile the agent layer *acts* on: WP10 drafts
//! replies from it. That makes the correction rule sharper here than on the
//! trait axes. An axis is a working hypothesis that new evidence may refine; a
//! voice field the user has set is an instruction, and [`VoiceProfile::suggest`]
//! refuses to overwrite one.
//!
//! Every setting is a closed enum. Free text would put prose into the
//! `profiles` table unsealed, which `docs/SECURITY.md` reserves for
//! `sealedText`, and it would give an inference somewhere a place to write a
//! sentence nobody validated.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceField {
    Register,
    Directness,
    Warmth,
    EmojiUse,
}

impl VoiceField {
    pub const ALL: &'static [VoiceField] = &[
        VoiceField::Register,
        VoiceField::Directness,
        VoiceField::Warmth,
        VoiceField::EmojiUse,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            VoiceField::Register => "register",
            VoiceField::Directness => "directness",
            VoiceField::Warmth => "warmth",
            VoiceField::EmojiUse => "emoji_use",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceRegister {
    Casual,
    Plain,
    Formal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceDirectness {
    Reserved,
    Balanced,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceWarmth {
    Cool,
    Even,
    Warm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmojiUse {
    Never,
    Sparing,
    Frequent,
}

/// One field and the value it is being given.
///
/// A single type for all four fields keeps "set it" and "propose it" to one
/// method each, so a future field cannot be added to one path and forgotten on
/// the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "field", content = "value")]
pub enum VoiceSetting {
    Register(VoiceRegister),
    Directness(VoiceDirectness),
    Warmth(VoiceWarmth),
    EmojiUse(EmojiUse),
}

impl VoiceSetting {
    pub fn field(self) -> VoiceField {
        match self {
            VoiceSetting::Register(_) => VoiceField::Register,
            VoiceSetting::Directness(_) => VoiceField::Directness,
            VoiceSetting::Warmth(_) => VoiceField::Warmth,
            VoiceSetting::EmojiUse(_) => VoiceField::EmojiUse,
        }
    }

    /// Every value a field can take, in the order the questionnaire offers
    /// them. `soul_import::questionnaire` declares the same tokens because the
    /// recorder has to refuse an option nobody offered; the test in
    /// `tests/one_questionnaire.rs` holds the two spellings together.
    pub fn all_of(field: VoiceField) -> Vec<VoiceSetting> {
        match field {
            VoiceField::Register => vec![
                VoiceSetting::Register(VoiceRegister::Casual),
                VoiceSetting::Register(VoiceRegister::Plain),
                VoiceSetting::Register(VoiceRegister::Formal),
            ],
            VoiceField::Directness => vec![
                VoiceSetting::Directness(VoiceDirectness::Reserved),
                VoiceSetting::Directness(VoiceDirectness::Balanced),
                VoiceSetting::Directness(VoiceDirectness::Direct),
            ],
            VoiceField::Warmth => vec![
                VoiceSetting::Warmth(VoiceWarmth::Cool),
                VoiceSetting::Warmth(VoiceWarmth::Even),
                VoiceSetting::Warmth(VoiceWarmth::Warm),
            ],
            VoiceField::EmojiUse => vec![
                VoiceSetting::EmojiUse(EmojiUse::Never),
                VoiceSetting::EmojiUse(EmojiUse::Sparing),
                VoiceSetting::EmojiUse(EmojiUse::Frequent),
            ],
        }
    }

    /// The wire spelling of the value, which is also the questionnaire's
    /// option key. Taken from serde rather than written out a second time.
    pub fn option_key(self) -> String {
        let value = match self {
            VoiceSetting::Register(value) => serde_json::to_value(value),
            VoiceSetting::Directness(value) => serde_json::to_value(value),
            VoiceSetting::Warmth(value) => serde_json::to_value(value),
            VoiceSetting::EmojiUse(value) => serde_json::to_value(value),
        };
        value
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .expect("a closed enum serializes to its name")
    }

    /// The setting an option key names, for one field.
    pub fn from_option(field: VoiceField, key: &str) -> Option<VoiceSetting> {
        VoiceSetting::all_of(field)
            .into_iter()
            .find(|setting| setting.option_key() == key)
    }
}

/// The `voice` object inside `SoulProfile`.
///
/// `user_set` is the lock. It is stored with the profile rather than derived,
/// because "the user changed this" has to survive a restart to mean anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub register: VoiceRegister,
    pub directness: VoiceDirectness,
    pub warmth: VoiceWarmth,
    pub emoji_use: EmojiUse,
    /// Fields the user set by hand. Inference must leave these alone.
    #[serde(default)]
    pub user_set: BTreeSet<VoiceField>,
}

impl Default for VoiceProfile {
    /// The neutral voice a profile starts with. Not a guess about the user:
    /// it is what the deterministic draft template falls back to until there
    /// is either an answer or an observation.
    fn default() -> Self {
        VoiceProfile {
            register: VoiceRegister::Plain,
            directness: VoiceDirectness::Balanced,
            warmth: VoiceWarmth::Even,
            emoji_use: EmojiUse::Sparing,
            user_set: BTreeSet::new(),
        }
    }
}

impl VoiceProfile {
    pub fn is_locked(&self, field: VoiceField) -> bool {
        self.user_set.contains(&field)
    }

    /// Apply a value the user chose, and remember that they chose it.
    pub fn set_by_user(&mut self, setting: VoiceSetting) {
        self.assign(setting);
        self.user_set.insert(setting.field());
    }

    /// Apply a value something inferred, unless the user already spoke.
    ///
    /// Returns whether it was applied, so the caller can audit the refusal
    /// rather than quietly reporting success.
    #[must_use]
    pub fn suggest(&mut self, setting: VoiceSetting) -> bool {
        if self.is_locked(setting.field()) {
            return false;
        }
        self.assign(setting);
        true
    }

    fn assign(&mut self, setting: VoiceSetting) {
        match setting {
            VoiceSetting::Register(value) => self.register = value,
            VoiceSetting::Directness(value) => self.directness = value,
            VoiceSetting::Warmth(value) => self.warmth = value,
            VoiceSetting::EmojiUse(value) => self.emoji_use = value,
        }
    }

    pub fn get(&self, field: VoiceField) -> VoiceSetting {
        match field {
            VoiceField::Register => VoiceSetting::Register(self.register),
            VoiceField::Directness => VoiceSetting::Directness(self.directness),
            VoiceField::Warmth => VoiceSetting::Warmth(self.warmth),
            VoiceField::EmojiUse => VoiceSetting::EmojiUse(self.emoji_use),
        }
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("a closed enum record always serializes")
    }

    /// Read the `voice` object back out of a stored profile.
    ///
    /// A profile written by an older build, or by a test that put something
    /// else in the field, falls back to the neutral voice rather than failing
    /// the whole read: the user's axes should still be visible.
    pub fn from_value(value: &Value) -> VoiceProfile {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }
}
