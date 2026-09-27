//! Q2-01 preparatory coverage: combinations absent from the existing single
//! screen/domain tests. All stores and prose are synthetic; no endpoint or
//! collection consent is configured. This is wiring/continuity evidence, not
//! evidence of user satisfaction or memory retrieval.

use soul_draft::{render_template, ProfileBrief, TemplateContext};
use soul_profile::voice::{EmojiUse, VoiceDirectness, VoiceRegister, VoiceSetting, VoiceWarmth};
use soul_profile::{profile_view, set_voice};
use soul_schema::common::EvidenceBand;
use soul_schema::profile::AxisPosition;
use soul_store_api::FakeStore;
use soul_testkit::fixtures;
use soulcore::commands::graph::TieEdgeView;
use soulcore::commands::memory::{MemoryChange, NewMemory};
use soulcore::commands::session::Session;
use soulcore::commands::store::AuditChainView;
use uuid::Uuid;

const PASTE: &str = "合成材料：下周再确认这个安排。";
const NEUTRAL_REPLY: &str = "你好，消息我看到了，先回一句。\n我这边的想法是这样：\n（这里写你要说的内容）\n先回到这里，有需要随时说。🙂";
const WARM_REPLY: &str = "你好，消息我看到了，先回一句。\n先谢谢你专门说一声。\n（这里写你要说的内容）\n先回到这里，有需要随时说。🙂";
const COOL_REPLY: &str = "你好，消息我看到了，先回一句。\n直接说重点。\n（这里写你要说的内容）\n先回到这里，有需要随时说。🙂";
const FORMAL_DIRECT_WARM_REPLY: &str = "您好，收到，我看了一下。\n先谢谢你专门说一声。\n（这里写你要说的内容）\n以上，若有需要请随时告知。";

/// Assert the privacy and audit contract at a checkpoint in a longer trail.
/// Existing tests own individual audit actions and consent/endpoint toggles.
fn private_chain(session: &Session, prose: &[&str]) -> AuditChainView {
    assert!(!session.snapshot().llm_endpoint_configured);
    let collection = session.collect_status();
    assert!(!collection.consent_granted);
    assert!(!collection.collector_running);
    let chain = session.audit().expect("read the audit checkpoint");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));
    let encoded = serde_json::to_string(&chain).expect("serialize audit checkpoint");
    for text in prose {
        for line in text.lines() {
            assert!(!line.is_empty());
            assert!(!encoded.contains(line), "audit includes synthetic prose");
        }
    }
    chain
}

fn assert_unknown_axes(session: &Session) {
    let profile = session.profile().expect("read profile");
    assert_eq!(profile.axes.len(), 5);
    assert!(profile.axes.iter().all(|axis| {
        axis.position == "unknown" && axis.evidence_count == 0 && !axis.locked_by_user
    }));
}

fn tie(session: &Session, relationship_id: &str) -> TieEdgeView {
    session
        .people()
        .expect("read graph")
        .ties
        .into_iter()
        .find(|edge| edge.relationship_id == relationship_id)
        .expect("the original relationship still exists")
}

/// Catches a later user change being ignored after reopening, stale session
/// voice caches, or one store's settings leaking into an independent store.
#[test]
fn warm_cool_warm_survives_repeated_reopens_without_changing_a_control_store() {
    let owner_dir = tempfile::tempdir().expect("owner temporary store");
    let control_dir = tempfile::tempdir().expect("control temporary store");
    let mut control = Session::open(control_dir.path());
    let control_profile = control.profile().expect("neutral control profile");
    assert_eq!(
        control.draft_pasted(PASTE).expect("control draft").text,
        NEUTRAL_REPLY
    );
    let mut previous_entries = Vec::new();

    for (option, expected) in [
        ("warm", WARM_REPLY),
        ("cool", COOL_REPLY),
        ("warm", WARM_REPLY),
    ] {
        let mut owner = Session::open(owner_dir.path());
        assert_eq!(private_chain(&owner, &[PASTE]).entries, previous_entries);
        owner
            .set_voice("warmth", option)
            .expect("change warmth after reopening");
        let saved_profile = owner.profile().expect("saved profile");
        let saved_chain = private_chain(&owner, &[PASTE, expected]);
        drop(owner);

        let mut reopened = Session::open(owner_dir.path());
        assert_eq!(reopened.profile().expect("reopened profile"), saved_profile);
        assert_eq!(
            private_chain(&reopened, &[PASTE, expected]).entries,
            saved_chain.entries
        );
        assert_unknown_axes(&reopened);
        let voice = &saved_profile.voice.fields;
        let warmth = voice
            .iter()
            .find(|field| field.field == "warmth")
            .expect("warmth field");
        assert_eq!(warmth.value, option);
        assert!(warmth.locked_by_user);
        assert_eq!(
            reopened.draft_pasted(PASTE).expect("reopened draft").text,
            expected
        );
        let drafted_chain = private_chain(&reopened, &[PASTE, expected]);
        assert!(drafted_chain.entries.starts_with(&saved_chain.entries));
        assert_eq!(drafted_chain.entries.len(), saved_chain.entries.len() + 1);
        previous_entries = drafted_chain.entries;
        drop(reopened);

        assert_eq!(
            control.profile().expect("unchanged control profile"),
            control_profile
        );
        assert_eq!(
            control
                .draft_pasted(PASTE)
                .expect("unchanged control draft")
                .text,
            NEUTRAL_REPLY
        );
        private_chain(&control, &[PASTE, NEUTRAL_REPLY]);
    }

    drop(control);
    let mut reopened_control = Session::open(control_dir.path());
    assert_eq!(
        reopened_control.profile().expect("control after reopening"),
        control_profile
    );
    assert_eq!(
        reopened_control
            .draft_pasted(PASTE)
            .expect("reopened control draft")
            .text,
        NEUTRAL_REPLY
    );
    private_chain(&reopened_control, &[PASTE, NEUTRAL_REPLY]);
}

