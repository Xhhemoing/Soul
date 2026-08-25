//! What a draft is allowed to know about the user.
//!
//! AC-07 is the rule this module exists for: a voice field the user set is
//! what the prompt says, and no inference gets to overwrite it on the way.
//! `soul-profile` already refuses to overwrite a set field in the *store*
//! ([`VoiceProfile::suggest`]); the same refusal has to hold here, because a
//! brief is assembled in memory and a caller with an observation in hand is
//! exactly the caller who would otherwise apply it.
//!
//! So [`ProfileBrief::apply_inferred`] delegates to the same method and
//! returns whether it took, and there is no setter that bypasses it. The only
//! way a user value changes is [`ProfileBrief::set_by_user`].
//!
//! Two things a brief deliberately does not carry:
//!
//! * an axis with no evidence behind it. `evidence_band: none` means nobody
//!   has observed anything, and a prompt that asserted it anyway would be the
//!   product inventing a personality;
//! * anybody's name. The brief is about the user, and the redactor scrubs
//!   identifiers out of it regardless before it becomes a request body.

use std::collections::BTreeSet;

use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_profile::view::{render_voice, ProfileView};
use soul_profile::voice::{VoiceField, VoiceProfile, VoiceSetting};
use soul_schema::common::EvidenceBand;

use crate::error::DraftResult;

/// One axis as a draft prompt states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisReading {
    /// Already checked against the diagnostic denylist.
    pub text: String,
    pub band: EvidenceBand,
    pub locked_by_user: bool,
}

/// The user, as far as a draft prompt is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileBrief {
    voice: VoiceProfile,
    readings: Vec<AxisReading>,
}

impl ProfileBrief {
    /// A brief carrying a voice and no axis readings.
    pub fn new(voice: VoiceProfile) -> ProfileBrief {
        ProfileBrief {
            voice,
            readings: Vec::new(),
        }
    }

    /// The brief a profile with nothing in it yet produces: the neutral voice
    /// and no claims. This is what the desktop shell drafts with before an
    /// import or a questionnaire has happened.
    pub fn neutral() -> ProfileBrief {
        ProfileBrief::new(VoiceProfile::default())
    }

    /// Read the brief off a resolved profile view.
    ///
    /// `view.voice` is the stored voice, `user_set` included, so the locks
    /// survive into the brief rather than being recomputed from something.
    /// Axes with no evidence are left out; see the module docs.
    pub fn from_view(view: &ProfileView) -> DraftResult<ProfileBrief> {
        let mut brief = ProfileBrief::new(view.voice.clone());
        for axis in &view.axes {
            if axis.evidence_band == EvidenceBand::None {
                continue;
            }
            assert_non_clinical(&axis.reading)?;
            brief.readings.push(AxisReading {
                text: axis.reading.clone(),
                band: axis.evidence_band,
                locked_by_user: axis.locked_by_user,
            });
        }
        Ok(brief)
    }

    pub fn voice(&self) -> &VoiceProfile {
        &self.voice
    }

    pub fn readings(&self) -> &[AxisReading] {
        &self.readings
    }

    /// Which voice fields the user set by hand.
    pub fn locked_fields(&self) -> &BTreeSet<VoiceField> {
        &self.voice.user_set
    }

    /// The user changed a voice field. Pins it against later inference.
    pub fn set_by_user(&mut self, setting: VoiceSetting) -> &mut Self {
        self.voice.set_by_user(setting);
        self
    }

    /// Apply a voice value something observed rather than something the user
    /// said.
    ///
    /// Returns `false`, and changes nothing, when the user has already set
    /// that field. This is AC-07 inside the drafting path: the refusal is the
    /// function's purpose, which is why the result is `#[must_use]` on the way
    /// down into `VoiceProfile::suggest`.
    pub fn apply_inferred(&mut self, setting: VoiceSetting) -> bool {
        self.voice.suggest(setting)
    }

    /// Add an axis reading the caller resolved itself.
    pub fn with_reading(
        mut self,
        text: impl Into<String>,
        band: EvidenceBand,
        locked_by_user: bool,
    ) -> DraftResult<ProfileBrief> {
        let text = text.into();
        assert_non_clinical(&text)?;
        self.readings.push(AxisReading {
            text,
            band,
            locked_by_user,
        });
        Ok(self)
    }

    /// The brief in words: what goes into the prompt, and what the
    /// deterministic template reads.
    ///
    /// Descriptive throughout. Nothing here is phrased as an instruction,
    /// because this text travels in the request's quoted-material slot, and
    /// the material slot is precisely the one Soul promises not to obey — see
    /// the note in `crate::draft`.
    pub fn render(&self) -> DraftResult<String> {
        let mut lines = vec![
            BRIEF_HEADING.to_owned(),
            format!("语气：{}", render_voice(&self.voice)),
            format!("语气来源：{}", self.render_voice_provenance()),
        ];

        if self.readings.is_empty() {
            lines.push("档案要点：暂无有证据支持的要点。".to_owned());
        } else {
            lines.push("档案要点：".to_owned());
            for reading in &self.readings {
                let lock = match reading.locked_by_user {
                    true => "，你已锁定",
                    false => "",
                };
                lines.push(format!(
                    "- {}（证据{}{lock}）",
                    reading.text,
                    band_word(reading.band),
                ));
            }
        }
        lines.push(WORKING_HYPOTHESIS_NOTICE.to_owned());

        let rendered = lines.join("\n");
        assert_non_clinical(&rendered)?;
        Ok(rendered)
    }

    fn render_voice_provenance(&self) -> String {
        let locked: Vec<&str> = VoiceField::ALL
            .iter()
            .filter(|field| self.voice.is_locked(**field))
            .map(|field| field_label(*field))
            .collect();
        match locked.is_empty() {
            true => "四项都还是默认值".to_owned(),
            false => format!("{} 由你设定，其余为默认值", locked.join("、")),
        }
    }
}

/// The line a brief opens with, so a reader of a recorded request body can see
/// where the profile material starts.
pub const BRIEF_HEADING: &str = "【本机档案，供起草参考】";

/// A voice field as a person would name it.
pub fn field_label(field: VoiceField) -> &'static str {
    match field {
        VoiceField::Register => "语域",
        VoiceField::Directness => "直接程度",
        VoiceField::Warmth => "温度",
        VoiceField::EmojiUse => "表情用量",
    }
}

fn band_word(band: EvidenceBand) -> &'static str {
    match band {
        EvidenceBand::Weak => "弱",
        EvidenceBand::Moderate => "中",
        EvidenceBand::Strong => "强",
        EvidenceBand::None => "无",
    }
}
