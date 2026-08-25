/**
 * A stand-in for soulcore, wired into the real `@tauri-apps/api` transport.
 *
 * The shell's tests run the actual `core.ts` — argument marshalling included —
 * and only the far side of the IPC is faked. The values below mirror
 * `crates/soulcore/src/commands/shell.rs`; `contract.test.ts` reads that file
 * and fails if the two drift apart, which is what keeps this double from
 * quietly becoming a nicer core than the real one.
 */

import { mockIPC } from "@tauri-apps/api/mocks";
import { vi } from "vitest";

import type {
  AuditChain,
  CloudNotice,
  CollectStatus,
  ConfigSnapshot,
  Draft,
  DraftNotices,
  E1DraftPlan,
  FilesView,
  ForgetPreview,
  ForgetReceipt,
  ImportPreview,
  ImportReceipt,
  IntakeReceipt,
  MemoryChange,
  MemoryDetail,
  MemoryList,
  NewMemory,
  PeopleGraph,
  PersonSummary,
  PlanPreview,
  ProfileScreen,
  Question,
  ResearchPreview,
  SessionStatus,
  StatedRow,
} from "../core";

export const CLOUD_LABEL = "尚未启用";

/** `soul_draft::draft::NOT_SENT_NOTICE`, checked by `contract.test.ts`. */
export const NOT_SENT_NOTICE = "这是草稿。Soul 不会替你发送，检查和修改之后由你自己发出去。";

/** `soul_draft::draft::TEMPLATE_NOTICE`. */
export const TEMPLATE_NOTICE = "本次没有用模型：草稿由本机确定性语气模板生成，没有任何内容离开本机。";

/** `soulcore::commands::draft::E1_PLAN_NOTICE`, checked by `contract.test.ts`. */
export const E1_PLAN_NOTICE =
  "确认之后，会发到你自己配置的模型端点的是这些：模型名、一段固定的系统指令，以及一段引用材料——里面是你自己的档案摘要（口吻、口吻来源、有证据支持的要点，和「工作假设，非临床结论」那句），加上你粘贴的这一段。第三人正文默认已占位，只有你二次确认「这一条按原文带上」时才按原文发出，而且只这一次；姓名与账号两种情况下都占位。下面的段数、计划哈希与准备编号是给你核对用的，不在发出去的内容里。草稿生成之后仍然由你自己决定要不要发出去，Soul 不会替你发送。";

/** `soulcore::commands::fileplan::READ_ONLY_NOTICE`, checked the same way. */
export const READ_ONLY_NOTICE =
  "v0.1 只做只读扫描与计划预览，不会移动、重命名或删除任何文件。受控目录整理的执行与撤销是 v0.1.1 的事。";

/** `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE`. */
export const WORKING_HYPOTHESIS_NOTICE = "工作假设，非临床结论";

export const DRAFT_NOTICES: DraftNotices = {
  not_sent: NOT_SENT_NOTICE,
  e1_plan: E1_PLAN_NOTICE,
  can_send: false,
};

/** A session that has been through the wizard and has its store open. */
export const OPEN_SESSION: SessionStatus = {
  wizard_completed: true,
  store_opened: true,
  key_protection: "developer_key_file",
  store_notice: "这是开发构建：数据库密钥放在库旁边的种子文件里，没有平台密钥保护。",
  config_problem: null,
};

export const NO_ROOTS: FilesView = {
  read_only_notice: READ_ONLY_NOTICE,
  executable_in_this_version: false,
  roots: [],
  unavailable_roots: [],
};

/** One directory, and the plan a scan of it produced. */
export function aFilesView(overrides: Partial<FilesView> = {}): FilesView {
  return { ...NO_ROOTS, roots: [{ path: "/home/li/下载" }], ...overrides };
}

export function aPlanPreview(overrides: Partial<PlanPreview> = {}): PlanPreview {
  return {
    root: "/home/li/下载",
    plan_hash: "b3".repeat(32),
    directory_snapshot: "c4".repeat(32),
    disk_unchanged: true,
    executable_in_this_version: false,
    read_only_notice: READ_ONLY_NOTICE,
    scanned_entries: 3,
    skipped_entries: 1,
    truncated: false,
    moves: [
      { from: "预算.csv", to: "表格/预算.csv", kind: "spreadsheet", kind_label: "表格", size_bytes: 12 },
      { from: "photo.jpg", to: "图片/photo.jpg", kind: "image", kind_label: "图片", size_bytes: 8 },
    ],
    left_alone: [
      { path: "mystery.qqq", reason: "unknown_kind", explanation: "认不出这是什么类型的文件" },
    ],
    ...overrides,
  };
}

export const EMPTY_GRAPH: PeopleGraph = {
  self_contact_id: null,
  people: [],
  ties: [],
  notice: WORKING_HYPOTHESIS_NOTICE,
  third_party_data_is_local_only: true,
};

/**
 * A graph with one other person in it.
 *
 * Shaped the way `soulcore::commands::graph::PeopleGraphView` shapes one:
 * counts, an identifier digest, and no name anywhere.
 */
