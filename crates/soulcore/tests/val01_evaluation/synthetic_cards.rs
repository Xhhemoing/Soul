//! Five fixed synthetic cards; never accepts arbitrary pasted user material.
use super::{briefs::ManualVoice, endpoint::MODEL, records::*};
use soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE;
use soul_profile::{
    view::{AxisView, ProfileView},
    voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth},
};
use soul_schema::{common::EvidenceBand, profile::AxisPosition};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardId {
    S1,
    S2,
    S3,
    S4,
    S5,
}

pub(super) const CARDS: [CardId; 5] = [CardId::S1, CardId::S2, CardId::S3, CardId::S4, CardId::S5];

pub(super) struct Card {
    pub label: &'static str,
    pub instruction: &'static str,
    pub paste: &'static str,
    pub reply: &'static str,
    pub task: TaskType,
    pub semantic: SemanticStratum,
    pub contrast: Contrast,
    pub reverse: bool,
}

impl CardId {
    pub fn index(self) -> usize {
        match self {
            Self::S1 => 0,
            Self::S2 => 1,
            Self::S3 => 2,
            Self::S4 => 3,
            Self::S5 => 4,
        }
    }
    pub fn card(self) -> Card {
        match self {
            Self::S1 => Card {
                label: "S1",
                instruction: "补全通用确认正文槽，保留安排尚未确定，再评价整体语气；不评原文理解。",
                paste: "合成消息：请先确认是否收到，具体安排之后再讨论。",
                reply: "",
                task: TaskType::T1, semantic: SemanticStratum::NotApplicable,
                contrast: Contrast::AB, reverse: false,
            },
            Self::S2 => Card {
                label: "S2",
                instruction: "评价通用确认回复；不得自行给出日期、参加决定、时段或承诺。",
                paste: "合成消息：请在周五之前确认是否参加周六的讨论，时段尚未确定。",
                reply: "收到你的消息，谢谢告知。",
                task: TaskType::T2, semantic: SemanticStratum::NotApplicable,
                contrast: Contrast::AC, reverse: false,
            },
            Self::S3 => Card {
                label: "S3",
                instruction: "核对周五下午不可、周六上午可、请求确认及地点未定，不得称已经约定。",
                paste: "合成任务：拟回复协调邀请。周五下午不能参加；周六上午可以；请对方确认是否方便。地点还没有确定，不要承诺地点。",
                reply: "周五下午我不能参加，周六上午可以，请确认是否方便。地点还没有确定。",
                task: TaskType::T3, semantic: SemanticStratum::Coordination,
                contrast: Contrast::BC, reverse: true,
            },
            Self::S4 => Card {
                label: "S4",
                instruction: "核对今晚不接手、本周不承诺代办、不提供替代日期；不虚构理由。",
                paste: "合成任务：婉拒临时协助。今晚不能接手这项工作；本周也不能承诺代办；希望礼貌说明边界，不提出替代完成日期。",
                reply: "抱歉，今晚我不能接手这项工作，本周也不能承诺代办。谢谢理解。",
                task: TaskType::T3, semantic: SemanticStratum::BoundaryRefusal,
                contrast: Contrast::AB, reverse: true,
            },
            Self::S5 => Card {
                label: "S5",
                instruction: "核对讨论暂停、下一步未定、询问继续意愿及允许暂缓；不推断情绪关系。",
                paste: "合成任务：跟进一次未完成的讨论。上次讨论暂停了，目前没有确定下一步；想询问对方是否愿意继续；如果暂时不想继续，也可以稍后再说。不要推断对方情绪或关系状态。",
                reply: "上次讨论暂停后，我们还没有确定下一步。你愿意继续聊吗？如果暂时不想继续，也可以稍后再说。",
                task: TaskType::T3, semantic: SemanticStratum::RelationshipFollowup,
                contrast: Contrast::AC, reverse: false,
            },
        }
    }
    pub fn slot(self) -> RegisteredSlot {
        let card = self.card();
        let (mut left, mut right) = card.contrast.variants();
        if card.reverse {
            std::mem::swap(&mut left, &mut right);
        }
        RegisteredSlot {
            evaluation_run: "synthetic-five-card-run".into(),
            task_slot: self.index() as u32 + 1,
            pair_id: format!("synthetic-{}", card.label),
            task_type: card.task,
            semantic_stratum: card.semantic,
            contrast: card.contrast,
            left_variant: left,
            right_variant: right,
            presentation_seed: 1040 + self.index() as u64,
            conditions: Conditions {
                path: match card.task {
                    TaskType::T1 => Path::LocalTemplate,
                    TaskType::T2 => Path::EndpointPlaceholder,
                    TaskType::T3 => Path::EndpointOriginal,
                },
                model_id: if card.task == TaskType::T1 {
                    Reported::NotExposed
                } else {
                    Reported::Known(MODEL.into())
                },
                model_version: Reported::NotExposed,
                service_parameters: Reported::NotExposed,
                context_limit: Reported::NotExposed,
                redaction_condition: match card.task {
                    TaskType::T1 => RedactionCondition::LocalOnly,
                    TaskType::T2 => RedactionCondition::Placeholder,
                    TaskType::T3 => RedactionCondition::SingleUseOriginal,
                },
            },
        }
    }
}

pub(super) fn manual() -> ManualVoice {
    ManualVoice {
        register: VoiceRegister::Formal,
        directness: VoiceDirectness::Direct,
        warmth: VoiceWarmth::Warm,
        emoji_use: EmojiUse::Never,
    }
}

pub(super) fn snapshot() -> ProfileView {
    ProfileView {
        profile_id: Uuid::from_u128(1041),
        voice: VoiceProfile {
            warmth: VoiceWarmth::Cool,
            ..VoiceProfile::default()
        },
        notice: WORKING_HYPOTHESIS_NOTICE,
        axes: vec![AxisView {
            axis_id: Uuid::from_u128(1042),
            label: "合成轴".into(),
            position: AxisPosition::Mixed,
            evidence_band: EvidenceBand::Weak,
            locked_by_user: false,
            evidence: vec![],
            inferences: vec![],
            reading: "倾向核对安排后再继续沟通".into(),
        }],
    }
}
