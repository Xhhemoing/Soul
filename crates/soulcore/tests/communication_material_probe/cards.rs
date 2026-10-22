//! Hand-authored synthetic facts and diagnostic expectations, not inferred truth.
pub struct Scene {
    pub label: &'static str,
    pub goal: &'static str,
    pub forbidden: &'static str,
    pub paste: &'static str,
    pub keyword: &'static str,
    pub normal: [&'static str; 3],
    pub paraphrase: [&'static str; 3],
    pub update: [&'static str; 3],
    pub missing: [&'static str; 3],
    pub missing_task_requirement: &'static str,
    pub missing_fact_label: &'static str,
}

pub const SCENES: [Scene; 3] = [
    Scene {
        label: "coordination",
        goal: "合成目标：先确认最新安排，再回复邀请。",
        forbidden: "合成限制：不得替我承诺尚未确定的时间。",
        paste: "合成来信：请确认这次安排。",
        keyword: "周五",
        normal: [
            "合成安排：周五讨论方案。",
            "合成条件：周五是否参加仍需本人确认。",
            "合成无关记录：早餐只喝温水。",
        ],
        paraphrase: [
            "合成安排：星期五讨论方案。",
            "合成条件：当日是否参加仍需本人确认。",
            "合成无关广告：周五商店促销。",
        ],
        update: [
            "合成旧安排：周五讨论方案。",
            "合成更新：先前安排取消，改到星期六。",
            "合成无关记录：早餐只喝温水。",
        ],
        missing: [
            "合成背景：有人提过周五聚会，但没有确认具体时间。",
            "合成无关记录：早餐只喝温水。",
            "合成无关记录：上周借了一本书。",
        ],
        missing_task_requirement: "合成任务要求：回复中准确引用双方已确认的具体开始时刻；不能补造，其他确定部分可以继续。",
        missing_fact_label: "confirmed_start_time",
    },
    Scene {
        label: "decline",
        goal: "合成目标：温和拒绝本次邀请。",
        forbidden: "合成限制：不解释私人原因，也不承诺下次。",
        paste: "合成来信：希望这次能够参加。",
        keyword: "拒绝",
        normal: [
            "合成意图：拒绝本次邀请。",
            "合成边界：拒绝时不承诺下次。",
            "合成无关记录：早餐只喝温水。",
        ],
        paraphrase: [
            "合成意图：这次不参加。",
            "合成边界：婉谢邀请，不约下一次。",
            "合成无关记录：拒绝推销电话。",
        ],
        update: [
            "合成旧边界：拒绝本次邀请，可以另约。",
            "合成新边界：不再作未来邀约，也不说明私人原因。",
            "合成无关记录：早餐只喝温水。",
        ],
        missing: [
            "合成背景：已决定拒绝本次邀请，但没有保存双方约定的取消截止日。",
            "合成无关记录：早餐只喝温水。",
            "合成无关记录：上周借了一本书。",
        ],
        missing_task_requirement: "合成任务要求：在明确婉拒中准确引用双方已约定的取消截止日；拒绝意图已确定，不需要重新询问。",
        missing_fact_label: "agreed_cancellation_deadline",
    },
    Scene {
        label: "relationship_followup",
        goal: "合成目标：核对背景后准备一次简短跟进。",
        forbidden: "合成限制：不推断对方心理，不把往来次数当作亲密证明。",
        paste: "合成来信：关于之前的事情想再沟通。",
        keyword: "联系",
        normal: [
            "合成事实：联系内容仅是工作协调。",
            "合成边界：联系时不询问私人原因。",
            "合成无关记录：早餐只喝温水。",
        ],
        paraphrase: [
            "合成事实：先前往来仅是工作协调。",
            "合成边界：再次问候时不询问私人原因。",
            "合成无关记录：联系商店领取包裹。",
        ],
        update: [
            "合成旧安排：可以联系讨论工作。",
            "合成更新：对方后来明确要求暂时不要再发消息。",
            "合成无关记录：早餐只喝温水。",
        ],
        missing: [
            "合成背景：过去有工作联系并约定跟进项目，但没有保存该项目名称。",
            "合成无关记录：早餐只喝温水。",
            "合成无关记录：上周借了一本书。",
        ],
        missing_task_requirement: "合成任务要求：简短跟进中准确引用上次约定的项目名；不知道项目名须核对，不推断对方意愿。",
        missing_fact_label: "previously_agreed_project_name",
    },
];

pub struct Card<'a> {
    pub scene: &'a Scene,
    pub variant: &'static str,
    pub notes: [&'static str; 3],
    pub required: &'static [usize],
    pub irrelevant: &'static [usize],
    pub counter: Option<usize>,
    pub unavailable_facts: Option<usize>,
    pub task_requirement: &'static str,
    pub missing_fact_label: Option<&'static str>,
    pub time_specific: bool,
    pub expected_query_indices: &'static [usize],
}

pub fn cards() -> Vec<Card<'static>> {
    SCENES
        .iter()
        .flat_map(|scene| {
            [
                Card {
                    scene,
                    variant: "normal",
                    notes: scene.normal,
                    required: &[0, 1],
                    irrelevant: &[2],
                    counter: None,
                    unavailable_facts: Some(0),
                    task_requirement: scene.goal,
                    missing_fact_label: None,
                    time_specific: false,
                    expected_query_indices: &[0, 1],
                },
                Card {
                    scene,
                    variant: "paraphrase_with_distractor",
                    notes: scene.paraphrase,
                    required: &[0, 1],
                    irrelevant: &[2],
                    counter: None,
                    unavailable_facts: Some(0),
                    task_requirement: scene.goal,
                    missing_fact_label: None,
                    time_specific: false,
                    expected_query_indices: &[2],
                },
                Card {
                    scene,
                    variant: "update_counterevidence",
                    notes: scene.update,
                    required: &[0, 1],
                    irrelevant: &[2],
                    counter: Some(1),
                    unavailable_facts: Some(0),
                    task_requirement: scene.goal,
                    missing_fact_label: None,
                    time_specific: false,
                    expected_query_indices: &[0],
                },
                Card {
                    scene,
                    variant: "unsupported_fact_time",
                    notes: scene.update,
                    required: &[],
                    irrelevant: &[2],
                    counter: None,
                    unavailable_facts: None,
                    task_requirement:
                        "合成能力探针：请求按事实有效时间选择材料；不据此评判语义时间推理。",
                    missing_fact_label: None,
                    time_specific: true,
                    expected_query_indices: &[],
                },
                Card {
                    scene,
                    variant: "missing_material",
                    notes: scene.missing,
                    required: &[],
                    irrelevant: &[1, 2],
                    counter: None,
                    unavailable_facts: Some(1),
                    task_requirement: scene.missing_task_requirement,
                    missing_fact_label: Some(scene.missing_fact_label),
                    time_specific: false,
                    expected_query_indices: &[0],
                },
            ]
        })
        .collect()
}