export function aPeopleGraph(overrides: Partial<PeopleGraph> = {}): PeopleGraph {
  return {
    self_contact_id: "0192f000-0000-7000-8000-000000000001",
    people: [
      {
        contact_id: "0192f000-0000-7000-8000-000000000001",
        is_you: true,
        identifier_hint: "aaaaaaaa",
        interaction_count: 6,
        last_contact_utc: "2026-08-20T09:00:00Z",
        tie_count: 1,
        forgotten: false,
      },
      {
        contact_id: "0192f000-0000-7000-8000-000000000002",
        is_you: false,
        identifier_hint: "bbbbbbbb",
        interaction_count: 6,
        last_contact_utc: "2026-08-20T09:00:00Z",
        tie_count: 1,
        forgotten: false,
      },
    ],
    ties: [
      {
        relationship_id: "0192f000-0000-7000-8000-00000000000a",
        from_contact_id: "0192f000-0000-7000-8000-000000000001",
        to_contact_id: "0192f000-0000-7000-8000-000000000002",
        types: ["direct", "reciprocal"],
        band: "moderate",
        interaction_count: 6,
        outgoing_count: 3,
        incoming_count: 3,
        conversation_count: 2,
        active_day_count: 4,
        first_contact_utc: "2026-07-01T08:00:00Z",
        last_contact_utc: "2026-08-20T09:00:00Z",
        local_only: true,
        evidence: [
          { evidence_id: "0192f000-0000-7000-8000-0000000000e1", kind: "message", method: "heuristic", strength: "moderate" },
          { evidence_id: "0192f000-0000-7000-8000-0000000000e2", kind: "message", method: "heuristic", strength: "moderate" },
        ],
      },
    ],
    notice: WORKING_HYPOTHESIS_NOTICE,
    third_party_data_is_local_only: true,
    ...overrides,
  };
}

/** A summary the way `soul-draft` builds one: counts, and what backs them. */
export function aPersonSummary(overrides: Partial<PersonSummary> = {}): PersonSummary {
  return {
    contact_id: "0192f000-0000-7000-8000-000000000002",
    source: "counts",
    text: "这个人：一共 6 次往来，其中你发出 3 次。工作假设，非临床结论",
    points: [
      {
        statement: "一共 6 次往来，分布在 4 天里",
        evidence_ids: [
          "0192f000-0000-7000-8000-0000000000e1",
          "0192f000-0000-7000-8000-0000000000e2",
        ],
        band: "moderate",
      },
    ],
    notice: WORKING_HYPOTHESIS_NOTICE,
    clinical_claim: false,
    ...overrides,
  };
}

/** What `prepare_draft` answers with: the shape of a request, never its text. */
export function anE1Plan(overrides: Partial<E1DraftPlan> = {}): E1DraftPlan {
  return {
    preparation_id: "0192f000-0000-7000-8000-0000000000f1",
    plan_hash: "d5".repeat(32),
    model: "unnamed-model",
    third_party_turns: 1,
    placeheld_turns: 1,
    carries_exempted_original: false,
    notice: E1_PLAN_NOTICE,
    not_sent_notice: NOT_SENT_NOTICE,
    ...overrides,
  };
}

/**
 * A draft the way the local path produces one.
 *
 * The counts are zero because that path builds no request body at all, which
 * is `soul-draft`'s `BodyFacts::default` and not a rounding of something.
 */
export function aTemplateDraft(overrides: Partial<Draft> = {}): Draft {
  return {
    text: "收到，我看一下，晚点回你。\n（这里写你要说的内容）",
    source: "tone_template",
    delivery: false,
    third_party_turns: 0,
    placeheld_turns: 0,
    carries_exempted_original: false,
    degraded: null,
    injection_signals: [],
    not_sent_notice: NOT_SENT_NOTICE,
    source_notice: TEMPLATE_NOTICE,
    ...overrides,
  };
}

/** `soulcore::commands::memory::FORGET_NOTICE`, checked by `contract.test.ts`. */
export const FORGET_NOTICE =
  "遗忘销毁的是这条记忆的内容密钥：正文从此打不开，行会留成一块墓碑，引用过它的推断会被标成失去依据。这一步不可撤销，也不写任何文件。这不是把磁盘块擦干净：SSD 上可能还留着旧密文，只是没有密钥再也打不开。";

/** `soulcore::commands::store::RESEARCH_PREVIEW_NOTICE`. */
export const RESEARCH_PREVIEW_NOTICE =
  "研究预览只在这块屏幕上存在：它不落盘、不出网，关掉这一页它就没有了。行是按事件类型与小时桶聚合出来的计数，别人的数据在查询里就被排除掉了。";

/** `soulcore::commands::store::AUDIT_CHAIN_NOTICE`. */
export const AUDIT_CHAIN_NOTICE =
  "审计链只记「发生过什么」，不记内容：一条记录里能有的只有动作、结论、理由码、涉及到的编号和计数。每一条都带着上一条的哈希，所以中间被人改过或者抽掉一条，回放的时候就对不上。";

/** `soulcore::commands::import::IMPORT_LOCAL_ONLY_NOTICE`. */
export const IMPORT_LOCAL_ONLY_NOTICE =
  "导入全程在本机：文件由你自己挑，内容读进来就地加密入库，不上传，也不会被当成指令执行。导入的正文一律当数据看待。";

/** What one export file contains, the way the core counts it. */
export function anImportPreview(overrides: Partial<ImportPreview> = {}): ImportPreview {
  return {
    source: "soul-import-v1",
    participants: 4,
    conversations: 2,
    messages: 16,
    messages_with_injection_markers: 1,
    owner_identified: true,
    writes_anything: false,
    notice: IMPORT_LOCAL_ONLY_NOTICE,
    ...overrides,
  };
}

export function anImportReceipt(overrides: Partial<ImportReceipt> = {}): ImportReceipt {
  return {
    source: "soul-import-v1",
    contacts_created: 4,
    contacts_matched: 0,
    events_written: 16,
    evidence_written: 16,
    messages_with_injection_markers: 1,
    ties_rebuilt: 3,
    notice: IMPORT_LOCAL_ONLY_NOTICE,
    ...overrides,
  };
}

/** `soulcore::commands::session::NO_ANSWERS_NOTICE`. */
export const NO_ANSWERS_NOTICE =
  "这份问卷一道题都没有答，所以没有东西可以写进档案。随便答一道都行，没答的那些会留成「还看不出方向」，不会被猜。";

