//! The deterministic tone template: what Soul drafts with no model at all.
//!
//! PRODUCT_LOCK says the profile, the graph, collection, memory, the audit and
//! the statistical people summary all keep working without a key, and that
//! drafting falls back to a deterministic tone template. AC-17 is that
//! sentence as a test, and the two words in it that matter are *deterministic*
//! and *template*:
//!
//! * deterministic — the output is a pure function of the voice and two
//!   counts. No clock, no randomness, no model. The same draft twice is the
//!   same bytes twice, which is what makes it something a test can pin;
//! * template — it is a skeleton with a slot in it, not a pretend reply. The
//!   product would rather hand the user an obviously unfinished sentence than
//!   a fluent one it made up. `BODY_SLOT` is the shape of that promise.
//!
//! Nothing here reads the conversation. The counts come in, the prose does
//! not: a template that quoted the other person would be putting third-party
//! text into a screen — and then a clipboard — for no benefit, and the whole
//! reason the fallback exists is that nothing has to leave the machine.

use soul_policy::clinical::assert_non_clinical;
use soul_profile::voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth};

use crate::brief::ProfileBrief;
use crate::error::DraftResult;

/// Where the user's own words go. Deliberately unmistakable.
pub const BODY_SLOT: &str = "（这里写你要说的内容）";

/// What shape the conversation is, as a count.
///
/// A number rather than the turns themselves, so this module cannot read
/// anybody's prose even by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TemplateContext {
    /// Turns written by someone other than the user.
    pub inbound_turns: usize,
}

impl TemplateContext {
    pub fn replying_to(inbound_turns: usize) -> TemplateContext {
        TemplateContext { inbound_turns }
    }

    /// Nothing to reply to: the user is starting something rather than
    /// answering it.
    pub fn opening() -> TemplateContext {
        TemplateContext { inbound_turns: 0 }
    }

    pub fn is_opening(&self) -> bool {
        self.inbound_turns == 0
    }
}

/// Render one draft.
///
/// Four voice fields choose four fragments and the counts choose one more.
/// That is the whole model: a lookup, not a generator.
pub fn render(brief: &ProfileBrief, context: TemplateContext) -> DraftResult<String> {
    let voice = brief.voice();

    let mut lines = Vec::with_capacity(5);
    lines.push(format!("{}{}", greeting(voice), receipt(voice, context)));
    lines.push(stance(voice).to_owned());
    lines.push(BODY_SLOT.to_owned());
    lines.push(format!("{}{}", closing(voice), emoji(voice)));

    let rendered = lines.join("\n");
    assert_non_clinical(&rendered)?;
    Ok(rendered)
}

fn greeting(voice: &VoiceProfile) -> &'static str {
    match voice.register {
        VoiceRegister::Casual => "嗨，",
        VoiceRegister::Plain => "你好，",
        VoiceRegister::Formal => "您好，",
    }
}

/// How the draft opens. Directness picks the phrasing; whether there is
/// anything to reply to picks between two sets.
fn receipt(voice: &VoiceProfile, context: TemplateContext) -> &'static str {
    match (voice.directness, context.is_opening()) {
        (VoiceDirectness::Direct, false) => "收到，我看了一下。",
        (VoiceDirectness::Balanced, false) => "消息我看到了，先回一句。",
        (VoiceDirectness::Reserved, false) => "刚看到，抽空回一下。",
        (VoiceDirectness::Direct, true) => "有件事说一下。",
        (VoiceDirectness::Balanced, true) => "有件事想跟你说。",
        (VoiceDirectness::Reserved, true) => "有件小事，想问问你。",
    }
}

fn stance(voice: &VoiceProfile) -> &'static str {
    match voice.warmth {
        VoiceWarmth::Warm => "先谢谢你专门说一声。",
        VoiceWarmth::Even => "我这边的想法是这样：",
        VoiceWarmth::Cool => "直接说重点。",
    }
}

fn closing(voice: &VoiceProfile) -> &'static str {
    match voice.register {
        VoiceRegister::Casual => "先这样，有事再说。",
        VoiceRegister::Plain => "先回到这里，有需要随时说。",
        VoiceRegister::Formal => "以上，若有需要请随时告知。",
    }
}

fn emoji(voice: &VoiceProfile) -> &'static str {
    match voice.emoji_use {
        EmojiUse::Never => "",
        EmojiUse::Sparing => "🙂",
        EmojiUse::Frequent => "🙂🙂",
    }
}
