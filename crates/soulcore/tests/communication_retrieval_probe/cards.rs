//! Truth is authored here, never supplied to either experimental strategy.
pub struct Scene {
    pub label: &'static str,
    pub query: &'static str,
    pub lexical: [&'static str; 2],
    pub paraphrase: [&'static str; 2],
    pub revision_old: &'static str,
    pub updated: &'static str,
    pub noise_title: &'static str,
    pub noise_summary: &'static str,
}

pub const SCENES: [Scene; 3] = [
    Scene {
        label: "coordination",
        query: "样机 交接",
        lexical: [
            "本事项甲组：样机交接定在周二，地点为北楼。",
            "本事项甲组：样机交接需要携带已签收的清单，不能替未确认人员答应到场。",
        ],
        paraphrase: [
            "本事项甲组：试制设备移交定在周二，地点为北楼。",
            "本事项甲组：设备转交时带上已签收的清单，不能替未确认人员答应到场。",
        ],
        revision_old: "本事项甲组旧约定：样机交接定在周二，地点为北楼。",
        updated: "本事项甲组更新：试制设备移交改到周四南楼，旧约定已取消；不能继续引用原安排。",
        noise_title: "其他事项乙组：样机 交接",
        noise_summary: "这条记录只谈乙组旧展会的样机交接，与甲组当前约定无关。",
    },
    Scene {
        label: "decline",
        query: "评审 志愿",
        lexical: [
            "本事项丙组：我决定谢绝评审志愿邀请，不承担本期工作。",
            "本事项丙组：回复评审志愿邀请只表达感谢，不解释私人缘由、不许诺下期参加。",
        ],
        paraphrase: [
            "本事项丙组：我决定不担任选拔活动的义工，不承担本期工作。",
            "本事项丙组：答复选拔活动邀请只表达感谢，不解释私人缘由、不许诺下期参加。",
        ],
        revision_old: "本事项丙组旧记录：我曾答应接受评审志愿邀请，参加本期选拔活动。",
        updated: "本事项丙组更新：之前答应参与选拔活动现已撤回，本期明确谢绝；不解释私人缘由。",
        noise_title: "其他事项丁组：评审 志愿",
        noise_summary: "这条记录只谈丁组往届评审志愿招募，与丙组本期邀请无关。",
    },
    Scene {
        label: "followup",
        query: "画册 校样",
        lexical: [
            "本事项戊组：上次约定收到画册校样后再核对封面。",
            "本事项戊组：跟进画册校样只询问是否方便给出进度，不设置回复期限、不推断对方态度。",
        ],
        paraphrase: [
            "本事项戊组：上次约定收到图录试印件后再核对封面。",
            "本事项戊组：续问图录试印件只询问是否方便给出进度，不设置回复期限、不推断对方态度。",
        ],
        revision_old: "本事项戊组旧约定：收到画册校样后再核对封面。",
        updated: "本事项戊组更新：图录试印件暂缓，本次只问封面资料何时方便提供，不沿用旧催稿约定。",
        noise_title: "其他事项己组：画册 校样",
        noise_summary: "这条记录只谈己组往年画册校样归档，与戊组本次封面约定无关。",
    },
];

pub struct Record {
    pub title: String,
    pub summary: String,
}

pub struct Card {
    pub scene: &'static str,
    pub variant: &'static str,
    pub position: &'static str,
    pub query: &'static str,
    pub records: Vec<Record>,
    pub order: Vec<usize>,
    pub required: [usize; 2],
    pub update: Option<usize>,
}

pub fn card(scene: &Scene, variant: &'static str, size: usize, position: &'static str) -> Card {
    assert!([3, 8, 16, 32].contains(&size));
    let necessary = match variant {
        "lexical_control" => scene.lexical,
        "paraphrase" => scene.paraphrase,
        "revision" => [scene.revision_old, scene.updated],
        _ => unreachable!(),
    };
    let mut records: Vec<_> = necessary
        .iter()
        .enumerate()
        .map(|(index, summary)| Record {
            title: format!("当前事项资料{index}"),
            summary: (*summary).into(),
        })
        .collect();
    for index in 2..size {
        let strong = index % 2 == 0;
        records.push(Record {
            title: if strong {
                scene.noise_title.into()
            } else {
                "其他事项庚组：仓储台账".into()
            },
            summary: format!(
                "无关资料编号{index}：{}",
                if strong {
                    scene.noise_summary
                } else {
                    "这条只谈庚组仓库登记，与当前沟通事项及对象均不同。"
                }
            ),
        });
    }
    let insertion = match position {
        "front" => 0,
        "middle" => (size - 2) / 2,
        "back" => size - 2,
        _ => unreachable!(),
    };
    let mut order: Vec<_> = (2..size).collect();
    order.splice(insertion..insertion, [0, 1]);
    // Three records cannot place an adjacent necessary pair at three distinct
    // offsets. The middle permutation straddles the one noise record.
    if size == 3 && position == "middle" {
        order = vec![0, 2, 1];
    }
    Card {
        scene: scene.label,
        variant,
        position,
        query: scene.query,
        records,
        order,
        required: [0, 1],
        update: (variant == "revision").then_some(1),
    }
}