/**
 * The eleven questions, shaped the way `profile::question_views` shapes them.
 *
 * Kept short of the real prompts on purpose — `contract.test.ts` checks the
 * count and the ids against the canonical Rust list, so a question added there
 * fails here rather than being silently undrawn.
 */
export const QUESTIONS: readonly Question[] = [
  ...["curiosity", "orderliness", "social_energy", "accommodation", "emotional_steadiness"].map(
    (axis) => ({
      question_id: `q.axis.${axis}`,
      prompt: `关于「${axis}」这条轴，你更偏哪一边？`,
      moves: "axis" as const,
      options: [
        { value: "leans_low", reading: "偏这一端" },
        { value: "mixed", reading: "两端都有，看场合" },
        { value: "leans_high", reading: "偏那一端" },
      ],
      prose: false,
    }),
  ),
  {
    question_id: "q.voice.register",
    prompt: "给不太熟的人写消息时，你的语气偏正式还是偏随意？",
    moves: "voice",
    options: [
      { value: "casual", reading: "随意" },
      { value: "plain", reading: "平实" },
      { value: "formal", reading: "正式" },
    ],
    prose: false,
  },
  {
    question_id: "q.voice.directness",
    prompt: "写消息时，你更常直说，还是先铺垫？",
    moves: "voice",
    options: [
      { value: "reserved", reading: "含蓄" },
      { value: "balanced", reading: "适中" },
      { value: "direct", reading: "直接" },
    ],
    prose: false,
  },
  {
    question_id: "q.voice.emoji_use",
    prompt: "你平时用表情符号多吗？",
    moves: "voice",
    options: [
      { value: "never", reading: "不用表情" },
      { value: "sparing", reading: "偶尔用表情" },
      { value: "frequent", reading: "经常用表情" },
    ],
    prose: false,
  },
  {
    question_id: "q.boundary.topics",
    prompt: "有哪些话题，你不希望 Soul 替你起草或分析？",
    moves: "boundary",
    options: [],
    prose: true,
  },
  {
    question_id: "q.boundary.availability",
    prompt: "什么时间段你基本不回消息？",
    moves: "boundary",
    options: [],
    prose: true,
  },
  {
    question_id: "q.value.what_matters",
    prompt: "有没有一件事，是你希望 Soul 无论如何都替你守住的？",
    moves: "value",
    options: [],
    prose: true,
  },
];

/** What one recorded answer leaves behind. AC-03 is `profile_is_empty`. */
export function anIntakeReceipt(overrides: Partial<IntakeReceipt> = {}): IntakeReceipt {
  return {
    answered: 1,
    axes_known: 1,
    axes_unknown: 4,
    voice_fields_user_set: 0,
    stated_entries: 0,
    profile_is_empty: false,
    evidence_ids: ["0192f000-0000-7000-8000-0000000000d1"],
    ...overrides,
  };
}

const AXIS_ID = "0192b0c0-5001-7a01-8b01-000000000001";

/** A profile with one axis answered and four left alone. */
export function aProfileScreen(overrides: Partial<ProfileScreen> = {}): ProfileScreen {
  return {
    profile_id: "0192b0c0-5001-7c01-8c01-000000000001",
    axes: [
      {
        axis_id: AXIS_ID,
        label: "好奇与开放",
        position: "leans_high",
        reading: "好奇与开放：偏向尝试新的做法",
        evidence_band: "moderate",
        evidence_count: 1,
        locked_by_user: false,
        choices: [
          { position: "leans_low", reading: "偏向熟悉稳妥的做法" },
          { position: "mixed", reading: "两端都有，看场合" },
          { position: "leans_high", reading: "偏向尝试新的做法" },
        ],
        inferences: [],
      },
      {
        axis_id: "0192b0c0-5001-7a02-8b02-000000000002",
        label: "条理与执行",
        position: "unknown",
        reading: "条理与执行：还看不出方向",
        evidence_band: "none",
        evidence_count: 0,
        locked_by_user: false,
        choices: [
          { position: "leans_low", reading: "偏向随性推进" },
          { position: "mixed", reading: "两端都有，看场合" },
          { position: "leans_high", reading: "偏向先规划再动手" },
        ],
        inferences: [
          {
            inference_id: "0192f000-0000-7000-8000-0000000000c1",
            position: "leans_low",
            band: "weak",
            evidence_count: 1,
            state: "live",
            falsifier: null,
          },
        ],
      },
    ],
    voice: {
      fields: [
        {
          field: "register",
          label: "语域",
          value: "plain",
          locked_by_user: false,
          options: [
            { value: "casual", reading: "随意" },
            { value: "plain", reading: "平实" },
            { value: "formal", reading: "正式" },
          ],
          question_id: "q.voice.register",
        },
      ],
      reading: "平实、适中、平和、偶尔用表情",
    },
    stated: [
      {
        field: "boundary",
        question_id: "q.boundary.topics",
        prompt: "有哪些话题，你不希望 Soul 替你起草或分析？",
        event_id: "0192f000-0000-7000-8000-0000000000b1",
        evidence_id: "0192f000-0000-7000-8000-0000000000b2",
      },
    ],
    reading: "语气：平实、适中、平和、偶尔用表情\n好奇与开放：偏向尝试新的做法｜证据中，1条\n工作假设，非临床结论",
    positions: ["leans_low", "mixed", "leans_high"],
    notice: WORKING_HYPOTHESIS_NOTICE,
    ...overrides,
  };
}

/**
 * Which axis in `aProfileScreen` each axis question moves.
 *
 * The core reads the pairing off `soul_import::questionnaire`; the double has
 * to be told, because the question fixture and the profile fixture are written
 * apart. `aProfileScreen` carries two of the five axes, so the other three
 * axis questions are recorded and move nothing this fixture can show — which
 * is the same thing the real core does when it is asked for a screen: it shows
 * the rows it has.
 */
