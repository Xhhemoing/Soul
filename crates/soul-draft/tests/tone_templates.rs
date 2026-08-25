//! The draft this product writes when there is no endpoint: deterministic, in
//! the user's voice, and never a constant.
//!
//! Two failures these are written to catch. One is a template that ignores the
//! voice — a fixed sentence satisfies "drafting returned something", so the
//! check is that changing any of the four fields changes the bytes. The other
//! is vocabulary: WP03 shipped a voice word that contained a forbidden term
//! inside it and only the exhaustive render caught it, so all eighty-one
//! combinations are rendered here and scanned against the whole denylist.

use soul_draft::{template_draft, tone_directive};
use soul_policy::clinical::assert_non_clinical;
use soul_policy::redactor::{KnownIdentifiers, Redactor, Turn, THIRD_PARTY_PLACEHOLDER};
use soul_profile::voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth};
use soul_schema::common::SealedSubject;
use soul_testkit::fixtures;
use uuid::Uuid;

const THIRD_PARTY_BODY: &str = "明天上午十点在公司门口见";
const OWNER_BODY: &str = "我看一下日程再回你。";

fn turns() -> Vec<Turn> {
    vec![
        Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, THIRD_PARTY_BODY),
        Turn::new(Uuid::now_v7(), SealedSubject::Owner, OWNER_BODY),
    ]
}

fn material() -> soul_policy::redactor::RedactedBody {
    Redactor::new(KnownIdentifiers::new()).redact_for_e1(&turns())
}

fn voice(
    register: VoiceRegister,
    directness: VoiceDirectness,
    warmth: VoiceWarmth,
    emoji_use: EmojiUse,
) -> VoiceProfile {
    VoiceProfile {
        register,
        directness,
        warmth,
        emoji_use,
        user_set: Default::default(),
    }
}

const REGISTERS: [VoiceRegister; 3] = [
    VoiceRegister::Casual,
    VoiceRegister::Plain,
    VoiceRegister::Formal,
];
const DIRECTNESS: [VoiceDirectness; 3] = [
    VoiceDirectness::Reserved,
    VoiceDirectness::Balanced,
    VoiceDirectness::Direct,
];
const WARMTHS: [VoiceWarmth; 3] = [VoiceWarmth::Cool, VoiceWarmth::Even, VoiceWarmth::Warm];
const EMOJI: [EmojiUse; 3] = [EmojiUse::Never, EmojiUse::Sparing, EmojiUse::Frequent];

/// D-05.1: same voice, same material, same bytes.
#[test]
fn same_input_same_voice_is_byte_identical() {
    let voice = VoiceProfile::default();
    let body = material();

    let first = template_draft(&voice, &body);
    let second = template_draft(&voice, &body);

    assert_eq!(first, second, "the template is not deterministic");
    assert!(
        first.contains(THIRD_PARTY_PLACEHOLDER),
        "the draft is written against placeheld material: {first}",
    );
    assert!(
        first.contains(OWNER_BODY),
        "the user's own words are theirs to keep: {first}",
    );
    assert!(
        !first.contains(THIRD_PARTY_BODY),
        "somebody else's prose is not part of a local draft either: {first}",
    );
}

/// D-05.2: a constant string cannot pass for a template. Every one of the four
/// voice fields changes the draft, and all eighty-one renders differ.
#[test]
fn different_directness_and_emoji_change_the_draft() {
    let body = material();

    let reserved = template_draft(
        &voice(
            VoiceRegister::Plain,
            VoiceDirectness::Reserved,
            VoiceWarmth::Even,
            EmojiUse::Never,
        ),
        &body,
    );
    let direct = template_draft(
        &voice(
            VoiceRegister::Plain,
            VoiceDirectness::Direct,
            VoiceWarmth::Even,
            EmojiUse::Never,
        ),
        &body,
    );
    assert_ne!(reserved, direct, "directness must reach the draft");

    let frequent = template_draft(
        &voice(
            VoiceRegister::Plain,
            VoiceDirectness::Reserved,
            VoiceWarmth::Even,
            EmojiUse::Frequent,
        ),
        &body,
    );
    assert_ne!(reserved, frequent, "emoji use must reach the draft");

    let mut rendered = std::collections::BTreeSet::new();
    for register in REGISTERS {
        for directness in DIRECTNESS {
            for warmth in WARMTHS {
                for emoji_use in EMOJI {
                    let drafted =
                        template_draft(&voice(register, directness, warmth, emoji_use), &body);
                    assert!(
                        rendered.insert(drafted),
                        "two voices produced the same draft: {register:?}/{directness:?}/{warmth:?}/{emoji_use:?}",
                    );
                }
            }
        }
    }
    assert_eq!(rendered.len(), 81);
}

/// D-05.4: every combination, against the whole denylist rather than against
/// the handful of words a reviewer would think to look for.
#[test]
fn all_81_voice_combinations_render_non_clinical() {
    let body = material();
    let terms = fixtures::denylist_terms().expect("the denylist fixture");
    assert!(
        terms.len() > 50,
        "the denylist came back too short to be the real one: {}",
        terms.len(),
    );

    let mut combinations = 0usize;
    for register in REGISTERS {
        for directness in DIRECTNESS {
            for warmth in WARMTHS {
                for emoji_use in EMOJI {
                    let voice = voice(register, directness, warmth, emoji_use);
                    for text in [template_draft(&voice, &body), tone_directive(&voice)] {
                        assert_non_clinical(&text).expect("the draft says nothing clinical");
                        let lowered = text.to_lowercase();
                        for term in &terms {
                            assert!(
                                !lowered.contains(&term.to_lowercase()),
                                "`{term}` reached a rendered draft: {text}",
                            );
                        }
                    }
                    combinations += 1;
                }
            }
        }
    }
    assert_eq!(combinations, 81);
}

/// The line that carries the voice onto the wire says the voice and nothing
/// else: no free JSON, and not the internal lock list.
#[test]
fn the_tone_line_carries_four_words_and_no_internal_state() {
    let spoken = tone_directive(&voice(
        VoiceRegister::Formal,
        VoiceDirectness::Direct,
        VoiceWarmth::Cool,
        EmojiUse::Never,
    ));

    assert!(spoken.contains("正式") && spoken.contains("直接"));
    assert!(
        !spoken.contains("user_set"),
        "the lock list is internal: {spoken}",
    );
    assert!(
        !spoken.contains('{'),
        "the voice reaches the wire as words, not as its JSON: {spoken}",
    );
}