/// Literal outputs are independent of the renderer: swapping two option
/// mappings, dropping a field in from_view, or discarding a lock must fail.
/// The existing 81-combination test owns exhaustive clinical screening.
#[test]
fn eleven_profiles_match_manual_briefs_and_literal_templates_with_unknown_axes() {
    struct Case {
        name: &'static str,
        settings: &'static [VoiceSetting],
        expected: &'static str,
    }
    let cases = [
        Case { name: "neutral", settings: &[], expected: NEUTRAL_REPLY },
        Case {
            name: "casual", settings: &[VoiceSetting::Register(VoiceRegister::Casual)],
            expected: "嗨，消息我看到了，先回一句。\n我这边的想法是这样：\n（这里写你要说的内容）\n先这样，有事再说。🙂",
        },
        Case {
            name: "formal", settings: &[VoiceSetting::Register(VoiceRegister::Formal)],
            expected: "您好，消息我看到了，先回一句。\n我这边的想法是这样：\n（这里写你要说的内容）\n以上，若有需要请随时告知。🙂",
        },
        Case {
            name: "reserved", settings: &[VoiceSetting::Directness(VoiceDirectness::Reserved)],
            expected: "你好，刚看到，抽空回一下。\n我这边的想法是这样：\n（这里写你要说的内容）\n先回到这里，有需要随时说。🙂",
        },
        Case {
            name: "direct", settings: &[VoiceSetting::Directness(VoiceDirectness::Direct)],
            expected: "你好，收到，我看了一下。\n我这边的想法是这样：\n（这里写你要说的内容）\n先回到这里，有需要随时说。🙂",
        },
        Case { name: "cool", settings: &[VoiceSetting::Warmth(VoiceWarmth::Cool)], expected: COOL_REPLY },
        Case { name: "warm", settings: &[VoiceSetting::Warmth(VoiceWarmth::Warm)], expected: WARM_REPLY },
        Case {
            name: "never emoji", settings: &[VoiceSetting::EmojiUse(EmojiUse::Never)],
            expected: "你好，消息我看到了，先回一句。\n我这边的想法是这样：\n（这里写你要说的内容）\n先回到这里，有需要随时说。",
        },
        Case {
            name: "frequent emoji", settings: &[VoiceSetting::EmojiUse(EmojiUse::Frequent)],
            expected: "你好，消息我看到了，先回一句。\n我这边的想法是这样：\n（这里写你要说的内容）\n先回到这里，有需要随时说。🙂🙂",
        },
        Case {
            name: "formal direct warm never", settings: &[
                VoiceSetting::Register(VoiceRegister::Formal),
                VoiceSetting::Directness(VoiceDirectness::Direct),
                VoiceSetting::Warmth(VoiceWarmth::Warm),
                VoiceSetting::EmojiUse(EmojiUse::Never),
            ], expected: FORMAL_DIRECT_WARM_REPLY,
        },
        Case {
            name: "casual reserved cool frequent", settings: &[
                VoiceSetting::Register(VoiceRegister::Casual),
                VoiceSetting::Directness(VoiceDirectness::Reserved),
                VoiceSetting::Warmth(VoiceWarmth::Cool),
                VoiceSetting::EmojiUse(EmojiUse::Frequent),
            ], expected: "嗨，刚看到，抽空回一下。\n直接说重点。\n（这里写你要说的内容）\n先这样，有事再说。🙂🙂",
        },
    ];
    assert_eq!(cases.len(), 11);
    for case in cases {
        let mut store = FakeStore::new();
        let profile_id = Uuid::from_u128(0x0192b0c0_5001_7c01_8c01_000051320001);
        let mut manual = ProfileBrief::neutral();
        for &setting in case.settings {
            set_voice(&mut store, profile_id, setting, 1_787_529_600)
                .expect("store synthetic voice");
            manual.set_by_user(setting);
        }
        let view = profile_view(&store, profile_id).expect("resolve stored profile");
        assert_eq!(view.axes.len(), 5, "{}", case.name);
        assert!(
            view.axes.iter().all(|axis| {
                axis.position == AxisPosition::Unknown
                    && axis.evidence_band == EvidenceBand::None
                    && axis.evidence.is_empty()
                    && !axis.locked_by_user
            }),
            "{}",
            case.name
        );
        let stored = ProfileBrief::from_view(&view).expect("brief from the stored view");
        assert!(stored.readings().is_empty(), "{}", case.name);
        assert_eq!(
            stored.locked_fields().len(),
            case.settings.len(),
            "{}",
            case.name
        );
        assert_eq!(stored, manual, "{}: stored and manual brief", case.name);
        let context = TemplateContext::replying_to(1);
        let neutral = render_template(&ProfileBrief::neutral(), context).expect("neutral template");
        let from_store = render_template(&stored, context).expect("stored template");
        let from_manual = render_template(&manual, context).expect("manual template");
        assert_eq!(neutral, NEUTRAL_REPLY, "{}: independent neutral", case.name);
        assert_eq!(from_store, case.expected, "{}: stored", case.name);
        assert_eq!(from_manual, case.expected, "{}: manual", case.name);
        if case.settings.is_empty() {
            assert_eq!(from_store, neutral);
        } else {
            assert_ne!(from_store, neutral, "{} must change the output", case.name);
        }
    }
}