const AXIS_OF_QUESTION: Record<string, string> = {
  "q.axis.curiosity": AXIS_ID,
  "q.axis.orderliness": "0192b0c0-5001-7a02-8b02-000000000002",
};

/**
 * The profile after a questionnaire, moved the way the core moves one.
 *
 * An axis the user answered for takes that position and the axis's own words
 * for it; a voice field whose question was answered takes the value and locks;
 * a prose answer leaves a `StatedRow`, which is a pointer and holds none of
 * what was typed. A blank moves nothing — it is a skip, not an erasure — and
 * answering the same question twice replaces rather than appends.
 *
 * What this does not do is recompose `reading`. That prose is built in Rust
 * where the denylist can see it, so the double leaves the sentence it was
 * given rather than inventing a new one on this side.
 */
function profileAfter(
  current: ProfileScreen,
  answers: readonly { question_id: string; given: string }[],
  questions: readonly Question[],
): ProfileScreen {
  let moved = current;
  for (const answer of answers) {
    const given = answer.given.trim();
    if (given === "") continue;
    const question = questions.find((one) => one.question_id === answer.question_id);
    const axisId = AXIS_OF_QUESTION[answer.question_id];
    let stated = moved.stated;
    if (question !== undefined && (question.moves === "boundary" || question.moves === "value")) {
      const at = questions.indexOf(question);
      const row: StatedRow = {
        field: question.moves,
        question_id: question.question_id,
        prompt: question.prompt,
        event_id: `0192f000-0000-7000-8000-${(0xb00 + at * 2).toString(16).padStart(12, "0")}`,
        evidence_id: `0192f000-0000-7000-8000-${(0xb01 + at * 2).toString(16).padStart(12, "0")}`,
      };
      stated = [...stated.filter((one) => one.question_id !== row.question_id), row];
    }
    moved = {
      ...moved,
      axes: moved.axes.map((axis) =>
        axis.axis_id !== axisId
          ? axis
          : {
              ...axis,
              position: given,
              reading: `${axis.label}：${
                axis.choices.find((choice) => choice.position === given)?.reading ?? given
              }`,
              evidence_band: "weak",
              evidence_count: axis.evidence_count + 1,
            },
      ),
      voice: {
        ...moved.voice,
        fields: moved.voice.fields.map((field) =>
          field.question_id === answer.question_id
            ? { ...field, value: given, locked_by_user: true }
            : field,
        ),
      },
      stated,
    };
  }
  return moved;
}

export const NO_MEMORIES: MemoryList = {
  memories: [],
  memory_types: ["episodic", "semantic", "procedural", "preference", "commitment"],
  forget_notice: FORGET_NOTICE,
};

const MEMORY_ID = "0192f000-0000-7000-8000-0000000000a1";

export function aMemoryList(overrides: Partial<MemoryList> = {}): MemoryList {
  return {
    ...NO_MEMORIES,
    memories: [
      {
        memory_id: MEMORY_ID,
        memory_type: "episodic",
        forget_state: "active",
        third_party_content_present: false,
        title_chars: 6,
        summary_chars: 24,
      },
    ],
    ...overrides,
  };
}

export function aMemoryDetail(overrides: Partial<MemoryDetail> = {}): MemoryDetail {
  return {
    memory_id: MEMORY_ID,
    memory_type: "episodic",
    title: "搬家那天",
    summary: "下午三点交的钥匙，晚上在新厨房煮了面。",
    third_party_content_present: false,
    content_key_id: "0192f000-0000-7000-8000-0000000000k1",
    ...overrides,
  };
}

export const PREVIEW_ID = "0192f000-0000-7000-8000-0000000000p1";

export function aForgetPreview(overrides: Partial<ForgetPreview> = {}): ForgetPreview {
  return {
    preview_id: PREVIEW_ID,
    memory_id: MEMORY_ID,
    content_key_count: 1,
    memories_affected: 1,
    contacts_affected: 0,
    sealed_blobs_destroyed: 2,
    inferences_orphaned: 1,
    audit_entries_retained: 3,
    destroys_anything: false,
    notice: FORGET_NOTICE,
    ...overrides,
  };
}

export function aForgetReceipt(overrides: Partial<ForgetReceipt> = {}): ForgetReceipt {
  return {
    memory_id: MEMORY_ID,
    content_keys_destroyed: 1,
    sealed_blobs_destroyed: 2,
    inferences_orphaned: 1,
    matched_preview: true,
    ...overrides,
  };
}

/**
 * A preview of what research would see: counts and buckets.
 *
 * `written_to_disk` and `third_party_rows` are the literals the Rust types
 * pin, so a double that softened either of them would not compile.
 */
export function aResearchPreview(overrides: Partial<ResearchPreview> = {}): ResearchPreview {
  return {
    manifest_id: "0192f000-0000-7000-8000-0000000000m1",
    export_kind: "research_preview",
    written_to_disk: false,
    third_party_rows: 0,
    candidate_rows_total: 4,
    third_party_rows_excluded: 2,
    fields: ["event_kind", "time_bucket_utc", "aggregate_count"],
    rows: [
      {
        event_kind: "app_usage",
        time_bucket_utc: "2026-08-20T09:00:00Z",
        duration_bucket: null,
        self_trait_axis: null,
        self_trait_band: null,
        aggregate_count: 2,
      },
    ],
    third_party_body: "excluded",
    notice: RESEARCH_PREVIEW_NOTICE,
    ...overrides,
  };
}

export const EMPTY_CHAIN: AuditChain = {
  entries: [],
  verified: true,
  verification_problem: null,
  notice: AUDIT_CHAIN_NOTICE,
};

