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
  ImportCounts,
  ImportNamespace,
  ImportOperation,
  ImportOperationView,
  ImportPrepareRequest,
  PreparedImport,
  ImportRecoveryState,
  LocalInsight,
  MemoryContextRequest,
  MemoryContextView,
  TaskContextRequest,
  TaskContextView,
  MemoryDraftPlan,
  MemoryDraftValue,
  InsightContextRequest,
  InsightContextView,
  InsightReference,
  InsightDraftValue,
  InsightDraftPlan,
  MemorySelectionPreview,
  SelectedMemory,
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
  EvidenceSourcesRequest,
  EvidenceSourcesView,
  EvidenceEventRequest,
  EvidenceEventView,
  EvidenceEventSource,
  AssistanceContextView,
  AssistanceGoalFields,
  AssistanceItemDetail,
  AssistanceItemRow,
  AssistancePointer,
  AssistanceWorkMetadata,
  AssistanceWorkStatus,
  AssistanceReminderView,
  AssistanceFeedbackView,
  AssistanceNotificationAttempt,
  AssistanceCategoryPreferences,
  AssistanceCategoryPreference,
  AssistanceNotificationRuntimeStatus,
  NotificationWithdrawal,
  AssistanceCategory,
  AssistanceLocalPreparation,
  AssistanceLocalTemplatePreview,
  AssistanceLocalTemplateRequest,
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
        locked_by_user: false,
        user_band: null,
        machine_band: "moderate",
        interaction_count: 6,
        outgoing_count: 3,
        incoming_count: 3,
        conversation_count: 2,
        active_day_count: 4,
        venue_split_measured: true,
        direct_count: 6,
        group_count: 0,
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
  "遗忘销毁的是这条记忆的内容密钥：正文从此打不开，行会留成一块墓碑，引用过它的推断会被标成失去依据。这一步不可撤销。它不动 Soul 数据目录以外的任何文件，但 Soul 自己的加密库要写：密钥行和密文行被删掉，墓碑、失据标记和一条审计记录被写进去，数据库文件和它的日志都会跟着变。它不是文件整理的执行。这不是把磁盘块擦干净：SSD 上可能还留着旧密文，只是没有密钥再也打不开。";

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

export const IMPORT_NAMESPACE_ID = "0192f000-0000-7000-8000-00000000aa01";
export const IMPORT_OPERATION_ID = "0192f000-0000-7000-8000-00000000bb01";
export const IMPORT_PREVIEW_ID = "0192f000-0000-7000-8000-00000000cc01";

export function anImportCounts(overrides: Partial<ImportCounts> = {}): ImportCounts {
  return { participants: 4, messages: 16, duplicates: 0, contacts_created: 4, contacts_matched: 0,
    events_written: 16, evidence_written: 16, messages_with_injection_markers: 1, ties_rebuilt: 3, ...overrides };
}

export function anImportOperation(overrides: Partial<ImportOperation> = {}): ImportOperation {
  return { operation_id: IMPORT_OPERATION_ID, preview_id: IMPORT_PREVIEW_ID, source: "soul-import-v1",
    namespace_id: IMPORT_NAMESPACE_ID, legacy_choice: "reject", state: "reserved", preview: anImportCounts(),
    receipt: null, error_code: null, audit_state: "not_applicable", created_at: 1_800_000_000, ...overrides };
}

export function anImportOperationView(overrides: Partial<ImportOperationView> = {}): ImportOperationView {
  return { operation: anImportOperation(), namespace_revoked: false, prepared_here: true, ...overrides };
}

export function aPreparedImport(overrides: Partial<PreparedImport> = {}): PreparedImport {
  return { operation: anImportOperation(), preview: anImportPreview(), legacy_events: 0,
    notice: IMPORT_LOCAL_ONLY_NOTICE, ...overrides };
}