/// A future accidental memory read must not alter today's profile-only
/// fallback, and a memory edit must not overwrite the stored voice.
#[test]
fn unrelated_memory_changes_do_not_change_the_profile_template_after_reopening() {
    let dir = tempfile::tempdir().expect("memory temporary store");
    let mut session = Session::open(dir.path());
    for (field, value) in [
        ("register", "formal"),
        ("directness", "direct"),
        ("warmth", "warm"),
        ("emoji_use", "never"),
    ] {
        session
            .set_voice(field, value)
            .expect("set synthetic profile voice");
    }
    let profile = session.profile().expect("profile before memory");
    let baseline = session
        .draft_pasted(PASTE)
        .expect("draft before memory")
        .text;
    assert_eq!(baseline, FORMAL_DIRECT_WARM_REPLY);
    let new = NewMemory {
        memory_type: "episodic".to_owned(),
        title: "合成记忆标题甲".to_owned(),
        summary: "合成记忆：周末整理蓝色资料盒。".to_owned(),
    };
    let created = session.write_memory(&new).expect("write unrelated memory");
    assert_eq!(created.title, new.title);
    assert_eq!(created.summary, new.summary);
    assert_eq!(
        session
            .draft_pasted(PASTE)
            .expect("draft after memory write")
            .text,
        baseline
    );
    let edited_title = "合成记忆标题乙";
    let edited_summary = "合成记忆：改在月底整理绿色资料盒。";
    let edited = session
        .edit_memory(
            &created.memory_id,
            &MemoryChange {
                title: Some(edited_title.to_owned()),
                summary: Some(edited_summary.to_owned()),
                ..MemoryChange::default()
            },
        )
        .expect("edit unrelated memory");
    assert_eq!(edited.title, edited_title);
    assert_eq!(edited.summary, edited_summary);
    assert_eq!(
        session.profile().expect("profile after memory edit"),
        profile
    );
    assert_eq!(
        session
            .draft_pasted(PASTE)
            .expect("draft after memory edit")
            .text,
        baseline
    );
    let prose = [
        PASTE,
        new.title.as_str(),
        new.summary.as_str(),
        edited_title,
        edited_summary,
        FORMAL_DIRECT_WARM_REPLY,
    ];
    let before = private_chain(&session, &prose);
    drop(session);

    let mut reopened = Session::open(dir.path());
    assert_eq!(private_chain(&reopened, &prose).entries, before.entries);
    assert_eq!(
        reopened
            .memory(&created.memory_id)
            .expect("edited memory survives"),
        edited
    );
    assert_eq!(
        reopened.profile().expect("profile survives memory edit"),
        profile
    );
    assert_unknown_axes(&reopened);
    assert_eq!(
        reopened
            .draft_pasted(PASTE)
            .expect("draft after memory reopen")
            .text,
        baseline
    );
    assert!(private_chain(&reopened, &prose)
        .entries
        .starts_with(&before.entries));
}