export function anAuditChain(overrides: Partial<AuditChain> = {}): AuditChain {
  return {
    ...EMPTY_CHAIN,
    entries: [
      {
        seq: 1,
        entry_id: "0192f000-0000-7000-8000-0000000000e9",
        ts: "2026-08-20T09:00:00Z",
        action: "memory.write",
        decision: "allowed",
        reason_code: null,
        subject_refs: [MEMORY_ID],
        items: 1,
        bytes: null,
        plan_hash: null,
        capability_token_id: null,
        egress_class: "none",
        prev_hash: "00".repeat(32),
        entry_hash: "11".repeat(32),
        follows_previous: true,
      },
      {
        seq: 2,
        entry_id: "0192f000-0000-7000-8000-0000000000ea",
        ts: "2026-08-20T09:05:00Z",
        action: "file.plan",
        decision: "allowed",
        reason_code: null,
        subject_refs: [],
        items: 3,
        bytes: 20,
        plan_hash: "b3".repeat(32),
        capability_token_id: "0192f000-0000-7000-8000-0000000000t1",
        egress_class: "none",
        prev_hash: "11".repeat(32),
        entry_hash: "22".repeat(32),
        follows_previous: true,
      },
    ],
    ...overrides,
  };
}

/** `soulcore::commands::session::COLLECT_DURATION_ONLY_NOTICE`. */
export const COLLECT_DURATION_ONLY_NOTICE =
  "这一版的采集只记一样东西：哪个应用在前台，以及它在前台待了多久。窗口标题不记，文件内容不记，键盘和剪贴板连代码路径都没有。应用名和时长一起密封在库里，跟着这一次采集的内容密钥走。";

/** `soulcore::commands::session::COLLECT_OFF_NOTICE`. */
export const COLLECT_OFF_NOTICE = "采集现在是关的：没有给出同意，也没有采集线程在跑。";

/** `soulcore::commands::session::COLLECT_RUNNING_NOTICE`. */
export const COLLECT_RUNNING_NOTICE =
  "采集正在进行：后台线程在按秒看前台是哪个应用，换了应用就把上一段的时长写成一条记录。按「停止采集」之后 1 秒内不会再有新的记录。";

/** `soulcore::commands::session::COLLECT_NOT_OBSERVING_NOTICE`. */
export const COLLECT_NOT_OBSERVING_NOTICE =
  "同意已经记下来了，但这台机器上没有东西在采：v0.1 只在 Windows 上看前台，别的平台上给出的同意就只是同意，不会去看任何窗口。";

/**
 * Collection as a fresh launch finds it: nobody has consented, so nothing is
 * running. `survives_restart` is the literal `false` in both the Rust type and
 * the TypeScript one — a double that softened it would not compile.
 */
export const COLLECT_OFF: CollectStatus = {
  consent_granted: false,
  collector_running: false,
  source: "windows.foreground_process",
  events_collected: 0,
  survives_restart: false,
  duration_only_notice: COLLECT_DURATION_ONLY_NOTICE,
  notice: COLLECT_OFF_NOTICE,
};

export function aCollectStatus(overrides: Partial<CollectStatus> = {}): CollectStatus {
  return { ...COLLECT_OFF, ...overrides };
}

/** A collector that is actually running, the way a Windows machine reports it. */
export const COLLECT_RUNNING: CollectStatus = aCollectStatus({
  consent_granted: true,
  collector_running: true,
  events_collected: 3,
  notice: COLLECT_RUNNING_NOTICE,
});

export const CLOSED_CLOUD: CloudNotice = {
  state: "not_yet_available",
  label: CLOUD_LABEL,
  enabled: false,
  performs_network_request: false,
  explanation:
    "v0.1 没有云端出网的代码路径。开关留在这里是为了让你看见它默认是关的，点它不会发出任何请求，也不会把任何内容送出本机。",
};

/** `soulcore::commands::shell::LLM_ENDPOINT_SESSION_ONLY_NOTICE`. */
export const LLM_ENDPOINT_SESSION_ONLY_NOTICE =
  "地址只在这次运行里有效，退出 Soul 再打开需要重新填写。填写的时候不会访问这个地址，只有你在起草页确认生成、或在人脉图上看某个人的摘要时才会。";

/** `soulcore::commands::session::ENDPOINT_UNPARSABLE_NOTICE`. */
export const ENDPOINT_UNPARSABLE_NOTICE =
  "这个地址不像一个端点：要 http:// 或 https:// 开头，后面跟主机名，端口不写就按 80 或 443 算，比如 http://127.0.0.1:11434/v1；地址里不能带用户名和密码。这一次什么都没有保存，端点还是没有填写。";

export const CLOSED_SNAPSHOT: ConfigSnapshot = {
  collect_enabled: false,
  cloud: CLOSED_CLOUD,
  llm_endpoint_configured: false,
  llm_endpoint_notice: LLM_ENDPOINT_SESSION_ONLY_NOTICE,
  authorized_root_count: 0,
  fully_closed: true,
  open_capabilities: [],
};

/**
 * The snapshot that belongs beside [`COLLECT_RUNNING`].
 *
 * `Session::grant_collection` writes the in-memory `Config` the way
 * `set_user_endpoint` does, so a core that reports a collector running and a
 * snapshot saying 全部关闭 is a state no real IPC can produce. A test that
 * wants the ledger open hands both.
 */
export const COLLECTING_SNAPSHOT: ConfigSnapshot = withCollect(CLOSED_SNAPSHOT, true);

export interface RecordedCall {
  readonly cmd: string;
  readonly payload: unknown;
}

export interface FakeCore {
  /** Every command the shell sent, in order. */
  readonly calls: RecordedCall[];
  callsTo(cmd: string): RecordedCall[];
}

/**
 * Install the double.
 *
 * `complete_wizard` refuses an unacknowledged wizard the way the core does,
 * because the UI has to handle the refusal; everything else is a constant,
 * because everything else in this work package is.
 */
