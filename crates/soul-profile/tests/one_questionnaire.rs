//! One question set, asked once, understood the same way by both crates.
//!
//! WP03 and WP06 each landed a questionnaire: seven questions keyed
//! `q.axis.curiosity` here, eight keyed `voice.directness` there. Nothing
//! overwrote anything, which is precisely why the divergence was survivable
//! and why it would have shipped — the user would simply have been asked about
//! their own voice twice, and half the answers would never have reached the
//! profile.
//!
//! There is now one list, in `soul-import`, and one mapping, here.
//! `fixtures/questionnaire/v0_1.json` is what stops either from moving on its
//! own: it names every question id, what shape it is, and which profile field
//! it moves. The import side checks the first two against the same file.

use serde::Deserialize;

use soul_import::questionnaire::{AnswerShape, QUESTIONS};
use soul_profile::questionnaire::{questionnaire, target_of, voice_question_id, Answer};
use soul_profile::voice::{VoiceField, VoiceSetting};
use soul_profile::{axes, QuestionTarget, StatedField};
use soul_testkit::fixtures;

#[derive(Debug, Deserialize)]
struct Definition {
    questions: Vec<DefinedQuestion>,
}

#[derive(Debug, Deserialize)]
struct DefinedQuestion {
    id: String,
    shape: String,
    #[serde(default)]
    options: Vec<String>,
    profile_target: String,
}

fn definition() -> Definition {
    fixtures::read_json("questionnaire/v0_1.json").expect("the canonical question set")
}

/// The pin. Ids, order, and what each one moves.
#[test]
fn the_profile_asks_exactly_the_questions_the_recorder_offers() {
    let defined = definition();

    let asked: Vec<&str> = questionnaire().iter().map(|q| q.question_id).collect();
    let recorded: Vec<&str> = QUESTIONS.iter().map(|q| q.key).collect();
    let pinned: Vec<&str> = defined.questions.iter().map(|q| q.id.as_str()).collect();

    assert_eq!(
        asked, recorded,
        "a question the recorder asks that the profile cannot place is a question \
         whose answer goes nowhere",
    );
    assert_eq!(asked, pinned, "the fixture is the id every store points at");
    assert_eq!(asked.len(), 11);
}

#[test]
fn every_question_lands_on_the_field_the_fixture_says_it_does() {
    for defined in definition().questions {
        let target =
            target_of(&defined.id).unwrap_or_else(|| panic!("{} has no profile field", defined.id));

        let landed = match target {
            QuestionTarget::Axis(axis) => format!("axis.{}", axis.key),
            QuestionTarget::Voice(field) => format!("voice.{}", field.as_str()),
            QuestionTarget::Stated(field) => field.as_str().to_owned(),
        };
        assert_eq!(landed, defined.profile_target, "{} moved", defined.id);

        // The recorder declares the options as opaque tokens; here they have
        // to parse into a position or a setting, or the sink would refuse an
        // answer the wizard was allowed to give.
        match (target, defined.shape.as_str()) {
            (QuestionTarget::Axis(_), "choice") => {
                for option in &defined.options {
                    assert!(
                        axes::position_by_key(option).is_some(),
                        "{} offers `{option}`, which is not a direction",
                        defined.id,
                    );
                }
            }
            (QuestionTarget::Voice(field), "choice") => {
                for option in &defined.options {
                    assert!(
                        VoiceSetting::from_option(field, option).is_some(),
                        "{} offers `{option}`, which {} does not take",
                        defined.id,
                        field.as_str(),
                    );
                }
            }
            (QuestionTarget::Stated(_), "prose") => {
                assert!(defined.options.is_empty(), "a text box offers no options")
            }
            (target, shape) => panic!("{} is {shape} but moves {}", defined.id, target.label()),
        }
    }
}

/// The three voice fields the wizard asks about are pinned by the answer; the
/// fourth keeps its neutral default until the user changes it in the profile
/// view.
#[test]
fn the_voice_fields_the_wizard_asks_about_are_the_ones_with_a_question() {
    assert_eq!(
        voice_question_id(VoiceField::Register),
        Some("q.voice.register")
    );
    assert_eq!(
        voice_question_id(VoiceField::Directness),
        Some("q.voice.directness")
    );
    assert_eq!(
        voice_question_id(VoiceField::EmojiUse),
        Some("q.voice.emoji_use")
    );
    assert_eq!(
        voice_question_id(VoiceField::Warmth),
        None,
        "warmth is not asked, and saying so is better than pointing at a question that does not exist",
    );
}

/// What a UI has to do to answer: hand back the option key it drew. A wizard
/// should not have to know that `leans_high` is a position and `formal` is a
/// register.
#[test]
fn an_answer_can_be_built_from_the_option_the_wizard_drew() {
    for question in questionnaire() {
        match question.shape {
            AnswerShape::Choice(options) => {
                for option in options {
                    let answer = Answer::for_question(question.question_id, option)
                        .unwrap_or_else(|e| panic!("{} / {option}: {e}", question.question_id));
                    assert_eq!(answer.question_id(), question.question_id);
                }
                let refused = Answer::for_question(question.question_id, "有点两边都占")
                    .expect_err("an option nobody offered");
                assert!(
                    !refused.to_string().contains("有点两边都占"),
                    "a refusal does not repeat what came in: {refused}",
                );
            }
            AnswerShape::Prose => {
                let answer = Answer::for_question(question.question_id, "晚上十一点以后不回")
                    .expect("a text box takes what was typed");
                assert!(matches!(answer, Answer::Prose { .. }));
            }
        }
    }

    assert!(matches!(
        Answer::for_question("q.axis.made_up", "leans_high"),
        Err(soul_profile::ProfileError::UnknownQuestion(_)),
    ));
}

/// Both halves of the boundary pair land in the same profile field, which is
/// the reason `StatedField` exists rather than one target per question.
#[test]
fn the_two_boundary_questions_share_one_field() {
    for id in ["q.boundary.topics", "q.boundary.availability"] {
        assert_eq!(
            target_of(id),
            Some(QuestionTarget::Stated(StatedField::Boundary)),
        );
    }
    assert_eq!(
        target_of("q.value.what_matters"),
        Some(QuestionTarget::Stated(StatedField::Value)),
    );
}