/// Rebuild must preserve a persisted user verdict, while release must restore
/// the machine value from that rebuild rather than a stale pre-import value.
#[test]
fn a_tie_correction_survives_reopening_and_new_import_until_released() {
    let dir = tempfile::tempdir().expect("tie temporary store");
    let corpus = fixtures::read_text("import/soul-import-v1/three_partners.jsonl")
        .expect("synthetic corpus");
    let followup = concat!(
        r#"{"type":"header","format":"soul-import-v1","version":1,"exported_at":"2026-08-24T12:00:00Z"}"#,
        "\n",
        r#"{"type":"message","id":"q2-followup-01","occurred_at":"2026-08-21T10:00:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"合成后续安排甲。"}"#,
        "\n",
        r#"{"type":"message","id":"q2-followup-02","occurred_at":"2026-08-21T10:01:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-lilei","text":"合成后续安排乙。"}"#,
        "\n",
        r#"{"type":"message","id":"q2-followup-03","occurred_at":"2026-08-22T10:00:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"合成后续安排甲。"}"#,
        "\n",
        r#"{"type":"message","id":"q2-followup-04","occurred_at":"2026-08-22T10:01:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-lilei","text":"合成后续安排乙。"}"#,
        "\n",
        r#"{"type":"message","id":"q2-followup-05","occurred_at":"2026-08-23T21:00:00Z","sender_scope":"self","conversation_id":"c-01","sender_id":"u-self","text":"合成后续安排甲。"}"#,
        "\n",
        r#"{"type":"message","id":"q2-followup-06","occurred_at":"2026-08-23T21:01:00Z","sender_scope":"third_party","conversation_id":"c-01","sender_id":"u-lilei","text":"合成后续安排乙。"}"#,
        "\n",
    );
    let mut session = Session::open(dir.path());
    session
        .commit_soul_import_v1(&corpus)
        .expect("initial synthetic import");
    let first = session.people().expect("initial graph");
    let candidates: Vec<_> = first
        .ties
        .iter()
        .filter(|edge| {
            edge.interaction_count == 6 && edge.outgoing_count == 3 && edge.incoming_count == 3
        })
        .collect();
    assert_eq!(
        candidates.len(),
        1,
        "fixture has one six-message reciprocal peer"
    );
    let initial = candidates[0];
    let id = initial.relationship_id.clone();
    assert_ne!(initial.band, "weak", "the user's correction must disagree");
    session.correct_tie(&id, "weak").expect("pin synthetic tie");
    let locked = tie(&session, &id);
    assert_eq!(locked.band, "weak");
    assert_eq!(locked.user_band.as_deref(), Some("weak"));
    assert_eq!(locked.machine_band.as_deref(), Some(initial.band.as_str()));
    assert!(locked.locked_by_user);
    let prose = [
        "合成后续安排甲。",
        "合成后续安排乙。",
        "公司门口",
        "café",
        "好的没问题",
    ];
    let before = private_chain(&session, &prose);
    drop(session);

    let mut reopened = Session::open(dir.path());
    assert_eq!(tie(&reopened, &id), locked);
    assert_eq!(private_chain(&reopened, &prose).entries, before.entries);
    reopened
        .commit_soul_import_v1(followup)
        .expect("new messages trigger rebuild");
    let rebuilt = tie(&reopened, &id);
    assert_eq!(
        reopened.people().expect("rebuilt graph").people.len(),
        first.people.len()
    );
    assert_eq!(rebuilt.interaction_count, initial.interaction_count + 6);
    assert_eq!(rebuilt.band, "weak");
    assert_eq!(rebuilt.user_band.as_deref(), Some("weak"));
    assert!(rebuilt.locked_by_user);
    let machine = rebuilt
        .machine_band
        .clone()
        .expect("rebuild keeps the machine reading");
    assert_ne!(
        machine, "weak",
        "release must visibly change the effective band"
    );
    assert_ne!(
        machine, initial.band,
        "new evidence must change the machine reading"
    );
    let rebuilt_chain = private_chain(&reopened, &prose);
    assert!(rebuilt_chain.entries.starts_with(&before.entries));
    reopened
        .release_tie(&id)
        .expect("release persisted correction after rebuild");
    let released = tie(&reopened, &id);
    assert_eq!(released.band, machine);
    assert_eq!(released.user_band, None);
    assert!(!released.locked_by_user);
    assert_eq!(released.interaction_count, rebuilt.interaction_count);
    let released_chain = private_chain(&reopened, &prose);
    assert!(released_chain.entries.starts_with(&rebuilt_chain.entries));
    assert_eq!(
        released_chain.entries.len(),
        rebuilt_chain.entries.len() + 1
    );
    drop(reopened);

    let after_release = Session::open(dir.path());
    assert_eq!(tie(&after_release, &id), released);
    assert_eq!(
        private_chain(&after_release, &prose).entries,
        released_chain.entries
    );
}
