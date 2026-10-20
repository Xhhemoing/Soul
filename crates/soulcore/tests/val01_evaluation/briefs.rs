//! Explicit A/B/C construction for synthetic evaluation, never a Session fallback.
use super::records::Variant;
use soul_draft::{error::DraftResult, ProfileBrief};
use soul_profile::{
    view::ProfileView,
    voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceSetting, VoiceWarmth},
};

#[derive(Clone, Copy, Debug)]
pub struct ManualVoice {
    pub register: VoiceRegister,
    pub directness: VoiceDirectness,
    pub warmth: VoiceWarmth,
    pub emoji_use: EmojiUse,
}

pub fn build_brief(
    variant: Variant,
    snapshot: &ProfileView,
    manual: ManualVoice,
) -> DraftResult<ProfileBrief> {
    match variant {
        Variant::A => Ok(ProfileBrief::neutral()),
        // Keep this Result: a failed B must never become a successful neutral A.
        Variant::B => ProfileBrief::from_view(snapshot),
        Variant::C => {
            let mut brief = ProfileBrief::new(VoiceProfile::default());
            for setting in [
                VoiceSetting::Register(manual.register),
                VoiceSetting::Directness(manual.directness),
                VoiceSetting::Warmth(manual.warmth),
                VoiceSetting::EmojiUse(manual.emoji_use),
            ] {
                brief.set_by_user(setting);
            }
            Ok(brief)
        }
    }
}

pub fn same_template_voice(left: &ProfileBrief, right: &ProfileBrief) -> bool {
    let (left, right) = (left.voice(), right.voice());
    (left.register, left.directness, left.warmth, left.emoji_use)
        == (
            right.register,
            right.directness,
            right.warmth,
            right.emoji_use,
        )
}
