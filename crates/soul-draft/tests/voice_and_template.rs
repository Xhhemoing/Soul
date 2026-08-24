//! AC-07 and AC-17: the prompt uses the voice the user set, and with no key
//! there is a deterministic template instead of a model.
//!
//! AC-07 is checked through the real profile write path rather than by handing
//! `ProfileBrief` a value directly. That matters: the claim is not "a struct
//! keeps what you put in it" but "a voice the user pinned survives an
//! inference that disagrees, all the way from the store into the prompt". So
//! the test sets the field, has something infer the opposite, and then reads
//! the profile back the way drafting reads it.
//!
//! AC-17 is checked twice over — once for what the template *is* (a pure
//! function: same input, same bytes) and once for what it is *not* (a socket:
//! a mock endpoint is running throughout and receives nothing).

use uuid::Uuid;

use soul_draft::brief::ProfileBrief;
use soul_draft::draft::{DraftSource, Drafter, NOT_SENT_NOTICE, TEMPLATE_NOTICE};
use soul_draft::template::{self, TemplateContext, BODY_SLOT};
use soul_draft::DraftRequest;
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_policy::redactor::{KnownIdentifiers, Redactor};
use soul_profile::view::profile_view;
use soul_profile::voice::{
    EmojiUse, VoiceDirectness, VoiceField, VoiceProfile, VoiceRegister, VoiceSetting, VoiceWarmth,
};
use soul_profile::{read_voice, set_voice, suggest_voice};
use soul_store_api::FakeStore;
use soul_testkit::MockLlm;

const NOW: i64 = 1_787_529_600;
const MODEL: &str = "local-model";

fn drafter() -> Drafter {
    Drafter::new(Redactor::new(KnownIdentifiers::new()), MODEL)
}

/// A brief whose voice the user pinned to `直接` and `正式`, read back out of
/// a store after something tried to infer the opposite.
fn brief_after_a_correction_and_a_disagreement() -> (ProfileBrief, bool, bool) {
    let mut store = FakeStore::new();
    let profile_id = Uuid::now_v7();

    set_voice(
        &mut store,
        profile_id,
        VoiceSetting::Directness(VoiceDirectness::Direct),
        NOW,
    )
    .expect("the user sets how direct they are");
    set_voice(
        &mut store,
        profile_id,
        VoiceSetting::Register(VoiceRegister::Formal),
        NOW,
    )
    .expect("the user sets the register");

    // Something observed the opposite of both. The store refuses the pinned
    // fields and accepts the one nobody claimed.
    let directness_took = suggest_voice(
        &mut store,
        profile_id,
        VoiceSetting::Directness(VoiceDirectness::Reserved),
        NOW,
    )
    .expect("the suggestion is answered");
    let warmth_took = suggest_voice(
        &mut store,
        profile_id,
        VoiceSetting::Warmth(VoiceWarmth::Warm),
        NOW,
    )
    .expect("the suggestion is answered");

    let view = profile_view(&store, profile_id).expect("the profile resolves");
    let brief = ProfileBrief::from_view(&view).expect("the brief builds");

    // The brief must be reading the same voice the store holds, not a default.
    let stored = read_voice(&store, profile_id).expect("the stored voice");
    assert_eq!(brief.voice(), &stored);

    (brief, directness_took, warmth_took)
}

#[test]
fn a_pinned_voice_field_survives_an_inference_that_disagrees_with_it() {
    let (brief, directness_took, warmth_took) = brief_after_a_correction_and_a_disagreement();

    assert!(
        !directness_took,
        "an inference must not overwrite a field the user set",
    );
    assert!(
        warmth_took,
        "a field nobody pinned is still open to inference; \
         otherwise this test would pass with a store that refuses everything",
    );

    assert_eq!(brief.voice().directness, VoiceDirectness::Direct);
    assert_eq!(brief.voice().register, VoiceRegister::Formal);
    assert_eq!(brief.voice().warmth, VoiceWarmth::Warm);
    assert!(brief.locked_fields().contains(&VoiceField::Directness));
    assert!(brief.locked_fields().contains(&VoiceField::Register));
    assert!(!brief.locked_fields().contains(&VoiceField::Warmth));
}