/**
 * How the double answers the commands whose answer a test cares about.
 *
 * `drafting` is a function of the paste rather than a constant so a test can
 * assert on what the shell actually sent, and can throw to make the core
 * refuse the way a real refusal arrives — as a value, not an exception type.
 */
export interface FakeCoreOptions {
  readonly snapshot?: ConfigSnapshot;
  readonly status?: SessionStatus;
  readonly drafting?: (pasted: string) => Draft;
  readonly files?: FilesView;
  readonly authorizing?: (path: string) => FilesView;
  readonly planning?: (path: string) => PlanPreview;
  readonly graph?: PeopleGraph;
  /** As `graph`, but able to throw the way a refusal arrives — as a value. */
  readonly graphing?: () => PeopleGraph;
  readonly summarizing?: (contactId: string) => PersonSummary;
  /**
   * How the double answers 用你自己的模型端点写.
   *
   * `includeOriginal` is the user's second confirmation, and the double is
   * handed it so a test can answer the way the core would: a placeheld plan
   * until it is set, and one that carries the original after. A double that
   * ignored it could not tell the two apart, which is the whole of AC-13 on
   * this side.
   */
  readonly preparing?: (pasted: string, includeOriginal: boolean) => E1DraftPlan;
  readonly generating?: (approval: { preparation_id: string; plan_hash: string }) => Draft;
  /** Able to throw: a file that does not parse arrives as a refusal value. */
  readonly readingImport?: (format: string, text: string) => ImportPreview;
  readonly importing?: (format: string, text: string) => ImportReceipt;
  readonly questions?: readonly Question[];
  /** Able to throw, because a questionnaire with nothing in it is refused. */
  readonly recording?: (answers: readonly { question_id: string; given: string }[]) =>
    | IntakeReceipt
    | never;
  readonly profile?: () => ProfileScreen;
  readonly correcting?: (axisId: string, position: string) => ProfileScreen;
  readonly voicing?: (field: string, option: string) => ProfileScreen;
  readonly memories?: () => MemoryList;
  readonly opening?: (memoryId: string) => MemoryDetail;
  readonly writing?: (memory: NewMemory) => MemoryDetail;
  readonly editing?: (memoryId: string, change: MemoryChange) => MemoryDetail;
  readonly pricing?: (memoryId: string) => ForgetPreview;
  readonly forgetting?: (confirmation: {
    preview_id: string;
    memory_id: string;
  }) => ForgetReceipt;
  readonly research?: () => ResearchPreview;
  readonly audit?: () => AuditChain;
  /** The collection state this launch starts in. Nobody has consented yet. */
  readonly collect?: CollectStatus;
  /**
   * What one `collect_status` read does to the state it reads.
   *
   * The real core recounts the table on every read, so a page that stays put
   * while a collector runs sees a number that moves without anything on this
   * side having asked for it. A double that answered a constant could not tell
   * a page that rereads from one that froze on mount. The default is identity:
   * reading changes nothing unless a test says it does.
   */
  readonly rereading?: (current: CollectStatus) => CollectStatus;
  /**
   * How the double answers 开始采集 / 停止采集.
   *
   * Stateful by default, because the page's whole subject is a state that
   * changes: the fallbacks below mirror what the core does — consent is
   * recorded either way, and a collector only starts when there is a
   * foreground source to feed it. Both can throw, which is how a refusal
   * arrives.
   */
  readonly granting?: (current: CollectStatus) => CollectStatus;
  readonly revoking?: (current: CollectStatus) => CollectStatus;
  /**
   * How the double answers 保存端点.
   *
   * Stateful for the same reason collection is: the page's subject is a state
   * that changes. It can throw, which is how the core's refusal of something
   * that is not an address arrives.
   */
  readonly endpointing?: (url: string, current: ConfigSnapshot) => ConfigSnapshot;
}

/** Consent recorded; a collector only where there is a desktop to watch. */
function grantedFrom(current: CollectStatus): CollectStatus {
  const observing = current.source !== "unsupported";
  return {
    ...current,
    consent_granted: true,
    collector_running: observing,
    notice: observing ? COLLECT_RUNNING_NOTICE : COLLECT_NOT_OBSERVING_NOTICE,
  };
}

function revokedFrom(current: CollectStatus): CollectStatus {
  return {
    ...current,
    consent_granted: false,
    collector_running: false,
    notice: COLLECT_OFF_NOTICE,
  };
}

/**
 * The same move for collection, because on the core side it is the same
 * in-memory field: `Session::grant_collection` sets `Config.collect_enabled`
 * once the ledger has recorded the grant, and revoking clears it. Consent is
 * what moves it rather than the collector thread — a machine with no
 * foreground source has still had the capability opened on it — which is why
 * this is driven off `consent_granted` and not off `collector_running`.
 */
function withCollect(current: ConfigSnapshot, enabled: boolean): ConfigSnapshot {
  const others = current.open_capabilities.filter((name) => name !== "collect_enabled");
  const open = enabled ? [...others, "collect_enabled"] : others;
  return {
    ...current,
    collect_enabled: enabled,
    open_capabilities: open,
    fully_closed: open.length === 0,
  };
}

/** And once directories have been authorized, which is a count as well. */
function withRoots(current: ConfigSnapshot, count: number): ConfigSnapshot {
  const others = current.open_capabilities.filter((name) => name !== "authorized_roots");
  const open = count > 0 ? [...others, "authorized_roots"] : others;
  return {
    ...current,
    authorized_root_count: count,
    open_capabilities: open,
    fully_closed: open.length === 0,
  };
}

/**
 * The snapshot the core answers with once an endpoint is there, or once it is
 * not: a boolean, the capability list it belongs on, and 全部关闭 following
 * from that list being empty. The address is in none of them.
 */