export const DRAFT_MEMORY_ID = "0192f000-0000-7000-8000-00000000ee11";
export const DRAFT_CONTEXT_ID = "0192f000-0000-7000-8000-00000000dd11";
export function aSelectedMemory(overrides: Partial<SelectedMemory> = {}): SelectedMemory {
  const id = overrides.memory_id ?? DRAFT_MEMORY_ID;
  return { memory_id: id, memory_type: "preference", title: "合成图书馆偏好", summary: "这次想在合成图书馆学习。", privacy_class: "owner",
    citation: { memory_id: id, title_blob_id: "0192f000-0000-7000-8000-00000000e001", summary_blob_id: "0192f000-0000-7000-8000-00000000e002" }, ...overrides };
}
export function aMemorySelection(overrides: Partial<MemorySelectionPreview> = {}): MemorySelectionPreview {
  return { selected: [aSelectedMemory()], excluded: [], outcome: "context", notice: "本次本地记忆参考；未核验事实时间和语义冲突，不授予研究或端点原文许可。", ...overrides };
}
export function aMemoryContextView(overrides: Partial<MemoryContextView> = {}): MemoryContextView {
  return { context_id: DRAFT_CONTEXT_ID, preview: aMemorySelection(), ...overrides };
}
export function aMemoryDraftPlan(overrides: Partial<MemoryDraftPlan> = {}): MemoryDraftPlan {
  return { prepared: { plan: anE1Plan(), retained_memory_ids: [DRAFT_MEMORY_ID], endpoint_owner_ids: [],
    encoded_bytes: 1024, body_preview: "[本次记忆仅供本地参考，正文未向端点提供]" }, context: aMemorySelection(), ...overrides };
}
export const LOCAL_INSIGHT_ID = "0192f000-0000-7000-8000-000000001511";
export function aLocalInsight(overrides: Partial<LocalInsight> = {}): LocalInsight {
  return { bundle_id: LOCAL_INSIGHT_ID, operation_id: IMPORT_OPERATION_ID, namespace_id: IMPORT_NAMESPACE_ID,
    validity: "active", ignored: false, counts: anImportCounts(), summaries: [aPersonSummary()], notice: "合成本地统计候选，未经模型出网。", ...overrides };
}
export const INSIGHT_CONTEXT_ID = "0192f000-0000-7000-8000-00000000dd21";
export function anInsightReference(overrides: Partial<InsightReference> = {}): InsightReference {
  const summary = overrides.summary ?? aPersonSummary();
  return { bundle_id: LOCAL_INSIGHT_ID, operation_id: IMPORT_OPERATION_ID, namespace_id: IMPORT_NAMESPACE_ID,
    contact_id: summary.contact_id, relationship_id: "0192f000-0000-7000-8000-00000000000a", summary,
    sources: [...new Set(summary.points.flatMap((point) => point.evidence_ids))].map((id) => ({ evidence_id: id, kind: "message", event_ids: ["0192f000-0000-7000-8000-00000000ea21"] })),
    notice: "合成本次统计材料出处，不证明整篇草稿逐句有据。", ...overrides };
}
export function anInsightContext(overrides: Partial<InsightContextView> = {}): InsightContextView {
  return { context_id: INSIGHT_CONTEXT_ID, reference: anInsightReference(), ...overrides };
}
export function anInsightDraftPlan(overrides: Partial<InsightDraftPlan> = {}): InsightDraftPlan {
  return { prepared: { plan: anE1Plan(), encoded_bytes: 1200, body_preview: "[本次统计仅作本地参考]", includes_counts: false, includes_goal: false }, reference: anInsightReference(), goal_preview: null, ...overrides };
}
export const ASSISTANCE_ITEM_ID = "0192f000-0000-7000-8000-00000000a511";
export const ASSISTANCE_REVISION = "0192f000-0000-7000-8000-00000000a512";
export function anAssistanceRow(overrides: Partial<AssistanceItemRow> = {}): AssistanceItemRow {
  return { item_id: ASSISTANCE_ITEM_ID, revision: ASSISTANCE_REVISION,
    source: { kind: "manual_goal", memory_id: DRAFT_MEMORY_ID, observation: null }, validity: "current", handling: "open",
    deadline_at: null, created_at: 1_800_000_000, updated_at: 1_800_000_000, ...overrides };
}
export function anAssistanceDetail(overrides: Partial<AssistanceItemDetail> = {}): AssistanceItemDetail {
  return { item: anAssistanceRow(), goal: aMemoryDetail({ memory_id: DRAFT_MEMORY_ID, title: "合成明确目标", summary: "完成条件由本人确认。" }),
    observation: null, notice: "合成事项详情；完成只表示自报，不代表核实。", ...overrides };
}
export function anAssistanceContext(overrides: Partial<Extract<AssistanceContextView, { kind: "memory" }>> = {}): AssistanceContextView {
  return { item_id: ASSISTANCE_ITEM_ID, revision: ASSISTANCE_REVISION, kind: "memory", memory: aMemoryContextView(), ...overrides };
}
export function anAssistanceWork(overrides: Partial<AssistanceWorkMetadata> = {}): AssistanceWorkMetadata {
  return { item_id: ASSISTANCE_ITEM_ID, item_revision: ASSISTANCE_REVISION, state: "none", pending_revision: null,
    claimed_revision: null, claim_id: null, result_revision: null, failure_code: null, ...overrides };
}
export function anAssistanceWorkStatus(overrides: Partial<AssistanceWorkStatus> = {}): AssistanceWorkStatus {
  return { active_worker: false, active_item_id: null, pending_items: 0, claimed_items: 0, foreground_preparation_held: false,
    coordination_unknown: false, collector_paused_for_import: false, can_begin: true, runtime_state: "running", last_runtime_error: null, ...overrides };
}
export const ASSISTANCE_SCHEDULE_REVISION = "0192f000-0000-7000-8000-00000000a530";
export const ASSISTANCE_FEEDBACK_REVISION = "0192f000-0000-7000-8000-00000000a531";
export const ASSISTANCE_NOTIFICATION_ATTEMPT = "0192f000-0000-7000-8000-00000000a532";
export const ASSISTANCE_WITHDRAWAL_ID = "0192f000-0000-7000-8000-00000000a533";
export function anAssistanceNotificationAttempt(overrides: Partial<AssistanceNotificationAttempt> = {}): AssistanceNotificationAttempt {
  return { attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, item_id: ASSISTANCE_ITEM_ID, item_revision: ASSISTANCE_REVISION,
    schedule_revision: ASSISTANCE_SCHEDULE_REVISION, delivery: "submitted_to_os", shown_at: null, opened_at: null, hidden_at: null,
    timed_out_at: null, tracking_expired_at: null, latest_withdrawal: null, ...overrides };
}
export function anAssistanceReminder(overrides: Partial<AssistanceReminderView> = {}): AssistanceReminderView {
  return { item_id: ASSISTANCE_ITEM_ID, item_revision: ASSISTANCE_REVISION, schedule_revision: null, state: "off", remind_at: null, latest_attempt: null, failure_code: null, ...overrides };
}
export function anAssistanceFeedback(overrides: Partial<AssistanceFeedbackView> = {}): AssistanceFeedbackView {
  return { item_id: ASSISTANCE_ITEM_ID, item_revision: ASSISTANCE_REVISION, feedback_revision: null, value: "none", ...overrides };
}
export function assistanceCategoryDefaults(): AssistanceCategoryPreferences {
  return { preferences: [{ category: "manual_goal", revision: null, muted: false }, { category: "observation", revision: null, muted: false }] };
}
export const ASSISTANCE_LOCAL_PREFERENCE_REVISION = "0192f000-0000-7000-8000-00000000a534";
export const ASSISTANCE_LOCAL_TEMPLATE_ID = "0192f000-0000-7000-8000-00000000a535";
export function anAssistanceLocalPreparation(overrides: Partial<AssistanceLocalPreparation> = {}): AssistanceLocalPreparation {
  return { item_id: ASSISTANCE_ITEM_ID, item_revision: ASSISTANCE_REVISION, preference_revision: null, enabled: false, state: "off", template_id: null, failure_code: null, ...overrides };
}
export function anAssistanceLocalTemplatePreview(overrides: Partial<AssistanceLocalTemplatePreview> = {}): AssistanceLocalTemplatePreview {
  return { item_id: ASSISTANCE_ITEM_ID, item_revision: ASSISTANCE_REVISION, preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION,
    template_id: ASSISTANCE_LOCAL_TEMPLATE_ID, kind: "opening_v1", prepared_at: 1792166400000, body: "你好。\n（这里写你要说的内容）", ...overrides };
}
export const EVIDENCE_EVENT_ID = "0192f000-0000-7000-8000-00000000ea31";
export function anEvidenceEventSource(overrides: Partial<EvidenceEventSource> = {}): EvidenceEventSource {
  return { event_id: EVIDENCE_EVENT_ID, namespace_id: IMPORT_NAMESPACE_ID, source: "import.soul_import_v1", occurred_at: "2026-10-15T12:00:00Z",
    direction: "incoming", venue: "direct", body_chars: 10, availability: "available", notice: "合成可明确查看的本地来源。", ...overrides };
}
export function anEvidenceSources(overrides: Partial<EvidenceSourcesView> = {}): EvidenceSourcesView {
  const graph = aPeopleGraph(); const tie = graph.ties[0]!;
  return { contact_id: graph.people[1]!.contact_id, relationship_id: tie.relationship_id, evidence_id: tie.evidence[0]!.evidence_id,
    kind: "message", events: [anEvidenceEventSource()], notice: "这里只列本条来源元数据，不打开正文。", ...overrides };
}
export function anEvidenceEvent(overrides: Partial<EvidenceEventView> = {}): EvidenceEventView {
  const anchors = anEvidenceSources(); const event = anEvidenceEventSource();
  return { contact_id: anchors.contact_id, relationship_id: anchors.relationship_id, evidence_id: anchors.evidence_id,
    event_id: event.event_id, namespace_id: event.namespace_id!, source: event.source, occurred_at: event.occurred_at,
    direction: event.direction, venue: event.venue, body: "合成单条本地敏感内容", notice: "仅当前本地来源，不认证旧统计。", ...overrides };
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
    ignored: [],
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


/**
 * Whether an answer would land on an axis the user has already corrected.
 *
 * `soul-profile`'s intake records such an answer and leaves the axis where the
 * user put it (D46), so the double has to know which of its two axis questions
 * points at a locked row.
 */
function answerHitsALockedAxis(
  profile: ProfileScreen,
  questionId: string,
): boolean {
  const axisId = AXIS_OF_QUESTION[questionId];
  return profile.axes.some((axis) => axis.axis_id === axisId && axis.locked_by_user);
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
    assistance_items_invalidated: 0,
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
    assistance_items_invalidated: 0,
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
    deny_rows_excluded: 0,
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

/** `soulcore::commands::shell::WIZARD_NOT_ACKNOWLEDGED_NOTICE`. */
export const WIZARD_NOT_ACKNOWLEDGED_NOTICE =
  "你还没勾上「我读过上面这几行」，向导就没有可以结束的东西。什么都没有写下，勾上之后再点一次就行。";

/** What the core says when a correction names something that is not a band. */
export const NOT_A_BAND_NOTICE = "一条关系只有弱、中等、强三档。";

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
  readonly evidenceSources?: (request: EvidenceSourcesRequest) => EvidenceSourcesView | Promise<EvidenceSourcesView>;
  readonly readingEvidenceEvent?: (request: EvidenceEventRequest) => EvidenceEventView | Promise<EvidenceEventView>;
  /** Able to throw, because an invalid band arrives as a refusal value. */
  readonly correctingTie?: (relationshipId: string, band: string) => PeopleGraph;
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
  readonly generating?: (approval: { preparation_id: string; plan_hash: string }) => Draft | Promise<Draft>;
  readonly discardingDraft?: () => boolean | Promise<boolean>;
  /** Able to throw: a file that does not parse arrives as a refusal value. */
  readonly readingImport?: (format: string, text: string) => ImportPreview;
  readonly importing?: (format: string, text: string) => ImportReceipt;
  readonly importNamespaces?: readonly ImportNamespace[];
  readonly listingImportNamespaces?: () => readonly ImportNamespace[] | Promise<readonly ImportNamespace[]>;
  readonly importOperations?: readonly ImportOperationView[];
  readonly preparingImport?: (request: ImportPrepareRequest) => PreparedImport | Promise<PreparedImport>;
  readonly confirmingImport?: (operationId: string, previewId: string) => ImportOperationView | Promise<ImportOperationView>;
  readonly discardingImport?: (operationId: string) => ImportOperationView | Promise<ImportOperationView>;
  readonly lookingUpImport?: (operationId: string) => ImportOperationView | Promise<ImportOperationView>;
  readonly recentImports?: () => readonly ImportOperationView[] | Promise<readonly ImportOperationView[]>;
  readonly recoveringImport?: () => ImportRecoveryState | Promise<ImportRecoveryState>;
  readonly preparingMemory?: (request: MemoryContextRequest) => MemoryContextView | Promise<MemoryContextView>;
  readonly preparingTask?: (request: TaskContextRequest) => TaskContextView | Promise<TaskContextView>;
  readonly draftingMemory?: (pasted: string, contextId: string) => MemoryDraftValue | Promise<MemoryDraftValue>;
  readonly preparingDraftMemory?: (pasted: string, contextId: string, ownerIds: readonly string[], includeOriginal: boolean) => MemoryDraftPlan | Promise<MemoryDraftPlan>;
  readonly preparingInsight?: (request: InsightContextRequest) => InsightContextView | Promise<InsightContextView>;
  readonly draftingInsight?: (pasted: string, contextId: string) => InsightDraftValue | Promise<InsightDraftValue>;
  readonly preparingDraftInsight?: (pasted: string, contextId: string, includeCounts: boolean, includeOriginal: boolean, includeGoal: boolean) => InsightDraftPlan | Promise<InsightDraftPlan>;
  readonly insights?: readonly LocalInsight[];
  readonly readingInsights?: () => readonly LocalInsight[] | Promise<readonly LocalInsight[]>;
  readonly ignoringInsight?: (bundleId: string) => LocalInsight | Promise<LocalInsight>;
  readonly recomputingInsight?: (bundleId: string) => LocalInsight | Promise<LocalInsight>;
  readonly assistanceRows?: readonly AssistanceItemRow[];
  readonly readingAssistanceItems?: () => { readonly items: readonly AssistanceItemRow[] } | Promise<{ readonly items: readonly AssistanceItemRow[] }>;
  readonly readingAssistanceItem?: (itemId: string) => AssistanceItemDetail | Promise<AssistanceItemDetail>;
  readonly changingAssistance?: (command: string, request: Record<string, unknown>) => AssistanceItemDetail | Promise<AssistanceItemDetail>;
  readonly preparingAssistance?: (request: AssistancePointer) => AssistanceContextView | Promise<AssistanceContextView>;
  readonly assistanceWorkStatus?: () => AssistanceWorkStatus | Promise<AssistanceWorkStatus>;
  readonly readingAssistanceWork?: (itemId: string) => AssistanceWorkMetadata | Promise<AssistanceWorkMetadata>;
  readonly retryingAssistanceWork?: (request: AssistancePointer) => AssistanceWorkMetadata | Promise<AssistanceWorkMetadata>;
  readonly readingAssistanceReminder?: (itemId: string) => AssistanceReminderView | Promise<AssistanceReminderView>;
  readonly settingAssistanceReminder?: (request: AssistancePointer & { readonly expected_schedule_revision: string | null; readonly remind_at: string | null }) => AssistanceReminderView | Promise<AssistanceReminderView>;
  readonly readingAssistanceFeedback?: (itemId: string) => AssistanceFeedbackView | Promise<AssistanceFeedbackView>;
  readonly settingAssistanceFeedback?: (request: AssistancePointer & { readonly expected_feedback_revision: string | null; readonly value: "helpful" | "not_relevant" | "clear" }) => AssistanceFeedbackView | Promise<AssistanceFeedbackView>;
  readonly readingAssistanceCategories?: () => AssistanceCategoryPreferences | Promise<AssistanceCategoryPreferences>;
  readonly settingAssistanceCategory?: (request: { readonly category: AssistanceCategory; readonly expected_revision: string | null; readonly muted: boolean }) => AssistanceCategoryPreference | Promise<AssistanceCategoryPreference>;
  readonly withdrawingAssistanceNotification?: (request: { readonly attempt_id: string; readonly expected_withdrawal_id: string | null }) => NotificationWithdrawal | Promise<NotificationWithdrawal>;
  readonly notificationRuntimeStatus?: () => AssistanceNotificationRuntimeStatus | Promise<AssistanceNotificationRuntimeStatus>;
  readonly readingAssistanceLocalPreparation?: (itemId: string) => AssistanceLocalPreparation | Promise<AssistanceLocalPreparation>;
  readonly settingAssistanceLocalPreparation?: (request: AssistancePointer & { readonly expected_preference_revision: string | null; readonly enabled: boolean }) => AssistanceLocalPreparation | Promise<AssistanceLocalPreparation>;
  readonly previewingAssistanceLocalTemplate?: (request: AssistanceLocalTemplateRequest) => AssistanceLocalTemplatePreview | Promise<AssistanceLocalTemplatePreview>;
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
  let assistanceWriteSequence = 0;
  const nextAssistanceRevision = (): string => `0192f000-0000-7000-8000-${(++assistanceWriteSequence).toString(16).padStart(12, "0")}`;
  /** The three pieces of state the double keeps, because the core keeps them
   *  too: a consent ledger, an endpoint that lives for one run, and a profile
   *  that a questionnaire or a correction writes to and the next read sees. */
  let collect = options.collect ?? COLLECT_OFF;
  let configuration = snapshot;
  let profile: ProfileScreen | null = null;
  let graphHeld: PeopleGraph = options.graph ?? EMPTY_GRAPH;
  const spentForgetPreviews = new Set<string>();
  const importOperations = new Map<string, ImportOperationView>(
    (options.importOperations ?? []).map((view) => [view.operation.operation_id, view]),
  );
  let importSequence = importOperations.size;
  let heldMemory: MemoryContextView | null = null;
  let heldInsight: InsightContextView | null = null;
  let heldGoal: MemorySelectionPreview | null = null;
  let insights = [...(options.insights ?? [])];
  let insightSequence = 0;

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
          throw {
            reason_code: "ROUTINE",
            explanation: WIZARD_NOT_ACKNOWLEDGED_NOTICE,
          };
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
        return (options.graphing ?? (() => graphHeld))();
      case "evidence_sources": {
        const request = (payload as { request: EvidenceSourcesRequest }).request;
        return options.evidenceSources?.(request) ?? anEvidenceSources(request);
      }
      case "read_evidence_event": {
        const request = (payload as { request: EvidenceEventRequest }).request;
        return options.readingEvidenceEvent?.(request) ?? anEvidenceEvent(request);
      }
      case "correct_tie": {
        const asked = payload as { relationshipId?: string; band?: string };
        if (options.correctingTie !== undefined) {
          graphHeld = options.correctingTie(asked.relationshipId ?? "", asked.band ?? "");
          return graphHeld;
        }
        if (!["weak", "moderate", "strong"].includes(asked.band ?? "")) {
          throw { reason_code: "ROUTINE", explanation: NOT_A_BAND_NOTICE };
        }
        const current = graphHeld;
        const evidenceId = `0192f000-0000-7000-8000-${(0xc0e000 + current.ties.reduce((n, t) => n + t.evidence.length, 0)).toString(16).padStart(12, "0")}`;
        const next: PeopleGraph = {
          ...current,
          ties: current.ties.map((tie) =>
            tie.relationship_id === asked.relationshipId
              ? {
                  ...tie,
                  band: asked.band ?? tie.band,
                  locked_by_user: true,
                  user_band: asked.band ?? tie.band,
                  machine_band: tie.machine_band ?? tie.band,
                  evidence: [
                    ...tie.evidence,
                    { evidence_id: evidenceId, kind: "user_correction", method: "user_stated", strength: "strong" },
                  ],
                }
              : tie,
          ),
        };
        graphHeld = next;
        return next;
      }
      case "release_tie": {
        const asked = payload as { relationshipId?: string };
        const current = graphHeld;
        const next: PeopleGraph = {
          ...current,
          ties: current.ties.map((tie) =>
            tie.relationship_id === asked.relationshipId
              ? {
                  ...tie,
                  band: tie.machine_band ?? tie.band,
                  locked_by_user: false,
                  user_band: null,
                }
              : tie,
          ),
        };
        graphHeld = next;
        return next;
      }
      case "person_summary":
        return (options.summarizing ?? (() => aPersonSummary()))(
          (payload as { contactId?: string }).contactId ?? "",
        );
      case "draft_notices":
        return DRAFT_NOTICES;
      case "assistance_items":
        return options.readingAssistanceItems?.() ?? { items: options.assistanceRows ?? [] };
      case "assistance_work_status":
        return options.assistanceWorkStatus?.() ?? anAssistanceWorkStatus();
      case "assistance_item_work": {
        const itemId = (payload as { itemId: string }).itemId;
        return options.readingAssistanceWork?.(itemId) ?? anAssistanceWork({ item_id: itemId });
      }
      case "retry_assistance_work": {
        const request = (payload as { request: AssistancePointer }).request;
        return options.retryingAssistanceWork?.(request) ?? anAssistanceWork({ item_id: request.item_id, item_revision: request.expected_revision, state: "pending", pending_revision: request.expected_revision });
      }
      case "assistance_reminder": {
        const itemId = (payload as { itemId: string }).itemId;
        return options.readingAssistanceReminder?.(itemId) ?? anAssistanceReminder({ item_id: itemId });
      }
      case "set_assistance_reminder": {
        const request = (payload as { request: Parameters<NonNullable<FakeCoreOptions["settingAssistanceReminder"]>>[0] }).request;
        return options.settingAssistanceReminder?.(request) ?? anAssistanceReminder({ item_id: request.item_id, item_revision: request.expected_revision,
          schedule_revision: nextAssistanceRevision(), state: request.remind_at === null ? "off" : "scheduled", remind_at: request.remind_at });
      }
      case "assistance_feedback": {
        const itemId = (payload as { itemId: string }).itemId; return options.readingAssistanceFeedback?.(itemId) ?? anAssistanceFeedback({ item_id: itemId });
      }
      case "set_assistance_feedback": {
        const request = (payload as { request: Parameters<NonNullable<FakeCoreOptions["settingAssistanceFeedback"]>>[0] }).request;
        return options.settingAssistanceFeedback?.(request) ?? anAssistanceFeedback({ item_id: request.item_id, item_revision: request.expected_revision,
          feedback_revision: nextAssistanceRevision(), value: request.value === "clear" ? "none" : request.value });
      }
      case "assistance_category_preferences": return options.readingAssistanceCategories?.() ?? assistanceCategoryDefaults();
      case "set_assistance_category_preference": {
        const request = (payload as { request: Parameters<NonNullable<FakeCoreOptions["settingAssistanceCategory"]>>[0] }).request;
        return options.settingAssistanceCategory?.(request) ?? { category: request.category, revision: nextAssistanceRevision(), muted: request.muted };
      }
      case "retry_assistance_notification_withdrawal": {
        const request = (payload as { request: { attempt_id: string; expected_withdrawal_id: string | null } }).request;
        return options.withdrawingAssistanceNotification?.(request) ?? { attempt_id: request.attempt_id, withdrawal_id: nextAssistanceRevision(), state: "pending" };
      }
      case "assistance_notification_runtime_status": return options.notificationRuntimeStatus?.() ?? { state: "running", last_error: null };
      case "assistance_local_preparation": {
        const itemId = (payload as { itemId: string }).itemId;
        return options.readingAssistanceLocalPreparation?.(itemId) ?? anAssistanceLocalPreparation({ item_id: itemId });
      }
      case "set_assistance_local_preparation": {
        const request = (payload as { request: Parameters<NonNullable<FakeCoreOptions["settingAssistanceLocalPreparation"]>>[0] }).request;
        return options.settingAssistanceLocalPreparation?.(request) ?? anAssistanceLocalPreparation({ item_id: request.item_id, item_revision: request.expected_revision,
          preference_revision: nextAssistanceRevision(), enabled: request.enabled, state: request.enabled ? "pending" : "off" });
      }
      case "preview_assistance_local_template": {
        const request = (payload as { request: AssistanceLocalTemplateRequest }).request;
        return options.previewingAssistanceLocalTemplate?.(request) ?? anAssistanceLocalTemplatePreview({ item_id: request.item_id, item_revision: request.expected_revision,
          preference_revision: request.expected_preference_revision, template_id: request.template_id });
      }
      case "assistance_item": {
        const id = (payload as { itemId: string }).itemId;
        return options.readingAssistanceItem?.(id) ?? anAssistanceDetail({ item: anAssistanceRow({ item_id: id }) });
      }
      case "create_assistance_goal":
      case "edit_assistance_goal":
      case "link_assistance_observation":
      case "refresh_assistance_observation":
      case "bind_assistance_goal_source":
      case "act_assistance_item": {
        const request = (payload as { request: Record<string, unknown> }).request;
        if (options.changingAssistance !== undefined) return options.changingAssistance(cmd, request);
        const fields = request as unknown as AssistanceGoalFields;
        return anAssistanceDetail({ item: anAssistanceRow({ item_id: typeof request.item_id === "string" ? request.item_id : ASSISTANCE_ITEM_ID }),
          goal: aMemoryDetail({ memory_id: DRAFT_MEMORY_ID, title: fields.title ?? "合成明确目标", summary: fields.summary ?? "完成条件由本人确认。" }) });
      }
      case "prepare_assistance_context": {
        const request = (payload as { request: AssistancePointer }).request;
        return Promise.resolve(options.preparingAssistance?.(request) ?? anAssistanceContext({ item_id: request.item_id, revision: request.expected_revision })).then((value) => {
          heldMemory = value.kind === "memory" ? value.memory : null; heldInsight = value.kind === "insight" ? value.insight : null;
          heldGoal = value.kind === "insight" ? value.goal_preview : null; return value;
        });
      }
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
        return Promise.resolve(options.discardingDraft?.() ?? true).then((value) => { heldMemory = null; heldInsight = null; heldGoal = null; return value; });
      case "prepare_insight_context": {
        const request = (payload as { request: InsightContextRequest }).request;
        heldGoal = null;
        heldMemory = null;
        return (async () => {
          heldInsight = await (options.preparingInsight?.(request) ?? anInsightContext({ reference: anInsightReference({ bundle_id: request.bundle_id, contact_id: request.contact_id,
            summary: aPersonSummary({ contact_id: request.contact_id }) }) }));
          return heldInsight;
        })();
      }
      case "local_draft_with_insight": {
        const asked = payload as { pasted: string; contextId: string };
        if (options.draftingInsight !== undefined) return options.draftingInsight(asked.pasted, asked.contextId);
        if (heldInsight?.context_id !== asked.contextId) throw { reason_code: "INSIGHT_CONTEXT_CHANGED", explanation: "没有这次统计材料用途。" };
        const reference = heldInsight.reference; heldInsight = null;
        return { draft: drafting(asked.pasted), reference } satisfies InsightDraftValue;
      }
      case "prepare_draft_with_insight": {
        const asked = payload as { pasted: string; contextId: string; includeCounts: boolean; includeOriginal: boolean; includeGoal: boolean };
        if (options.preparingDraftInsight !== undefined) return options.preparingDraftInsight(asked.pasted, asked.contextId, asked.includeCounts, asked.includeOriginal, asked.includeGoal);
        if (heldInsight?.context_id !== asked.contextId) throw { reason_code: "INSIGHT_CONTEXT_CHANGED", explanation: "没有这次统计材料用途。" };
        return anInsightDraftPlan({ reference: heldInsight.reference, goal_preview: asked.includeGoal ? heldGoal : null,
          prepared: { plan: anE1Plan({ carries_exempted_original: asked.includeOriginal }), encoded_bytes: 1200,
          body_preview: asked.includeCounts ? "[合成匿名 A2 派生计数]" : "[本次统计仅作本地参考]", includes_counts: asked.includeCounts, includes_goal: asked.includeGoal } });
      }
      case "prepare_task_context": {
        const request = (payload as { request: TaskContextRequest }).request;
        heldInsight = null; heldMemory = null;
        if (options.preparingTask === undefined) throw new Error("Synthetic task response must be explicitly supplied.");
        return (async () => { const value = await options.preparingTask!(request); heldMemory = value.memory; return value; })();
      }
      case "prepare_memory_context": {
        const request = (payload as { request: MemoryContextRequest }).request;
        heldInsight = null;
        return (async () => { heldMemory = await (options.preparingMemory?.(request) ?? aMemoryContextView()); return heldMemory; })();
      }
      case "draft_reply_with_memory": {
        const asked = payload as { pasted: string; contextId: string };
        if (options.draftingMemory !== undefined) return options.draftingMemory(asked.pasted, asked.contextId);
        if (heldMemory?.context_id !== asked.contextId) throw { reason_code: "PLAN_HASH_MISMATCH", explanation: "没有这次记忆参考许可。" };
        return { draft: drafting(asked.pasted), context: heldMemory.preview } satisfies MemoryDraftValue;
      }
      case "prepare_draft_with_memory": {
        const asked = payload as { pasted: string; contextId: string; ownerIds: readonly string[]; includeOriginal: boolean };
        if (options.preparingDraftMemory !== undefined) return options.preparingDraftMemory(asked.pasted, asked.contextId, asked.ownerIds, asked.includeOriginal);
        if (heldMemory?.context_id !== asked.contextId || asked.ownerIds.some((id) => !heldMemory?.preview.selected.some((entry) => entry.memory_id === id && entry.privacy_class === "owner"))) {
          throw { reason_code: "PLAN_HASH_MISMATCH", explanation: "记忆原文选择不在当前本人记录范围。" };
        }
        return aMemoryDraftPlan({ context: heldMemory.preview, prepared: {
          plan: anE1Plan({ carries_exempted_original: asked.includeOriginal }),
          retained_memory_ids: heldMemory.preview.selected.map((entry) => entry.memory_id), endpoint_owner_ids: asked.ownerIds,
          encoded_bytes: 1024, body_preview: asked.ownerIds.length === 0 ? "[本次记忆仅供本地参考]" : heldMemory.preview.selected.filter((entry) => asked.ownerIds.includes(entry.memory_id)).map((entry) => entry.summary).join("\n"),
        } });
      }
      case "local_insights":
        return options.readingInsights?.() ?? insights;
      case "ignore_local_insight": {
        const id = (payload as { bundleId: string }).bundleId;
        return (async () => {
          const held = insights.find((row) => row.bundle_id === id);
          if (held === undefined && options.ignoringInsight === undefined) throw { reason_code: "ROUTINE", explanation: "没有这个本地候选。" };
          const result = await (options.ignoringInsight?.(id) ?? { ...held!, ignored: true });
          insights = insights.map((row) => row.bundle_id === id ? result : row); return result;
        })();
      }
      case "recompute_local_insight": {
        const id = (payload as { bundleId: string }).bundleId;
        return (async () => {
          const held = insights.find((row) => row.bundle_id === id);
          if (held === undefined && options.recomputingInsight === undefined) throw { reason_code: "ROUTINE", explanation: "没有这个本地候选。" };
          insightSequence += 1;
          const result = await (options.recomputingInsight?.(id) ?? { ...held!, bundle_id: `0192f000-0000-7000-8000-${(0x1600 + insightSequence).toString(16).padStart(12, "0")}`, validity: "active" as const, ignored: false });
          insights = [...insights.map((row) => row.bundle_id === id ? { ...row, validity: "invalidated" as const } : row), result]; return result;
        })();
      }
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
      case "list_import_namespaces":
        return options.listingImportNamespaces?.() ?? options.importNamespaces ?? [];
      case "recent_import_operations":
        return options.recentImports?.() ?? [...importOperations.values()];
      case "import_recovery_state": {
        const operations = [...importOperations.values()];
        const reserved = operations.filter((view) => view.operation.state === "reserved");
        const running = operations.filter((view) => view.operation.state === "running");
        const held = reserved.find((view) => view.prepared_here)?.operation.operation_id ?? null;
        return options.recoveringImport?.() ?? { active_worker: running.length > 0,
          reserved_operations: reserved.length, running_operations: running.length, held_operation_id: held,
          can_start_new_intent: running.length === 0 && held === null };
      }
      case "import_operation": {
        const id = (payload as { operationId: string }).operationId;
        const found = options.lookingUpImport?.(id) ?? importOperations.get(id);
        if (found === undefined) throw { reason_code: "IMPORT_OPERATION_NOT_FOUND", explanation: "未找到该操作；不代表此前没有发生。" };
        return found;
      }
      case "prepare_import": {
        const request = (payload as { request: ImportPrepareRequest }).request;
        return (async () => {
          if ([...importOperations.values()].some((view) => view.prepared_here && view.operation.state === "reserved")) {
            throw { reason_code: "IMPORT_PREPARATION_HELD", explanation: "先精确丢弃当前旧准备，再建立新意图。" };
          }
          importSequence += 1;
          const preview = (options.readingImport ?? ((source: string) => anImportPreview({ source })))(request.source, request.text);
          const prepared = await (options.preparingImport?.(request) ?? aPreparedImport({
            preview,
            legacy_events: request.legacy_choice === "forward_only" ? 7 : 0,
            operation: anImportOperation({
              operation_id: `0192f000-0000-7000-8000-${(0xbb00 + importSequence).toString(16).padStart(12, "0")}`,
              preview_id: `0192f000-0000-7000-8000-${(0xcc00 + importSequence).toString(16).padStart(12, "0")}`,
              source: request.source, namespace_id: request.namespace_id ?? IMPORT_NAMESPACE_ID,
              legacy_choice: request.legacy_choice,
              preview: anImportCounts({ participants: preview.participants, messages: preview.messages,
                events_written: preview.messages, messages_with_injection_markers: preview.messages_with_injection_markers }),
            }),
          }));
          importOperations.set(prepared.operation.operation_id, anImportOperationView({ operation: prepared.operation }));
          return prepared;
        })();
      }
      case "confirm_import": {
        const asked = payload as { operationId: string; previewId: string };
        return (async () => {
          const held = importOperations.get(asked.operationId);
          if (held === undefined || held.operation.preview_id !== asked.previewId || held.operation.state !== "reserved") {
            throw { reason_code: "IMPORT_PREVIEW_MISMATCH", explanation: "这不是当前尚未消费的准备。" };
          }
          importOperations.set(asked.operationId, { ...held, operation: { ...held.operation, state: "running" } });
          const view = await (options.confirmingImport?.(asked.operationId, asked.previewId)
            ?? anImportOperationView({ operation: { ...held.operation, state: "committed", receipt: held.operation.preview,
              audit_state: "recorded" }, prepared_here: false }));
          importOperations.set(asked.operationId, view);
          return view;
        })();
      }
      case "discard_import": {
        const id = (payload as { operationId: string }).operationId;
        return (async () => {
          const held = importOperations.get(id);
          if (held === undefined || held.operation.state !== "reserved") throw { reason_code: "IMPORT_PREVIEW_MISMATCH", explanation: "没有可丢弃的准备。" };
          const view = await (options.discardingImport?.(id) ?? anImportOperationView({ operation: { ...held.operation, state: "discarded" }, prepared_here: false }));
          importOperations.set(id, view); return view;
        })();
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
          // A receipt that reports an answer as ignored is a receipt from a
          // core that did not apply it, so the double does not either.
          const refused = new Set(written.ignored.map((one) => one.question_id));
          profile = profileAfter(
            profileNow(),
            answers.filter((answer) => !refused.has(answer.question_id)),
            questions,
          );
          return written;
        }
        const kept = answers.filter((answer) => answer.given.trim() !== "");
        // The core refuses a questionnaire with nothing in it rather than
        // reporting an intake that wrote no rows. The double has to as well,
        // or the wizard's handling of that refusal is never exercised.
        if (kept.length === 0) throw { reason_code: "ROUTINE", explanation: NO_ANSWERS_NOTICE };
        // An answer on a corrected axis is written and not applied, and comes
        // back named in `ignored`; `answered` counts what moved (D46).
        const ignored = kept.filter((answer) =>
          answerHitsALockedAxis(profileNow(), answer.question_id),
        );
        const applied = kept.filter((answer) => !ignored.includes(answer));
        profile = profileAfter(profileNow(), applied, questions);
        return anIntakeReceipt({
          answered: applied.length,
          ignored: ignored.map((answer) => ({
            question_id: answer.question_id,
            reason: "axis_locked_by_user",
          })),
        });
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
        if (confirmation.preview_id !== PREVIEW_ID || spentForgetPreviews.has(confirmation.preview_id)) {
          throw {
            reason_code: "PLAN_HASH_MISMATCH",
            explanation: "这次遗忘对不上你刚才看过的那份影响面预览。什么都没有销毁。",
          };
        }
        spentForgetPreviews.add(confirmation.preview_id);
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