/// AC-07 proper: the *prompt*, not just the profile.
#[test]
fn the_prompt_carries_the_users_value_and_not_the_inferred_one() {
    let (brief, _, _) = brief_after_a_correction_and_a_disagreement();
    let rendered = brief.render().expect("the brief renders");

    assert!(
        rendered.contains("直接"),
        "the user's value is in the prompt"
    );
    assert!(
        !rendered.contains("含蓄"),
        "the refused inference must not appear anywhere in the prompt: {rendered}",
    );
    assert!(rendered.contains("正式"));
    assert!(
        rendered.contains("直接程度"),
        "the prompt says which fields the user pinned",
    );

    // The same value reaches the request body a generation would have used.
    let request = DraftRequest::new(brief, Vec::new());
    let body = drafter()
        .redact(&request, None)
        .expect("the body builds without an endpoint being involved");
    assert!(body.as_str().contains("直接"));
    assert!(!body.as_str().contains("含蓄"));
}

/// The in-memory half of the same rule. A caller holding an observation is
/// exactly the caller who would apply it over the user's value.
#[test]
fn a_brief_refuses_an_inference_about_a_field_the_user_set() {
    let mut brief = ProfileBrief::neutral();
    brief.set_by_user(VoiceSetting::EmojiUse(EmojiUse::Never));

    assert!(
        !brief.apply_inferred(VoiceSetting::EmojiUse(EmojiUse::Frequent)),
        "the refusal is the point of the function",
    );
    assert_eq!(brief.voice().emoji_use, EmojiUse::Never);

    assert!(
        brief.apply_inferred(VoiceSetting::Warmth(VoiceWarmth::Cool)),
        "an untouched field is still open",
    );
    assert_eq!(brief.voice().warmth, VoiceWarmth::Cool);
}

/// AC-17. The mock endpoint is running for the whole test and is never told
/// about; if a template ever grew a network call, its counter would move.
#[test]
fn with_no_endpoint_the_draft_is_a_template_and_nothing_is_contacted() {
    let mock = MockLlm::start().expect("start a mock endpoint nobody is configured to use");

    let brief = ProfileBrief::neutral();
    let request = DraftRequest::from_paste(brief, "明天上午十点在公司门口见，别迟到");
    let draft = drafter()
        .draft_offline(&request)
        .expect("drafting works without a key");

    assert_eq!(draft.source, DraftSource::ToneTemplate);
    assert_eq!(draft.degraded, None);
    assert!(
        draft.text.contains(BODY_SLOT),
        "the slot is left for the user"
    );
    assert_eq!(draft.not_sent_notice, NOT_SENT_NOTICE);
    assert_eq!(draft.source_notice, TEMPLATE_NOTICE);
    assert_eq!(
        mock.request_count(),
        0,
        "the no-key path must not open a connection",
    );

    // The third party's words are not in the draft either: the template never
    // reads them, so there is nothing to copy into a clipboard by accident.
    assert!(!draft.text.contains("公司门口"));
}

/// Determinism, which is the whole of what AC-17 promises about the no-key
/// path: the same inputs give the same draft, every time, on a `Drafter` that
/// was built fresh each round so nothing can be carried between them.
///
/// A run of 32 rather than a pair, because the ways this could go wrong are
/// ones a single repeat might miss — a hash map iterated in whatever order it
/// felt like, a clock consulted, a random tiebreak between two phrasings.
#[test]
fn the_template_is_a_function_of_its_inputs_and_nothing_else() {
    let mut voice = VoiceProfile::default();
    voice.set_by_user(VoiceSetting::Register(VoiceRegister::Formal));
    let brief = ProfileBrief::new(voice);
    let request = DraftRequest::from_paste(brief, "在吗");

    let first = drafter().draft_offline(&request).expect("first draft");
    for round in 1..32 {
        let again = drafter().draft_offline(&request).expect("another draft");
        assert_eq!(again, first, "draft {round} differs from the first one");
    }
}