function withEndpoint(current: ConfigSnapshot, configured: boolean): ConfigSnapshot {
  const others = current.open_capabilities.filter((name) => name !== "llm_endpoint");
  const open = configured ? [...others, "llm_endpoint"] : others;
  return {
    ...current,
    llm_endpoint_configured: configured,
    open_capabilities: open,
    fully_closed: open.length === 0,
  };
}

/**
 * Enough of `Origin::parse` to refuse what the core refuses.
 *
 * Not the whole parser: port syntax and IPv6 brackets are the Rust side's, and
 * `crates/soulcore/tests/session_e1.rs` is where they are tested. What is
 * mirrored is the part the page's behaviour depends on — an address that is not
 * one comes back as a refusal rather than as a save — because a double that
 * accepted anything would leave the refusal path on this page unexercised.
 */
function looksLikeAnOrigin(url: string): boolean {
  const authority = /^https?:\/\/([^/?#]+)/i.exec(url.trim())?.[1];
  return authority !== undefined && !authority.includes("@");
}

function savedEndpoint(url: string, current: ConfigSnapshot): ConfigSnapshot {
  if (!looksLikeAnOrigin(url)) {
    throw { reason_code: "EGRESS_TARGET_UNPARSABLE", explanation: ENDPOINT_UNPARSABLE_NOTICE };
  }
  return withEndpoint(current, true);
}

export function installFakeCore(
  snapshotOrOptions: ConfigSnapshot | FakeCoreOptions = CLOSED_SNAPSHOT,
): FakeCore {
  const options: FakeCoreOptions =
    "collect_enabled" in snapshotOrOptions ? { snapshot: snapshotOrOptions } : snapshotOrOptions;
  const snapshot = options.snapshot ?? CLOSED_SNAPSHOT;
  const status = options.status ?? OPEN_SESSION;
  const drafting = options.drafting ?? (() => aTemplateDraft());
  const files = options.files ?? NO_ROOTS;
  const questions = options.questions ?? QUESTIONS;
  const calls: RecordedCall[] = [];
  /** The three pieces of state the double keeps, because the core keeps them
   *  too: a consent ledger, an endpoint that lives for one run, and a profile
   *  that a questionnaire or a correction writes to and the next read sees. */
  let collect = options.collect ?? COLLECT_OFF;
  let configuration = snapshot;
  let profile: ProfileScreen | null = null;

  /** Read on demand, so a `profile` option that refuses still refuses. */
  const profileNow = (): ProfileScreen => {
    profile ??= (options.profile ?? (() => aProfileScreen()))();
    return profile;
  };

  mockIPC((cmd, payload) => {
    calls.push({ cmd, payload });
    switch (cmd) {
      case "config_snapshot":
        return configuration;
      case "session_status":
        return status;
      case "complete_wizard": {
        const answers = (payload as { answers?: { acknowledged_defaults_are_off?: boolean } })
          .answers;
        if (answers?.acknowledged_defaults_are_off !== true) {
          throw "the wizard was not acknowledged, so there is nothing to finish";
        }
        return snapshot;
      }
      case "cloud_toggle":
        // Whatever was requested, the answer is the notice. This is the whole
        // of AC-22 on the core side, and the shell must not improve on it.
        return snapshot.cloud;
      case "files_view":
        return files;
      case "authorize_directory": {
        const authorized = (options.authorizing ?? (() => files))(
          (payload as { path?: string }).path ?? "",
        );
        // `Session::authorize` pushes the canonical root onto the session's
        // `Config`, so the snapshot a later `config_snapshot` answers with has
        // grown a directory. The double does the same, or the overview's count
        // would look correct here while being a launch-time number in a
        // shipped build.
        configuration = withRoots(configuration, authorized.roots.length);
        return authorized;
      }
      case "preview_plan":
        return (options.planning ?? (() => aPlanPreview()))(
          (payload as { path?: string }).path ?? "",
        );
      case "people_graph":
        return (options.graphing ?? (() => options.graph ?? EMPTY_GRAPH))();
      case "person_summary":
        return (options.summarizing ?? (() => aPersonSummary()))(
          (payload as { contactId?: string }).contactId ?? "",
        );
      case "draft_notices":
        return DRAFT_NOTICES;
      case "draft_reply":
        return drafting((payload as { pasted?: string }).pasted ?? "");
      case "prepare_draft": {
        const asked = payload as { pasted?: string; includeOriginal?: boolean };
        return (options.preparing ?? (() => anE1Plan()))(
          asked.pasted ?? "",
          asked.includeOriginal ?? false,
        );
      }
      case "generate_draft":
        return (options.generating ?? (() => aTemplateDraft({ source: "user_endpoint" })))(
          (payload as { approval: { preparation_id: string; plan_hash: string } }).approval,
        );
      case "discard_draft":
        return true;
      case "set_user_endpoint":
        configuration = (options.endpointing ?? savedEndpoint)(
          (payload as { url?: string }).url ?? "",
          configuration,
        );
        return configuration;
      case "clear_user_endpoint":
        configuration = withEndpoint(configuration, false);
        return configuration;
      case "preview_soul_import_v1":
      case "preview_telegram": {
        const format = cmd === "preview_telegram" ? "telegram-desktop" : "soul-import-v1";
        const text = (payload as { text?: string }).text ?? "";
        return (options.readingImport ?? ((source: string) => anImportPreview({ source })))(
          format,
          text,
        );
      }
      case "commit_soul_import_v1":
      case "commit_telegram": {
        const format = cmd === "commit_telegram" ? "telegram-desktop" : "soul-import-v1";
        const text = (payload as { text?: string }).text ?? "";
        return (options.importing ?? ((source: string) => anImportReceipt({ source })))(
          format,
          text,
        );
      }
      case "questionnaire":
        return questions;
      case "answer_questionnaire": {
        const answers =
          (payload as { answers?: readonly { question_id: string; given: string }[] }).answers ??
          [];
        if (options.recording !== undefined) {
          // May throw, which is how a refusal arrives; nothing moves then.
          const written = options.recording(answers);
          profile = profileAfter(profileNow(), answers, questions);
          return written;
        }
        const kept = answers.filter((answer) => answer.given.trim() !== "");
        // The core refuses a questionnaire with nothing in it rather than
        // reporting an intake that wrote no rows. The double has to as well,
        // or the wizard's handling of that refusal is never exercised.
        if (kept.length === 0) throw { reason_code: "ROUTINE", explanation: NO_ANSWERS_NOTICE };
        profile = profileAfter(profileNow(), answers, questions);
        return anIntakeReceipt({ answered: kept.length });
      }
      case "profile_screen":
        return profileNow();
      case "correct_axis": {
        const asked = payload as { axisId?: string; position?: string };
        profile = (options.correcting ?? ((_axis: string, position: string) => aProfileScreen({
          axes: aProfileScreen().axes.map((axis, index) =>
            index === 0 ? { ...axis, position, locked_by_user: true } : axis,
          ),
        })))(asked.axisId ?? "", asked.position ?? "");
        return profile;
      }
      case "set_voice": {
        const asked = payload as { field?: string; option?: string };
        profile = (options.voicing ?? ((_field: string, option: string) => aProfileScreen({
          voice: {
            ...aProfileScreen().voice,
            fields: aProfileScreen().voice.fields.map((field) => ({
              ...field,
              value: option,
              locked_by_user: true,
            })),
          },
        })))(asked.field ?? "", asked.option ?? "");
        return profile;
      }
      case "memory_list":
        return (options.memories ?? (() => NO_MEMORIES))();
      case "memory_detail":
        return (options.opening ?? (() => aMemoryDetail()))(
          (payload as { memoryId?: string }).memoryId ?? "",
        );
      case "create_memory":
        return (options.writing ??
          ((memory: NewMemory) => aMemoryDetail({ ...memory })))(
          (payload as { memory: NewMemory }).memory,
        );
      case "update_memory": {
        const asked = payload as { memoryId?: string; change: MemoryChange };
        const changed = (id: string, change: MemoryChange): MemoryDetail => {
          const before = aMemoryDetail({ memory_id: id });
          return {
            ...before,
            memory_type: change.memory_type ?? before.memory_type,
            title: change.title ?? before.title,
            summary: change.summary ?? before.summary,
          };
        };
        return (options.editing ?? changed)(asked.memoryId ?? "", asked.change);
      }
      case "preview_forget":
        return (options.pricing ?? (() => aForgetPreview()))(
          (payload as { memoryId?: string }).memoryId ?? "",
        );
      case "forget_memory": {
        const confirmation = (payload as {
          confirmation: { preview_id: string; memory_id: string };
        }).confirmation;
        if (options.forgetting !== undefined) return options.forgetting(confirmation);
        // The core holds the preview it issued and refuses anything else.
        if (confirmation.preview_id !== PREVIEW_ID) {
          throw {
            reason_code: "PLAN_HASH_MISMATCH",
            explanation: "这次遗忘对不上你刚才看过的那份影响面预览。什么都没有销毁。",
          };
        }
        return aForgetReceipt({ memory_id: confirmation.memory_id });
      }
      case "research_preview":
        return (options.research ?? (() => aResearchPreview()))();
      case "audit_chain":
        return (options.audit ?? (() => EMPTY_CHAIN))();
      case "collect_status":
        collect = (options.rereading ?? ((current: CollectStatus) => current))(collect);
        return collect;
      case "grant_collect_consent":
        collect = (options.granting ?? grantedFrom)(collect);
        // The real handler writes the session's `Config` on the way through,
        // so the next `config_snapshot` says the capability is open. A double
        // that left the snapshot behind would let the overview keep claiming
        // 全部关闭 in a test while no shipped build could.
        configuration = withCollect(configuration, collect.consent_granted);
        return collect;
      case "revoke_collect_consent":
        collect = (options.revoking ?? revokedFrom)(collect);
        configuration = withCollect(configuration, collect.consent_granted);
        return collect;
      default:
        throw `the shell called a command the core does not have: ${cmd}`;
    }
  });

  return {
    calls,
    callsTo: (cmd: string) => calls.filter((call) => call.cmd === cmd),
  };
}

/**
 * Fail the test if anything in the shell tries to reach the network.
 *
 * jsdom has no real sockets, so this is about intent rather than packets: it
 * catches a component that grew a `fetch` on the way to satisfying a design
 * review. Every stub is a `vi.stubGlobal`, so `unstubGlobals` puts the
 * originals back between tests. Returns the list of attempts, which stays
 * empty in a passing run.
 */
export function forbidNetwork(): string[] {
  const attempts: string[] = [];
  const record =
    (what: string) =>
    (...args: unknown[]) => {
      attempts.push(`${what}(${args.map((argument) => String(argument)).join(", ")})`);
      throw new Error(`the shell must not reach the network, but it called ${what}`);
    };

  vi.stubGlobal("fetch", record("fetch"));
  vi.stubGlobal(
    "XMLHttpRequest",
    class {
      open = record("XMLHttpRequest.open");
      send = record("XMLHttpRequest.send");
    },
  );
  vi.stubGlobal(
    "WebSocket",
    class {
      constructor(...args: unknown[]) {
        record("WebSocket")(...args);
      }
    },
  );
  vi.stubGlobal(
    "EventSource",
    class {
      constructor(...args: unknown[]) {
        record("EventSource")(...args);
      }
    },
  );
  vi.stubGlobal("navigator", { ...navigator, sendBeacon: record("navigator.sendBeacon") });

  return attempts;
}