/// Every voice field has to reach the output. A template that ignored one
/// would still pass a "renders something" test while silently disregarding a
/// setting the user changed on purpose.
#[test]
fn changing_any_one_voice_field_changes_the_draft() {
    let base = VoiceProfile::default();
    let baseline = render_with(&base);

    for setting in [
        VoiceSetting::Register(VoiceRegister::Formal),
        VoiceSetting::Directness(VoiceDirectness::Direct),
        VoiceSetting::Warmth(VoiceWarmth::Warm),
        VoiceSetting::EmojiUse(EmojiUse::Frequent),
    ] {
        let mut voice = base.clone();
        voice.set_by_user(setting);
        assert_ne!(
            render_with(&voice),
            baseline,
            "{:?} did not change the draft",
            setting.field(),
        );
    }
}

#[test]
fn replying_and_starting_a_conversation_read_differently() {
    let brief = ProfileBrief::neutral();
    let replying = template::render(&brief, TemplateContext::replying_to(2)).expect("a reply");
    let opening = template::render(&brief, TemplateContext::opening()).expect("an opening");
    assert_ne!(replying, opening);
}

/// The exhaustive pass WP03 learned to write: a denylist hit inside one
/// combination of enum arms is invisible to review and to a single-case test.
/// 81 voices, both conversation shapes, brief and draft each.
#[test]
fn every_voice_combination_renders_something_this_product_may_say() {
    let registers = [
        VoiceRegister::Casual,
        VoiceRegister::Plain,
        VoiceRegister::Formal,
    ];
    let directness = [
        VoiceDirectness::Reserved,
        VoiceDirectness::Balanced,
        VoiceDirectness::Direct,
    ];
    let warmths = [VoiceWarmth::Cool, VoiceWarmth::Even, VoiceWarmth::Warm];
    let emojis = [EmojiUse::Never, EmojiUse::Sparing, EmojiUse::Frequent];

    let mut rendered = 0usize;
    for register in registers {
        for direct in directness {
            for warmth in warmths {
                for emoji in emojis {
                    let mut voice = VoiceProfile::default();
                    voice.set_by_user(VoiceSetting::Register(register));
                    voice.set_by_user(VoiceSetting::Directness(direct));
                    voice.set_by_user(VoiceSetting::Warmth(warmth));
                    voice.set_by_user(VoiceSetting::EmojiUse(emoji));

                    let brief = ProfileBrief::new(voice);
                    let prompt = brief.render().expect("the brief renders");
                    assert_non_clinical(&prompt).expect("the brief says nothing forbidden");
                    assert!(prompt.contains(WORKING_HYPOTHESIS_NOTICE));

                    for context in [TemplateContext::opening(), TemplateContext::replying_to(3)] {
                        let text = template::render(&brief, context).expect("the template renders");
                        assert_non_clinical(&text).expect("the draft says nothing forbidden");
                        assert!(text.contains(BODY_SLOT));
                        rendered += 1;
                    }
                }
            }
        }
    }
    assert_eq!(rendered, 81 * 2, "every combination was actually exercised");
}

/// An axis nobody has observed does not become a claim in the prompt.
#[test]
fn an_axis_with_no_evidence_stays_out_of_the_prompt() {
    let store = FakeStore::new();
    let view = profile_view(&store, Uuid::now_v7()).expect("a blank profile still resolves");
    assert_eq!(view.axes.len(), 5, "the blank profile has all five axes");

    let brief = ProfileBrief::from_view(&view).expect("the brief builds");
    assert!(brief.readings().is_empty());
    assert!(brief
        .render()
        .expect("renders")
        .contains("暂无有证据支持的要点"));
}

fn render_with(voice: &VoiceProfile) -> String {
    let request = DraftRequest::from_paste(ProfileBrief::new(voice.clone()), "在吗");
    drafter()
        .draft_offline(&request)
        .expect("the template renders")
        .text
}
