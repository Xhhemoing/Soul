/**
 * The one place in the shell that talks to soulcore.
 *
 * Everything under `src/` that is not this file renders values; this file is
 * the only module allowed to import `@tauri-apps/api`, and `eslint.config.js`
 * enforces that. The rule is not tidiness: WP09's brief is that the WebView
 * holds no business logic, and the cheapest way to keep that true is to make
 * "call the core" a thing that happens in exactly one module, where every call
 * is visible in one screenful.
 *
 * The types below mirror `crates/soulcore/src/commands/`. They are snake_case
 * because that is what crosses the IPC; renaming them here would mean
 * transforming values, and a transform is a place for a bug to live.
 */

import { invoke } from "@tauri-apps/api/core";
import type { ForgetAudit, ForgetCleanup } from "./forgetOutcome";

/** The only state the cloud switch has in v0.1. There is no `enabled`. */
export type CloudState = "not_yet_available";

export interface CloudNotice {
  readonly state: CloudState;
  /** Always the same words: 尚未启用. */
  readonly label: string;
  readonly enabled: boolean;
  /** Always false. Pressing the switch puts nothing on the wire. */
  readonly performs_network_request: boolean;
  readonly explanation: string;
}

export interface ConfigSnapshot {
  readonly collect_enabled: boolean;
  readonly cloud: CloudNotice;
  /** Whether the user has entered their own endpoint. Never the URL itself. */
  readonly llm_endpoint_configured: boolean;
  /** What the core says about that address: this run only, contacted by
   *  nothing until the user asks for a generation — an approved draft, or a
   *  person's summary on the graph. */
  readonly llm_endpoint_notice: string;
  readonly authorized_root_count: number;
  readonly fully_closed: boolean;
  readonly open_capabilities: readonly string[];
}

/**
 * What this session is, as opposed to what it is allowed to do.
 *
 * `wizard_completed` is read off the file beside the store, which is what
 * makes finishing the wizard survive a restart. `store_notice` is the core's
 * own sentence about the database — including, on a build whose key material
 * has no platform protection, the sentence that says so.
 */
export interface SessionStatus {
  readonly wizard_completed: boolean;
  readonly store_opened: boolean;
  readonly key_protection: string;
  readonly store_notice: string;
  readonly config_problem: string | null;
}

export interface WizardAnswers {
  readonly acknowledged_defaults_are_off: boolean;
}

/** A refused command: a code and a sentence, never an endpoint's own words. */
export interface Refusal {
  readonly reason_code: string;
  readonly explanation: string;
}

/** One directory the user has authorized for read-only scanning. */
export interface AuthorizedRoot {
  readonly path: string;
}

/** One that was authorized and can no longer be found. */
export interface UnavailableRoot {
  readonly path: string;
  readonly explanation: string;
}

export interface FilesView {
  readonly read_only_notice: string;
  /** Always false. There is no command that carries a plan out. */
  readonly executable_in_this_version: false;
  readonly roots: readonly AuthorizedRoot[];
  readonly unavailable_roots: readonly UnavailableRoot[];
}

export interface ProposedMove {
  readonly from: string;
  readonly to: string;
  readonly kind: string;
  readonly kind_label: string;
  readonly size_bytes: number;
}

export interface LeftAlone {
  readonly path: string;
  readonly reason: string;
  readonly explanation: string;
}

/**
 * One plan, mirroring `soulcore::commands::fileplan::PlanPreview`.
 *
 * `executable_in_this_version` is typed as the literal `false` so a component
 * that tried to branch towards an "execute" path would not compile. v0.1.1
 * owns the write half; v0.1 owns this screen.
 */
export interface PlanPreview {
  readonly root: string;
  readonly plan_hash: string;
  readonly directory_snapshot: string;
  readonly disk_unchanged: boolean;
  readonly executable_in_this_version: false;
  readonly read_only_notice: string;
  readonly scanned_entries: number;
  readonly skipped_entries: number;
  readonly truncated: boolean;
  readonly moves: readonly ProposedMove[];
  readonly left_alone: readonly LeftAlone[];
}

export interface EvidenceRow {
  readonly evidence_id: string;
  readonly kind: string;
  readonly method: string;
  readonly strength: string;
}

/** A person. There is no field on this that holds a name. */
export interface PersonNode {
  readonly contact_id: string;
  readonly is_you: boolean;
  /** Leading characters of an identifier digest, to tell two people apart. */
  readonly identifier_hint: string;
  readonly interaction_count: number;
  readonly last_contact_utc: string | null;
  readonly tie_count: number;
  readonly forgotten: boolean;
}

export interface TieEdge {
  readonly relationship_id: string;
  readonly from_contact_id: string;
  readonly to_contact_id: string;
  readonly types: readonly string[];
  readonly band: string;
  readonly locked_by_user: boolean;
  readonly user_band: string | null;
  readonly machine_band: string | null;
  readonly interaction_count: number;
  readonly outgoing_count: number;
  readonly incoming_count: number;
  readonly conversation_count: number;
  readonly active_day_count: number;
  readonly venue_split_measured: boolean;
  readonly direct_count: number;
  readonly group_count: number;
  readonly first_contact_utc: string;
  readonly last_contact_utc: string;
  readonly local_only: boolean;
  readonly evidence: readonly EvidenceRow[];
}

export interface PeopleGraph {
  readonly self_contact_id: string | null;
  readonly people: readonly PersonNode[];
  readonly ties: readonly TieEdge[];
  /** 工作假设，非临床结论. */
  readonly notice: string;
  readonly third_party_data_is_local_only: boolean;
}

export interface SummaryPoint {
  readonly statement: string;
  /** Never empty: the core refuses to build a point without evidence. */
  readonly evidence_ids: readonly string[];
  readonly band: string;
}

export interface PersonSummary {
  readonly contact_id: string;
  readonly source: string;
  readonly text: string;
  readonly points: readonly SummaryPoint[];
  readonly notice: string;
  /** Always false, and there is no code path in the core that sets it. */
  readonly clinical_claim: false;
}

/** Which path wrote the text. `tone_template` is the one with no key. */
export type DraftSource = "tone_template" | "user_endpoint";

/** Why an endpoint draft came back from the local template instead. */
export type Degradation = "reply_unreadable" | "reply_empty" | "reply_clinical";

/**
 * One draft, mirroring `crates/soul-draft/src/draft.rs`.
 *
 * There is no field here that names a person or expresses an action, and that
 * is the requirement rather than an accident: `soul-draft`'s
 * `tests/never_sends.rs` pins the list. `delivery` is typed as the literal
 * `false` so that a component which tried to branch on it would not compile.
 */
export interface Draft {
  /** The text the user may copy. Everything else says where it came from. */
  readonly text: string;
  readonly source: DraftSource;
  readonly delivery: false;
  readonly third_party_turns: number;
  readonly placeheld_turns: number;
  /** True only for a request the user confirmed twice for. */
  readonly carries_exempted_original: boolean;
  readonly degraded: Degradation | null;
  /**
   * What the pasted material tried to do. Shown, counted, and never acted on
   * — the core does not branch on it either, which is AC-25.
   */
  readonly injection_signals: readonly string[];
  readonly not_sent_notice: string;
  readonly source_notice: string;
}

/**
 * What the user reads before deciding whether anything may leave.
 *
 * Counts, a model name and two sentences: there is no field here that could
 * hold the third party's prose, because what is being approved is the *shape*
 * of a request. Both identifiers have to be echoed back on approval, and
 * `soulcore` refuses an approval that does not match.
 */
export interface E1DraftPlan {
  readonly preparation_id: string;
  readonly plan_hash: string;
  readonly model: string;
  readonly third_party_turns: number;
  readonly placeheld_turns: number;
  readonly carries_exempted_original: boolean;
  readonly notice: string;
  readonly not_sent_notice: string;
}

/** What the shell sends back when the user says yes. Both halves, verbatim. */
export interface Approval {
  readonly preparation_id: string;
  readonly plan_hash: string;
}

/** What the drafting screen says before there is a draft on it. */
export interface DraftNotices {
  readonly not_sent: string;
  readonly e1_plan: string;
  /** Always false. */
  readonly can_send: boolean;
}

/**
 * What one export file contains, mirroring
 * `soulcore::commands::import::ImportPreview`.
 *
 * Counts, a format name and two booleans. There is no field here that could
 * hold a message, a display name or an account handle, and that is the whole
 * of "the preview does not echo the file": the shell cannot render what it was
 * never sent. `writes_anything` is typed as the literal `false` so a component
 * that treated reading a file as importing it would not compile.
 */
export interface ImportPreview {
  /** `soul-import-v1` or `telegram-desktop`. */
  readonly source: string;
  readonly participants: number;
  readonly conversations: number;
  readonly messages: number;
  /** Lines that tried to give instructions. Counted, stored, obeyed by none. */
  readonly messages_with_injection_markers: number;
  readonly owner_identified: boolean;
  readonly writes_anything: false;
  readonly notice: string;
}

/**
 * `soulcore::commands::import::MAX_IMPORT_BYTES`, the byte budget one import
 * may read. The core enforces it on every preview and commit; the import
 * screen checks the same number against `File.size` first, because reading a
 * file into the WebView just to be told it was too big to read would be the
 * cost the budget exists to refuse. `contract.test.ts` pins the two sides to
 * the same literal.
 */
export const MAX_IMPORT_BYTES = 64 * 1024 * 1024;

/**
 * `soulcore::commands::import::IMPORT_OVER_BYTE_BUDGET_NOTICE`, the sentence
 * the core refuses an over-budget file with. The screen shows this exact
 * sentence when it refuses before reading, so the user is told the same
 * thing whichever side said no; `contract.test.ts` pins the twins equal.
 */
export const IMPORT_OVER_BYTE_BUDGET_NOTICE =
  "这个文件太大，没有读：这一版一次最多读 64 MB 的导出文本。把导出按时间段拆成几份小的，一份一份来。";

/** What one import wrote. Counts, for the same reason the preview is counts. */
export interface ImportReceipt {
  readonly source: string;
  readonly contacts_created: number;
  readonly contacts_matched: number;
  readonly events_written: number;
  readonly evidence_written: number;
  readonly messages_with_injection_markers: number;
  readonly ties_rebuilt: number;
  readonly notice: string;
}

/** What answering one question moves. */
export type QuestionMoves = "axis" | "voice" | "boundary" | "value";

/**
 * One answer a question offers: the recorder's token, and the words for it.
 *
 * `value` is what goes back, because `soul-import` declares the closed set so
 * it can refuse an option nobody offered. `reading` is built on the Rust side
 * for the reason every other sentence on screen is: that is where the
 * denylist can see it.
 */
export interface QuestionOption {
  readonly value: string;
  readonly reading: string;
}

/**
 * One of the eleven questions, as `soul_import::questionnaire::QUESTIONS`
 * declares it.
 */
export interface Question {
  readonly question_id: string;
  readonly prompt: string;
  readonly moves: QuestionMoves;
  readonly options: readonly QuestionOption[];
  /** True for a text box. Its answer is sealed and never comes back here. */
  readonly prose: boolean;
}

/** What the user gave. Blank means the question was skipped. */
export interface GivenAnswer {
  readonly question_id: string;
  readonly given: string;
}

/** What one questionnaire run left behind. AC-03 is `profile_is_empty`. */
export interface IgnoredAnswer {
  readonly question_id: string;
  readonly reason: string;
}

export interface IntakeReceipt {
  readonly answered: number;
  readonly axes_known: number;
  /** Axes nobody answered for. They stay `unknown`; nothing is guessed. */
  readonly axes_unknown: number;
  readonly voice_fields_user_set: number;
  readonly stated_entries: number;
  readonly profile_is_empty: boolean;
  readonly evidence_ids: readonly string[];
  readonly ignored: readonly IgnoredAnswer[];
}

export interface InferenceRow {
  readonly inference_id: string;
  readonly position: string;
  readonly band: string;
  readonly evidence_count: number;
  readonly state: string;
  readonly falsifier: string | null;
}

/** One position an axis can be corrected to, in that axis's own words. */
export interface AxisChoice {
  readonly position: string;
  readonly reading: string;
}

export interface AxisRow {
  readonly axis_id: string;
  readonly label: string;
  readonly position: string;
  /** The axis in words, built in the core where the denylist could see it. */
  readonly reading: string;
  readonly evidence_band: string;
  readonly evidence_count: number;
  readonly locked_by_user: boolean;
  readonly choices: readonly AxisChoice[];
  /** Every stored inference, including the ones a lock refused to apply. */
  readonly inferences: readonly InferenceRow[];
}

/** One value a voice field can take, and the word for it. */
export interface VoiceOption {
  readonly value: string;
  readonly reading: string;
}

export interface VoiceFieldRow {
  readonly field: string;
  /** What the field is about, in words. */
  readonly label: string;
  readonly value: string;
  readonly locked_by_user: boolean;
  readonly options: readonly VoiceOption[];
  readonly question_id: string | null;
}

export interface VoiceView {
  readonly fields: readonly VoiceFieldRow[];
  readonly reading: string;
}

/**
 * One boundary or value the user stated, as a pointer.
 *
 * There is no field on this that holds what they wrote. The words are sealed
 * in the event the recorder wrote, and nothing on this path opens the seal.
 */
export interface StatedRow {
  readonly field: string;
  readonly question_id: string;
  readonly prompt: string;
  readonly event_id: string;
  readonly evidence_id: string;
}

export interface ProfileScreen {
  readonly profile_id: string;
  readonly axes: readonly AxisRow[];
  readonly voice: VoiceView;
  readonly stated: readonly StatedRow[];
  readonly reading: string;
  readonly positions: readonly string[];
  readonly notice: string;
}

/** One memory in a list, with its prose left sealed. */
export interface MemoryRow {
  readonly memory_id: string;
  readonly memory_type: string;
  readonly forget_state: string;
  readonly third_party_content_present: boolean;
  readonly title_chars: number;
  readonly summary_chars: number;
}

export interface MemoryList {
  readonly memories: readonly MemoryRow[];
  readonly memory_types: readonly string[];
  readonly forget_notice: string;
}

/** One memory, opened because the user asked for this one. */
export interface MemoryDetail {
  readonly memory_id: string;
  readonly memory_type: string;
  readonly title: string;
  readonly summary: string;
  readonly third_party_content_present: boolean;
  readonly content_key_id: string;
}

export interface NewMemory {
  readonly memory_type: string;
  readonly title: string;
  readonly summary: string;
}

/** A change to one. An absent field is left as it is. */
export interface MemoryChange {
  readonly memory_type?: string | null;
  readonly title?: string | null;
  readonly summary?: string | null;
}

/**
 * What forgetting one memory would cost.
 *
 * `destroys_anything` is typed as the literal `false` so a component that
 * tried to treat the preview as the act would not compile. The act is
 * `forgetMemory`, and it refuses a confirmation that does not name this
 * `preview_id`.
 */
export interface ForgetPreview {
  readonly preview_id: string;
  readonly memory_id: string;
  readonly content_key_count: number;
  readonly memories_affected: number;
  readonly contacts_affected: number;
  readonly sealed_blobs_destroyed: number;
  readonly inferences_orphaned: number;
  readonly import_namespaces_revoked?: number;
  readonly local_insights_invalidated?: number;
  readonly assistance_items_invalidated?: number;
  readonly audit_entries_retained: number;
  readonly destroys_anything: false;
  readonly notice: string;
}

/** What the user echoes back to say they read the preview. */
export interface ForgetConfirmation {
  readonly preview_id: string;
  readonly memory_id: string;
}

export interface ForgetReceipt {
  readonly memory_id: string;
  readonly content_keys_destroyed: number;
  readonly sealed_blobs_destroyed: number;
  readonly inferences_orphaned: number;
  readonly import_namespaces_revoked?: number;
  readonly local_insights_invalidated?: number;
  readonly assistance_items_invalidated?: number;
  /** Whether the receipt charges what the preview quoted. */
  readonly matched_preview: boolean;
  /** Missing fields from a legacy peer stay unconfirmed, never default true. */
  readonly logical_committed?: boolean;
  readonly cleanup?: ForgetCleanup;
  readonly audit?: ForgetAudit;
}

export interface ForgetCleanupResult {
  readonly cleanup: ForgetCleanup;
}

/** One row of the research preview. Counts and buckets, never a body. */
export interface ResearchRow {
  readonly event_kind: string | null;
  readonly time_bucket_utc: string | null;
  readonly duration_bucket: string | null;
  readonly self_trait_axis: string | null;
  readonly self_trait_band: string | null;
  readonly aggregate_count: number | null;
}

/**
 * AC-20 as a screen.
 *
 * `written_to_disk` and `third_party_rows` are typed as the literals the core
 * can only produce: `soul_store_api::research::preview_manifest` sets the
 * first itself and `zero_third_party_rows` refuses to build a manifest with
 * anything but 0 in the second.
 */
export interface ResearchPreview {
  readonly manifest_id: string;
  readonly export_kind: string;
  readonly written_to_disk: false;
  readonly third_party_rows: 0;
  readonly candidate_rows_total: number;
  readonly third_party_rows_excluded: number;
  /**
   * Candidates that are the owner's own and were still dropped, because what
   * is stored on them says research may not count them by the hour. Imported
   * messages and questionnaire answers are the ones this covers.
   */
  readonly deny_rows_excluded: number;
  readonly fields: readonly string[];
  readonly rows: readonly ResearchRow[];
  readonly third_party_body: string;
  readonly notice: string;
}

/**
 * One audit entry, played back.
 *
 * Ids, two vocabulary words, a reason code and counts. There is no field here
 * that could hold prose, and `audit.schema.json` has none either.
 */
export interface AuditEntry {
  readonly seq: number;
  readonly entry_id: string;
  readonly ts: string;
  readonly action: string;
  readonly decision: string;
  readonly reason_code: string | null;
  readonly subject_refs: readonly string[];
  readonly items: number | null;
  readonly bytes: number | null;
  readonly plan_hash: string | null;
  readonly capability_token_id: string | null;
  readonly egress_class: string | null;
  readonly prev_hash: string;
  readonly entry_hash: string;
  readonly follows_previous: boolean;
}

export interface AuditChain {
  readonly entries: readonly AuditEntry[];
  readonly verified: boolean;
  readonly verification_problem: string | null;
  readonly notice: string;
}

/**
 * The collection screen's whole state, mirroring
 * `soulcore::commands::session::CollectStatus`.
 *
 * Two booleans, a fixed source label, a count and two sentences. There is no
 * field here that could hold an application name, a window title or a path, so
 * "the screen never shows what you were doing" is a property of the type
 * rather than a rule this shell follows. `survives_restart` is typed as the
 * literal `false` because the core has no code path that sets it: consent
 * lives in the running process, and `config.json` has nowhere to keep one.
 */
export interface CollectStatus {
  readonly consent_granted: boolean;
  /** Not the same as `consent_granted`: a build with no foreground source
   *  records the consent and starts nothing. */
  readonly collector_running: boolean;
  /** `windows.foreground_process`, `fake.scripted_desktop` or `unsupported`. */
  readonly source: string;
  /** Foreground events in the store, or null when the store did not open. */
  readonly events_collected: number | null;
  readonly survives_restart: false;
  readonly duration_only_notice: string;
  readonly notice: string;
}

/**
 * Command names, spelled once.
 *
 * `apps/desktop/src-tauri/src/commands.rs` registers exactly these, and a test
 * on the Rust side compares the two lists, so a typo here is a failing test
 * rather than a button that does nothing.
 */
export const COMMANDS = {
  listImportNamespaces: "list_import_namespaces",
  recentImportOperations: "recent_import_operations",
  importOperation: "import_operation",
  discardImport: "discard_import",
  prepareImport: "prepare_import",
  confirmImport: "confirm_import",
  importRecoveryState: "import_recovery_state",
  prepareMemoryContext: "prepare_memory_context",
  prepareTaskContext: "prepare_task_context",
  prepareInsightContext: "prepare_insight_context",
  evidenceSources: "evidence_sources",
  readEvidenceEvent: "read_evidence_event",
  localDraftWithInsight: "local_draft_with_insight",
  prepareDraftWithInsight: "prepare_draft_with_insight",
  draftReplyWithMemory: "draft_reply_with_memory",
  prepareDraftWithMemory: "prepare_draft_with_memory",
  localInsights: "local_insights",
  ignoreLocalInsight: "ignore_local_insight",
  recomputeLocalInsight: "recompute_local_insight",
  assistanceItems: "assistance_items",
  assistanceItem: "assistance_item",
  createAssistanceGoal: "create_assistance_goal",
  editAssistanceGoal: "edit_assistance_goal",
  linkAssistanceObservation: "link_assistance_observation",
  refreshAssistanceObservation: "refresh_assistance_observation",
  bindAssistanceGoalSource: "bind_assistance_goal_source",
  actAssistanceItem: "act_assistance_item",
  prepareAssistanceContext: "prepare_assistance_context",
  assistanceWorkStatus: "assistance_work_status",
  assistanceItemWork: "assistance_item_work",
  retryAssistanceWork: "retry_assistance_work",
  assistanceReminder: "assistance_reminder",
  setAssistanceReminder: "set_assistance_reminder",
  assistanceFeedback: "assistance_feedback",
  setAssistanceFeedback: "set_assistance_feedback",
  assistanceCategoryPreferences: "assistance_category_preferences",
  setAssistanceCategoryPreference: "set_assistance_category_preference",
  retryAssistanceNotificationWithdrawal: "retry_assistance_notification_withdrawal",
  assistanceNotificationRuntimeStatus: "assistance_notification_runtime_status",
  assistanceLocalPreparation: "assistance_local_preparation",
  setAssistanceLocalPreparation: "set_assistance_local_preparation",
  previewAssistanceLocalTemplate: "preview_assistance_local_template",
  configSnapshot: "config_snapshot",
  sessionStatus: "session_status",
  completeWizard: "complete_wizard",
  cloudToggle: "cloud_toggle",
  filesView: "files_view",
  authorizeDirectory: "authorize_directory",
  previewPlan: "preview_plan",
  peopleGraph: "people_graph",
  personSummary: "person_summary",
  correctTie: "correct_tie",
  releaseTie: "release_tie",
  draftReply: "draft_reply",
  draftNotices: "draft_notices",
  prepareDraft: "prepare_draft",
  generateDraft: "generate_draft",
  discardDraft: "discard_draft",
  setUserEndpoint: "set_user_endpoint",
  clearUserEndpoint: "clear_user_endpoint",
  previewSoulImportV1: "preview_soul_import_v1",
  previewTelegram: "preview_telegram",
  commitSoulImportV1: "commit_soul_import_v1",
  commitTelegram: "commit_telegram",
  questionnaire: "questionnaire",
  answerQuestionnaire: "answer_questionnaire",
  profileScreen: "profile_screen",
  correctAxis: "correct_axis",
  setVoice: "set_voice",
  memoryList: "memory_list",
  memoryDetail: "memory_detail",
  createMemory: "create_memory",
  updateMemory: "update_memory",
  previewForget: "preview_forget",
  forgetMemory: "forget_memory",
  retryForgetCleanup: "retry_forget_cleanup",
  researchPreview: "research_preview",
  prepareResearchSave: "prepare_research_save",
  confirmResearchSave: "confirm_research_save",
  cancelResearchSave: "cancel_research_save",
  auditChain: "audit_chain",
  collectStatus: "collect_status",
  grantCollectConsent: "grant_collect_consent",
  revokeCollectConsent: "revoke_collect_consent",
} as const;

export type ImportSource = "soul-import-v1" | "telegram-desktop";
export interface SelectedMemory {
  readonly memory_id: string; readonly memory_type: string; readonly title: string; readonly summary: string;
  readonly privacy_class: "owner" | "third_party" | "mixed" | "unknown";
  readonly citation: { readonly memory_id: string; readonly title_blob_id: string; readonly summary_blob_id: string };
}
export interface MemorySelectionPreview {
  readonly selected: readonly SelectedMemory[];
  readonly excluded: readonly { readonly memory_id: string; readonly reason: string }[];
  readonly outcome: "context" | "clarify" | "abstain"; readonly notice: string;
}
export interface MemoryContextView { readonly context_id: string | null; readonly preview: MemorySelectionPreview }
export interface MemoryContextRequest {
  readonly candidate_ids: readonly string[]; readonly query: string | null; readonly memory_type: string | null; readonly time_specific: boolean;
}
export interface TaskContextRequest { readonly candidate_ids: readonly string[]; readonly task: string }
export interface TaskAssessment {
  readonly status: "matched" | "clarify" | "unknown" | "budget_limited";
  readonly requirements: readonly { readonly field: string; readonly status: "matched" | "missing" | "conflict" | "unknown"; readonly source_ids: readonly string[] }[];
  readonly supplemented_ids: readonly string[]; readonly question: string | null; readonly notice: string;
}
export interface TaskContextView { readonly memory: MemoryContextView; readonly assessment: TaskAssessment }

/** Local task checking uses the same source revision and explicit candidate boundary. */
export async function prepareTaskContext(request: TaskContextRequest): Promise<TaskContextView> {
  const captured = { candidate_ids: [...request.candidate_ids], task: request.task };
  invalidateDraftSource();
  const value = await currentDraftResult<TaskContextView>(COMMANDS.prepareTaskContext, { request: captured });
  const allowed = new Set(captured.candidate_ids);
  const selected = new Set(value.memory.preview.selected.map((entry) => entry.memory_id));
  if (value.memory.preview.selected.some((entry) => !allowed.has(entry.memory_id) || entry.citation.memory_id !== entry.memory_id)
    || value.memory.preview.excluded.some((entry) => !allowed.has(entry.memory_id))
    || value.assessment.requirements.some((need) => need.source_ids.some((id) => !allowed.has(id)))
    || value.assessment.supplemented_ids.some((id) => !selected.has(id))
    || (value.assessment.status !== "matched" && value.memory.context_id !== null)
    || (value.assessment.status === "matched" && (value.memory.context_id === null || value.memory.preview.outcome !== "context"
      || value.assessment.requirements.length === 0 || value.assessment.question !== null
      || value.assessment.requirements.some((need) => need.status !== "matched" || !need.source_ids.some((id) => selected.has(id)))))) {
    throw new Error("任务核对返回的范围或状态不符，请重新核对。");
  }
  return value;
}
export interface MemoryDraftValue { readonly draft: Draft; readonly context: MemorySelectionPreview }
export interface MemoryDraftPlan {
  readonly prepared: { readonly plan: E1DraftPlan; readonly retained_memory_ids: readonly string[]; readonly endpoint_owner_ids: readonly string[]; readonly encoded_bytes: number; readonly body_preview: string };
  readonly context: MemorySelectionPreview;
}
/** Revoke old display ownership before requesting a new, read-only local grant. */
export function prepareMemoryContext(request: MemoryContextRequest): Promise<MemoryContextView> {
  invalidateDraftSource();
  return currentDraftResult(COMMANDS.prepareMemoryContext, { request });
}
export function draftReplyWithMemory(pasted: string, contextId: string): Promise<MemoryDraftValue> {
  return currentDraftResult(COMMANDS.draftReplyWithMemory, { pasted, contextId });
}
export function prepareDraftWithMemory(pasted: string, contextId: string, ownerIds: readonly string[], includeOriginal = false): Promise<MemoryDraftPlan> {
  return currentDraftResult(COMMANDS.prepareDraftWithMemory, { pasted, contextId, ownerIds, includeOriginal });
}
export interface ImportCounts {
  readonly participants: number; readonly messages: number; readonly duplicates: number;
  readonly contacts_created: number; readonly contacts_matched: number; readonly events_written: number;
  readonly evidence_written: number; readonly messages_with_injection_markers: number; readonly ties_rebuilt: number;
}
export interface InsightContextRequest {
  readonly bundle_id: string; readonly contact_id: string;
}
export interface EvidenceSourcesRequest {
  readonly contact_id: string; readonly relationship_id: string; readonly evidence_id: string;
}
export interface EvidenceEventRequest extends EvidenceSourcesRequest { readonly event_id: string }
export type EvidenceEventAvailability = "available" | "no_body" | "unavailable" | "too_large";
export interface EvidenceEventSource {
  readonly event_id: string; readonly namespace_id: string | null; readonly source: string;
  readonly occurred_at: string; readonly direction: "incoming" | "outgoing"; readonly venue: "direct" | "group";
  readonly body_chars: number | null; readonly availability: EvidenceEventAvailability; readonly notice: string;
}
export interface EvidenceSourcesView extends EvidenceSourcesRequest {
  readonly kind: string; readonly events: readonly EvidenceEventSource[]; readonly notice: string;
}
export interface EvidenceEventView extends EvidenceEventRequest {
  readonly namespace_id: string; readonly source: string; readonly occurred_at: string;
  readonly direction: "incoming" | "outgoing"; readonly venue: "direct" | "group";
  readonly body: string; readonly notice: string;
}
export function evidenceSources(request: EvidenceSourcesRequest): Promise<EvidenceSourcesView> {
  return currentDraftResult(COMMANDS.evidenceSources, { request });
}
export function readEvidenceEvent(request: EvidenceEventRequest): Promise<EvidenceEventView> {
  return currentDraftResult(COMMANDS.readEvidenceEvent, { request });
}
export interface InsightEvidenceSource {
  readonly evidence_id: string; readonly kind: string; readonly event_ids: readonly string[];
}
export interface InsightReference {
  readonly bundle_id: string; readonly operation_id: string; readonly namespace_id: string;
  readonly contact_id: string; readonly relationship_id: string; readonly summary: PersonSummary;
  readonly sources: readonly InsightEvidenceSource[]; readonly notice: string;
}
export interface InsightContextView { readonly context_id: string; readonly reference: InsightReference }
export interface InsightDraftValue { readonly draft: Draft; readonly reference: InsightReference }
export interface InsightDraftPlan {
  readonly prepared: { readonly plan: E1DraftPlan; readonly encoded_bytes: number; readonly body_preview: string; readonly includes_counts: boolean; readonly includes_goal: boolean };
  readonly reference: InsightReference;
  readonly goal_preview: MemorySelectionPreview | null;
}
export function prepareInsightContext(request: InsightContextRequest): Promise<InsightContextView> {
  invalidateDraftSource();
  return currentDraftResult(COMMANDS.prepareInsightContext, { request });
}
export function localDraftWithInsight(pasted: string, contextId: string): Promise<InsightDraftValue> {
  return currentDraftResult(COMMANDS.localDraftWithInsight, { pasted, contextId });
}
export function prepareDraftWithInsight(pasted: string, contextId: string, includeCounts = false, includeOriginal = false, includeGoal = false): Promise<InsightDraftPlan> {
  return currentDraftResult(COMMANDS.prepareDraftWithInsight, { pasted, contextId, includeCounts, includeOriginal, includeGoal });
}
export interface LocalInsight {
  readonly bundle_id: string; readonly operation_id: string; readonly namespace_id: string;
  readonly validity: "active" | "invalidated"; readonly ignored: boolean; readonly counts: ImportCounts;
  readonly summaries: readonly PersonSummary[]; readonly notice: string;
}
export function localInsights(): Promise<readonly LocalInsight[]> {
  return currentDraftResult(COMMANDS.localInsights, {});
}
export function ignoreLocalInsight(bundleId: string): Promise<LocalInsight> {
  return mutateDraftSource(COMMANDS.ignoreLocalInsight, { bundleId });
}
export function recomputeLocalInsight(bundleId: string): Promise<LocalInsight> {
  return mutateDraftSource(COMMANDS.recomputeLocalInsight, { bundleId });
}

export interface AssistancePointer { readonly item_id: string; readonly expected_revision: string }
export interface AssistanceObservationSource {
  readonly item_id: string; readonly revision: string; readonly namespace_id: string;
  readonly contact_id: string; readonly bundle_id: string;
}
export type AssistanceSource =
  | { readonly kind: "manual_goal"; readonly memory_id: string; readonly observation: AssistanceObservationSource | null }
  | { readonly kind: "observation"; readonly namespace_id: string; readonly contact_id: string; readonly bundle_id: string };
export interface AssistanceItemRow {
  readonly item_id: string; readonly revision: string; readonly source: AssistanceSource;
  readonly validity: "current" | "stale" | "forgotten";
  readonly handling: "open" | "cancelled" | "resolved_reported";
  readonly deadline_at: string | null; readonly created_at: number; readonly updated_at: number;
}
export interface AssistanceItemDetail {
  readonly item: AssistanceItemRow; readonly goal: MemoryDetail | null;
  readonly observation: InsightReference | null; readonly notice: string;
}
export interface AssistanceGoalFields { readonly title: string; readonly summary: string; readonly deadline_at: string | null }
/** Native returns revision, while mutation requests name expected_revision. */
export type AssistanceContextView =
  | { readonly item_id: string; readonly revision: string; readonly kind: "memory"; readonly memory: MemoryContextView }
  | { readonly item_id: string; readonly revision: string; readonly kind: "insight"; readonly insight: InsightContextView; readonly goal_preview: MemorySelectionPreview | null };

function checkedAssistanceDetail(value: AssistanceItemDetail, itemId?: string): AssistanceItemDetail {
  if ((itemId !== undefined && value.item.item_id !== itemId)
    || (value.goal !== null && (value.item.source.kind !== "manual_goal" || value.goal.memory_id !== value.item.source.memory_id))) {
    throw new Error("事项返回的身份不符，结果尚未确认，请重新读取。");
  }
  const source = value.item.source.kind === "observation" ? value.item.source : value.item.source.observation;
  if (value.observation !== null && (source === null || value.observation.bundle_id !== source.bundle_id
    || value.observation.contact_id !== source.contact_id || value.observation.namespace_id !== source.namespace_id
    || value.observation.summary.contact_id !== source.contact_id)) throw new Error("事项返回的观察来源不符，请重新读取。");
  return value;
}
export function assistanceItems(): Promise<{ readonly items: readonly AssistanceItemRow[] }> {
  return currentDraftResult(COMMANDS.assistanceItems, {});
}
export async function assistanceItem(itemId: string): Promise<AssistanceItemDetail> {
  return checkedAssistanceDetail(await currentDraftResult(COMMANDS.assistanceItem, { itemId }), itemId);
}
export async function createAssistanceGoal(request: AssistanceGoalFields & { readonly source_observation?: AssistancePointer | null }): Promise<AssistanceItemDetail> {
  return checkedAssistanceDetail(await mutateDraftSource(COMMANDS.createAssistanceGoal, { request }));
}
export async function editAssistanceGoal(request: AssistanceGoalFields & AssistancePointer): Promise<AssistanceItemDetail> {
  return checkedAssistanceDetail(await mutateDraftSource(COMMANDS.editAssistanceGoal, { request }), request.item_id);
}
export async function linkAssistanceObservation(request: { readonly bundle_id: string; readonly contact_id: string }, expectedNamespace?: string): Promise<AssistanceItemDetail> {
  const value = checkedAssistanceDetail(await mutateDraftSource<AssistanceItemDetail>(COMMANDS.linkAssistanceObservation, { request }));
  // An existing stable observation keeps its old bundle until explicit refresh.
  if (value.item.source.kind !== "observation" || value.item.source.contact_id !== request.contact_id
    || (expectedNamespace !== undefined && value.item.source.namespace_id !== expectedNamespace)) throw new Error("观察事项返回的来源身份不符，请重新读取。");
  return value;
}
export async function refreshAssistanceObservation(request: AssistancePointer & { readonly bundle_id: string }): Promise<AssistanceItemDetail> {
  return checkedAssistanceDetail(await mutateDraftSource(COMMANDS.refreshAssistanceObservation, { request }), request.item_id);
}
export async function bindAssistanceGoalSource(request: AssistancePointer & { readonly source_observation: AssistancePointer | null }): Promise<AssistanceItemDetail> {
  return checkedAssistanceDetail(await mutateDraftSource(COMMANDS.bindAssistanceGoalSource, { request }), request.item_id);
}
export async function actAssistanceItem(request: AssistancePointer & { readonly action: "cancel" | "complete_self_reported" }): Promise<AssistanceItemDetail> {
  return checkedAssistanceDetail(await mutateDraftSource(COMMANDS.actAssistanceItem, { request }), request.item_id);
}
export async function prepareAssistanceContext(request: AssistancePointer): Promise<AssistanceContextView> {
  invalidateDraftSource();
  const value = await currentDraftResult<AssistanceContextView>(COMMANDS.prepareAssistanceContext, { request });
  if (value.item_id !== request.item_id || value.revision !== request.expected_revision) throw new Error("事项准备返回的版本身份不符，请重新核对。");
  return value;
}
export interface AssistanceWorkMetadata {
  readonly item_id: string; readonly item_revision: string;
  readonly state: "none" | "pending" | "running" | "ready" | "source_unavailable" | "failed" | "unknown";
  readonly pending_revision: string | null; readonly claimed_revision: string | null;
  readonly claim_id: string | null; readonly result_revision: string | null; readonly failure_code: string | null;
}
export interface AssistanceWorkStatus {
  readonly active_worker: boolean; readonly active_item_id: string | null;
  readonly pending_items: number | null; readonly claimed_items: number | null;
  readonly foreground_preparation_held: boolean; readonly coordination_unknown: boolean;
  readonly collector_paused_for_import: boolean; readonly can_begin: boolean;
  readonly runtime_state: "not_started" | "running" | "stopped" | "failed"; readonly last_runtime_error: string | null;
}
export function assistanceWorkStatus(): Promise<AssistanceWorkStatus> {
  return currentDraftResult(COMMANDS.assistanceWorkStatus, {});
}
export async function assistanceItemWork(itemId: string): Promise<AssistanceWorkMetadata> {
  const value = await currentDraftResult<AssistanceWorkMetadata>(COMMANDS.assistanceItemWork, { itemId });
  if (value.item_id !== itemId) throw new Error("本地工作状态返回的事项身份不符，请重新核对。");
  return value;
}
export async function retryAssistanceWork(request: AssistancePointer): Promise<AssistanceWorkMetadata> {
  // Retrying local work does not revoke or replace a held foreground context.
  const value = await currentDraftResult<AssistanceWorkMetadata>(COMMANDS.retryAssistanceWork, { request });
  if (value.item_id !== request.item_id || value.item_revision !== request.expected_revision) throw new Error("本地重试返回的事项版本不符，结果尚未确认，请重新核对。");
  return value;
}
export interface NotificationWithdrawal {
  readonly withdrawal_id: string; readonly attempt_id: string;
  readonly state: "pending" | "claimed" | "withdrawn" | "failed" | "unknown" | "not_present";
}
export interface AssistanceNotificationAttempt {
  readonly attempt_id: string; readonly item_id: string; readonly item_revision: string; readonly schedule_revision: string;
  readonly delivery: "claimed" | "submitted_to_os" | "failed" | "unknown";
  readonly shown_at: number | null; readonly opened_at: number | null; readonly hidden_at: number | null;
  readonly timed_out_at: number | null; readonly tracking_expired_at: number | null; readonly latest_withdrawal: NotificationWithdrawal | null;
}
export interface AssistanceReminderView {
  readonly item_id: string; readonly item_revision: string; readonly schedule_revision: string | null;
  readonly state: "off" | "scheduled" | "invalidated" | "expired_in_inbox" | "suppressed" | "check_failed";
  readonly remind_at: string | null; readonly latest_attempt: AssistanceNotificationAttempt | null; readonly failure_code: string | null;
}
export interface AssistanceFeedbackView {
  readonly item_id: string; readonly item_revision: string; readonly feedback_revision: string | null;
  readonly value: "none" | "helpful" | "not_relevant";
}
export type AssistanceCategory = "manual_goal" | "observation";
export interface AssistanceCategoryPreference { readonly category: AssistanceCategory; readonly revision: string | null; readonly muted: boolean }
export interface AssistanceCategoryPreferences { readonly preferences: readonly AssistanceCategoryPreference[] }
export interface AssistanceNotificationRuntimeStatus {
  readonly state: "not_started" | "running" | "unsupported" | "failed" | "stopping" | "stopped"; readonly last_error: string | null;
}
function checkedReminder(value: AssistanceReminderView, itemId: string, revision?: string): AssistanceReminderView {
  if (value.item_id !== itemId || (revision !== undefined && value.item_revision !== revision)
    || (value.latest_attempt !== null && (value.latest_attempt.item_id !== itemId || (value.latest_attempt.latest_withdrawal !== null
      && value.latest_attempt.latest_withdrawal.attempt_id !== value.latest_attempt.attempt_id)))) throw new Error("提醒返回的事项或通知身份不符，请重新核对。");
  return value;
}
export async function assistanceReminder(itemId: string): Promise<AssistanceReminderView> {
  return checkedReminder(await currentDraftResult(COMMANDS.assistanceReminder, { itemId }), itemId);
}
export async function setAssistanceReminder(request: AssistancePointer & { readonly expected_schedule_revision: string | null; readonly remind_at: string | null }): Promise<AssistanceReminderView> {
  const value = checkedReminder(await currentDraftResult(COMMANDS.setAssistanceReminder, { request }), request.item_id, request.expected_revision);
  if (value.schedule_revision === null || value.schedule_revision === request.expected_schedule_revision
    || value.remind_at !== request.remind_at || value.state !== (request.remind_at === null ? "off" : "scheduled")) throw new Error("提醒更改返回的安排版本或选择不符，请重新核对。");
  return value;
}
function checkedFeedback(value: AssistanceFeedbackView, itemId: string, revision?: string): AssistanceFeedbackView {
  if (value.item_id !== itemId || (revision !== undefined && value.item_revision !== revision)) throw new Error("反馈返回的事项版本不符，请重新核对。"); return value;
}
export async function assistanceFeedback(itemId: string): Promise<AssistanceFeedbackView> {
  return checkedFeedback(await currentDraftResult(COMMANDS.assistanceFeedback, { itemId }), itemId);
}
export async function setAssistanceFeedback(request: AssistancePointer & { readonly expected_feedback_revision: string | null; readonly value: "helpful" | "not_relevant" | "clear" }): Promise<AssistanceFeedbackView> {
  const value = checkedFeedback(await currentDraftResult(COMMANDS.setAssistanceFeedback, { request }), request.item_id, request.expected_revision);
  if (value.feedback_revision === null || value.feedback_revision === request.expected_feedback_revision
    || value.value !== (request.value === "clear" ? "none" : request.value)) throw new Error("反馈更改返回的版本或选择不符，请重新核对。");
  return value;
}
export async function assistanceCategoryPreferences(): Promise<AssistanceCategoryPreferences> {
  const value = await currentDraftResult<AssistanceCategoryPreferences>(COMMANDS.assistanceCategoryPreferences, {});
  if (value.preferences.length !== 2 || !["manual_goal", "observation"].every((category) => value.preferences.filter((row) => row.category === category).length === 1)) throw new Error("提醒类别返回的范围不符，请重新核对。"); return value;
}
export async function setAssistanceCategoryPreference(request: { readonly category: AssistanceCategory; readonly expected_revision: string | null; readonly muted: boolean }): Promise<AssistanceCategoryPreference> {
  const value = await currentDraftResult<AssistanceCategoryPreference>(COMMANDS.setAssistanceCategoryPreference, { request });
  if (value.category !== request.category || value.muted !== request.muted || value.revision === null || value.revision === request.expected_revision) throw new Error("提醒类别返回的版本或选择不符，请重新核对。"); return value;
}
export async function retryAssistanceNotificationWithdrawal(request: { readonly attempt_id: string; readonly expected_withdrawal_id: string | null }): Promise<NotificationWithdrawal> {
  const value = await currentDraftResult<NotificationWithdrawal>(COMMANDS.retryAssistanceNotificationWithdrawal, { request });
  if (value.attempt_id !== request.attempt_id || value.withdrawal_id === request.expected_withdrawal_id || !value.withdrawal_id
    || !["pending", "claimed", "withdrawn", "failed", "unknown", "not_present"].includes(value.state)) throw new Error("撤除返回的通知或请求身份不符，请重新核对。"); return value;
}
export function assistanceNotificationRuntimeStatus(): Promise<AssistanceNotificationRuntimeStatus> { return currentDraftResult(COMMANDS.assistanceNotificationRuntimeStatus, {}); }
export interface AssistanceLocalPreparation {
  readonly item_id: string; readonly item_revision: string; readonly preference_revision: string | null;
  readonly enabled: boolean; readonly state: "off" | "none" | "pending" | "running" | "ready" | "source_unavailable" | "failed" | "unknown";
  readonly template_id: string | null; readonly failure_code: string | null;
}
export interface AssistanceLocalTemplateRequest extends AssistancePointer {
  readonly expected_preference_revision: string; readonly template_id: string;
}
export interface AssistanceLocalTemplatePreview {
  readonly item_id: string; readonly item_revision: string; readonly preference_revision: string; readonly template_id: string;
  readonly kind: "opening_v1"; readonly prepared_at: number; readonly body: string;
}
export const LOCAL_TEMPLATE_BODY_SLOT = "（这里写你要说的内容）";
function checkedLocalPreparation(value: AssistanceLocalPreparation, itemId: string, revision?: string): AssistanceLocalPreparation {
  if (value.item_id !== itemId || (revision !== undefined && value.item_revision !== revision)
    || (value.state === "ready" && (!value.enabled || value.preference_revision === null || value.template_id === null))) throw new Error("本地模板准备返回的事项版本或状态不符，请重新核对。");
  return value;
}
export async function assistanceLocalPreparation(itemId: string): Promise<AssistanceLocalPreparation> {
  return checkedLocalPreparation(await currentDraftResult(COMMANDS.assistanceLocalPreparation, { itemId }), itemId);
}
export async function setAssistanceLocalPreparation(request: AssistancePointer & { readonly expected_preference_revision: string | null; readonly enabled: boolean }): Promise<AssistanceLocalPreparation> {
  const value = checkedLocalPreparation(await currentDraftResult(COMMANDS.setAssistanceLocalPreparation, { request }), request.item_id, request.expected_revision);
  if (value.preference_revision === null || value.preference_revision === request.expected_preference_revision || value.enabled !== request.enabled
    || (!request.enabled && (value.state !== "off" || value.template_id !== null))) throw new Error("本地模板偏好返回的版本或选择不符，请重新核对。");
  return value;
}
export async function previewAssistanceLocalTemplate(request: AssistanceLocalTemplateRequest): Promise<AssistanceLocalTemplatePreview> {
  const value = await currentDraftResult<AssistanceLocalTemplatePreview>(COMMANDS.previewAssistanceLocalTemplate, { request });
  if (value.item_id !== request.item_id || value.item_revision !== request.expected_revision || value.preference_revision !== request.expected_preference_revision
    || value.template_id !== request.template_id || value.kind !== "opening_v1" || !Number.isFinite(value.prepared_at)
    || typeof value.body !== "string" || !value.body.includes(LOCAL_TEMPLATE_BODY_SLOT)) throw new Error("本地模板预览返回的事项、偏好、模板或正文占位不符，请重新核对。");
  return value;
}
export interface ImportNamespace {
  readonly namespace_id: string; readonly source: ImportSource;
  readonly state: "active" | "revoked"; readonly coverage: "full_from_creation" | "forward_only";
}
export interface ImportOperation {
  readonly operation_id: string; readonly preview_id: string; readonly source: ImportSource;
  readonly namespace_id: string; readonly legacy_choice: "reject" | "forward_only";
  readonly state: "reserved" | "running" | "committed" | "rejected" | "discarded" | "invalidated" | "interrupted";
  readonly preview: ImportCounts; readonly receipt: ImportCounts | null; readonly error_code: string | null;
  readonly audit_state: "not_applicable" | "recorded" | "unconfirmed"; readonly created_at: number;
}
export interface ImportOperationView {
  readonly operation: ImportOperation; readonly namespace_revoked: boolean; readonly prepared_here: boolean;
}
export interface ImportRecoveryState {
  readonly active_worker: boolean; readonly reserved_operations: number | null; readonly running_operations: number | null;
  readonly held_operation_id: string | null; readonly can_start_new_intent: boolean;
}
export function importRecoveryState(): Promise<ImportRecoveryState> {
  return invoke(COMMANDS.importRecoveryState);
}
export interface PreparedImport {
  readonly operation: ImportOperation; readonly preview: ImportPreview; readonly legacy_events: number; readonly notice: string;
}
export interface ImportPrepareRequest {
  readonly text: string; readonly source: ImportSource; readonly namespace_id: string | null;
  readonly legacy_choice: "reject" | "forward_only";
}

export function listImportNamespaces(): Promise<readonly ImportNamespace[]> {
  return invoke(COMMANDS.listImportNamespaces);
}
export function recentImportOperations(): Promise<readonly ImportOperationView[]> {
  return invoke(COMMANDS.recentImportOperations);
}
export function importOperation(operationId: string): Promise<ImportOperationView> {
  return invoke(COMMANDS.importOperation, { operationId });
}
export function prepareImport(request: ImportPrepareRequest): Promise<PreparedImport> {
  return mutateDraftSource(COMMANDS.prepareImport, { request });
}
export function confirmImport(operationId: string, previewId: string): Promise<ImportOperationView> {
  return mutateDraftSource(COMMANDS.confirmImport, { operationId, previewId });
}
export function discardImport(operationId: string): Promise<ImportOperationView> {
  return mutateDraftSource(COMMANDS.discardImport, { operationId });
}

/**
 * Display ownership within this WebView, not an authorization token. The core
 * independently validates its own source revision and one-use approval.
 * A fresh identity also distinguishes setting the same endpoint again and A/B/A.
 */
export type DraftSourceRevision = symbol;
let draftSourceRevision: DraftSourceRevision = Symbol("draft source");
let pendingDraftSourceMutations = 0;
const draftSourceListeners = new Set<(revision: DraftSourceRevision) => void>();

export function getDraftSourceRevision(): DraftSourceRevision {
  return draftSourceRevision;
}

/** Subscribe synchronously, including the current value so mounting has no gap. */
export function subscribeDraftSource(
  listener: (revision: DraftSourceRevision) => void,
): () => void {
  draftSourceListeners.add(listener);
  listener(draftSourceRevision);
  return () => { draftSourceListeners.delete(listener); };
}

/** A local delivery refusal; deliberately not a backend `Refusal` or rollback. */
export class DraftSourceChangedError extends Error {
  constructor() {
    super("起草依据或授权正在变更或已变更，请重新准备。这次旧结果不再展示。");
    this.name = "DraftSourceChangedError";
  }
}

export class SummarySourceChangedError extends Error {
  constructor() {
    super("摘要依据或端点正在变更或已变更，请重新选择联系人查看摘要。");
    this.name = "SummarySourceChangedError";
  }
}

export class FilePlanSourceChangedError extends Error {
  constructor() {
    super("计划依据或授权正在变更或已变更，请重新扫描。");
    this.name = "FilePlanSourceChangedError";
  }
}

function invalidateDraftSource(): void {
  draftSourceRevision = Symbol("draft source");
  for (const listener of [...draftSourceListeners]) listener(draftSourceRevision);
}

/**
 * Invalidate before IPC, even when the request fails or its outcome is unknown.
 * Pending mutations block draft admission and delivery. Settle invalidates
 * again before making the latest source available to a new request.
 * This covers calls through this module in one WebView, not other windows or
 * direct native callers. It does not cancel a request or retract sent bytes.
 */
async function mutateDraftSource<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  pendingDraftSourceMutations += 1;
  try {
    invalidateDraftSource();
    return await invoke<T>(command, args);
  } finally {
    pendingDraftSourceMutations -= 1;
    invalidateDraftSource();
  }
}

function currentDraftResult<T>(command: string, args: Record<string, unknown>,
  changed: () => Error = () => new DraftSourceChangedError()): Promise<T> {
  if (pendingDraftSourceMutations !== 0) return Promise.reject(changed());
  const source = draftSourceRevision;
  return invoke<T>(command, args).then((value) => {
    if (pendingDraftSourceMutations !== 0 || source !== draftSourceRevision) {
      throw changed();
    }
    return value;
  });
}

export function configSnapshot(): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.configSnapshot);
}

/** Whether the wizard is behind us, and whether there is a database to read. */
export function sessionStatus(): Promise<SessionStatus> {
  return invoke<SessionStatus>(COMMANDS.sessionStatus);
}

/** Finish the first-run wizard. The core refuses to return an open config. */
export function completeWizard(answers: WizardAnswers): Promise<ConfigSnapshot> {
  return mutateDraftSource<ConfigSnapshot>(COMMANDS.completeWizard, { answers });
}

/**
 * The user pressed the cloud switch.
 *
 * `requestedOn` is passed through so the core sees what was asked for; the
 * answer does not depend on it, and that is the point of AC-22.
 */
export function cloudToggle(requestedOn: boolean): Promise<CloudNotice> {
  return invoke<CloudNotice>(COMMANDS.cloudToggle, { requestedOn });
}

/** The directories this installation may read, as the core remembers them. */
export function filesView(): Promise<FilesView> {
  return invoke<FilesView>(COMMANDS.filesView);
}

/** The user named a directory Soul may read. Refused paths come back as one. */
export function authorizeDirectory(path: string): Promise<FilesView> {
  return mutateDraftSource<FilesView>(COMMANDS.authorizeDirectory, { path });
}

/**
 * Read one authorized directory and describe what tidying it would mean.
 *
 * There is no companion call that carries the plan out, in this file or in the
 * Rust one. v0.1 promises a preview; v0.1.1 owns the write half.
 */
export function previewPlan(path: string): Promise<PlanPreview> {
  return currentDraftResult<PlanPreview>(COMMANDS.previewPlan, { path },
    () => new FilePlanSourceChangedError());
}

/** The people graph, with every tie's evidence already resolved. */
export function peopleGraph(): Promise<PeopleGraph> {
  return invoke<PeopleGraph>(COMMANDS.peopleGraph);
}

/** Everything Soul will say about one person, and what each line rests on. */
export function personSummary(contactId: string): Promise<PersonSummary> {
  return currentDraftResult<PersonSummary>(COMMANDS.personSummary, { contactId },
    () => new SummarySourceChangedError());
}

export function correctTie(relationshipId: string, band: string): Promise<PeopleGraph> {
  return mutateDraftSource<PeopleGraph>(COMMANDS.correctTie, { relationshipId, band });
}

export function releaseTie(relationshipId: string): Promise<PeopleGraph> {
  return mutateDraftSource<PeopleGraph>(COMMANDS.releaseTie, { relationshipId });
}

/**
 * Write a reply to something the user pasted. Nothing leaves the machine.
 *
 * `pasted` is treated as somebody else's words by the core, without being read
 * for clues about whose they are. There is no second argument, and in
 * particular there is nowhere to put a recipient: the core has no command that
 * takes one.
 */
export function draftReply(pasted: string): Promise<Draft> {
  return currentDraftResult<Draft>(COMMANDS.draftReply, { pasted });
}

export function draftNotices(): Promise<DraftNotices> {
  return invoke<DraftNotices>(COMMANDS.draftNotices);
}

/**
 * Step one of the endpoint path: describe the request, and stop.
 *
 * Nothing has left when this resolves. What comes back is what the user is
 * being asked to approve.
 *
 * `includeOriginal` is the second confirmation PRODUCT_LOCK asks for before
 * one message's own words may travel: false unless the user pressed
 * 「这一条按原文带上」 on the confirmation panel, and false again on the next
 * call, because it is an argument and not a setting. Nothing on this side
 * remembers it — the core spends it while building the body.
 */
export function prepareDraft(
  pasted: string,
  includeOriginal = false,
): Promise<E1DraftPlan> {
  return currentDraftResult<E1DraftPlan>(COMMANDS.prepareDraft, { pasted, includeOriginal });
}

/**
 * Step two: the user read the counts and approved that exact preparation.
 *
 * The approval is the value the core handed over, echoed back unchanged. A
 * shell that assembled one itself would be approving something nobody read,
 * and `soulcore` refuses an approval whose two halves do not match.
 */
export function generateDraft(approval: Approval): Promise<Draft> {
  return currentDraftResult<Draft>(COMMANDS.generateDraft, { approval });
}

/** The user read the plan and said no. Returns whether there was one. */
export function discardDraft(): Promise<boolean> {
  return mutateDraftSource<boolean>(COMMANDS.discardDraft);
}

/**
 * The user's own OpenAI-compatible endpoint, for this run.
 *
 * The address goes one way. What comes back is the snapshot, which says
 * whether there is an endpoint and never what it is — so this shell cannot
 * redisplay it later, and neither can the next launch: the core writes it to
 * no file. Nothing is contacted here; the core parses the address and points
 * its egress guard at it, and the first request waits for the user to ask for
 * one — the approval on the drafting page, or 看这个人的摘要 on the graph.
 */
export function setUserEndpoint(url: string): Promise<ConfigSnapshot> {
  return mutateDraftSource<ConfigSnapshot>(COMMANDS.setUserEndpoint, { url });
}

/** The user took the address away. The core goes back to reaching nothing. */
export function clearUserEndpoint(): Promise<ConfigSnapshot> {
  return mutateDraftSource<ConfigSnapshot>(COMMANDS.clearUserEndpoint);
}

/**
 * Read a `soul-import-v1` JSONL file the user picked. Nothing is written.
 *
 * `text` is the file's text, read in the WebView by an `<input type="file">`:
 * the shell has no file-system permission and the core opens no files, so the
 * only way a file reaches Soul is the one the user pointed at.
 */
export function previewSoulImportV1(text: string): Promise<ImportPreview> {
  return invoke<ImportPreview>(COMMANDS.previewSoulImportV1, { text });
}

/** The same for the `result.json` Telegram Desktop's own export produced. */
export function previewTelegram(text: string): Promise<ImportPreview> {
  return invoke<ImportPreview>(COMMANDS.previewTelegram, { text });
}

/**
 * Seal the file into the store, after the user read the counts.
 *
 * The same text goes back: the core re-parses it rather than keeping a staged
 * copy of somebody's export in memory between two clicks, and the same bytes
 * produce the same counts.
 */
export function commitSoulImportV1(text: string): Promise<ImportReceipt> {
  return mutateDraftSource<ImportReceipt>(COMMANDS.commitSoulImportV1, { text });
}

export function commitTelegram(text: string): Promise<ImportReceipt> {
  return mutateDraftSource<ImportReceipt>(COMMANDS.commitTelegram, { text });
}

/** The eleven questions the wizard draws. Answerable with the store shut. */
export function questionnaire(): Promise<readonly Question[]> {
  return invoke<readonly Question[]>(COMMANDS.questionnaire);
}

/**
 * Hand in the questionnaire. AC-03.
 *
 * A blank answer is a skipped question, not a guess: the core drops it, and
 * the axis it would have moved stays `unknown`.
 */
export function answerQuestionnaire(
  answers: readonly GivenAnswer[],
): Promise<IntakeReceipt> {
  return mutateDraftSource<IntakeReceipt>(COMMANDS.answerQuestionnaire, { answers });
}

/** The profile: axes, voice, and the pointers to what the user stated. */
export function profileScreen(): Promise<ProfileScreen> {
  return invoke<ProfileScreen>(COMMANDS.profileScreen);
}

/** The user read an axis and said it is wrong. The core pins it. */
export function correctAxis(axisId: string, position: string): Promise<ProfileScreen> {
  return mutateDraftSource<ProfileScreen>(COMMANDS.correctAxis, { axisId, position });
}

/** The user set one voice field by hand. Inference stops touching it. */
export function setVoice(field: string, option: string): Promise<ProfileScreen> {
  return mutateDraftSource<ProfileScreen>(COMMANDS.setVoice, { field, option });
}

/** Every memory, with the prose still sealed. */
export function memoryList(): Promise<MemoryList> {
  return invoke<MemoryList>(COMMANDS.memoryList);
}

/** One memory, opened. */
export function memoryDetail(memoryId: string): Promise<MemoryDetail> {
  return invoke<MemoryDetail>(COMMANDS.memoryDetail, { memoryId });
}

export function createMemory(memory: NewMemory): Promise<MemoryDetail> {
  return mutateDraftSource<MemoryDetail>(COMMANDS.createMemory, { memory });
}

export function updateMemory(memoryId: string, change: MemoryChange): Promise<MemoryDetail> {
  return mutateDraftSource<MemoryDetail>(COMMANDS.updateMemory, { memoryId, change });
}

/**
 * What forgetting this memory would cost. Nothing is destroyed by asking.
 *
 * The core remembers the answer it gave, and `forgetMemory` refuses a
 * confirmation that does not name it — so a second click cannot destroy
 * something other than what was on the screen the user read.
 */
export function previewForget(memoryId: string): Promise<ForgetPreview> {
  return invoke<ForgetPreview>(COMMANDS.previewForget, { memoryId });
}

/**
 * Destroy the content key behind one memory. Irreversible.
 *
 * This is a forget, which D15 defines as key destruction, and it is the one
 * destructive thing v0.1 does. It writes Soul's own encrypted store and log;
 * it does not execute the file plans that belong to AC-27.
 */
export function forgetMemory(confirmation: ForgetConfirmation): Promise<ForgetReceipt> {
  return mutateDraftSource<ForgetReceipt>(COMMANDS.forgetMemory, { confirmation });
}

/** Only the WAL cleanup. Never reuses a destructive confirmation or writes audit. */
export function retryForgetCleanup(): Promise<ForgetCleanupResult> {
  return invoke<ForgetCleanupResult>(COMMANDS.retryForgetCleanup);
}

/** What the research track would see. On screen only. */
export function researchPreview(): Promise<ResearchPreview> {
  return invoke<ResearchPreview>(COMMANDS.researchPreview);
}

export interface ResearchSavePrepare {
  readonly scope: { readonly from_day: string; readonly to_day: string };
  readonly destination: string;
}
export interface ResearchSavePreview {
  readonly request_id: string;
  readonly destination: string;
  readonly prepared_at_utc: string;
  readonly dataset: {
    readonly scope: ResearchSavePrepare["scope"];
    readonly rows: readonly { readonly event_kind: string; readonly time_bucket_utc: string; readonly aggregate_count: number }[];
    readonly counts: {
      readonly candidate_events: number;
      readonly included_events: number;
      readonly excluded_policy_events: number;
      readonly excluded_unavailable_events: number;
      readonly excluded_time_events: number;
      readonly out_of_range_events: number;
    };
  };
  readonly notice: string;
}
export interface ResearchSaveReceipt {
  readonly request_id: string;
  readonly destination: string;
  readonly row_count: number;
  readonly included_events: number;
  readonly written_to_disk: boolean;
  readonly transaction_closed: boolean;
}
export interface ResearchSaveCancel {
  readonly cancelled: boolean;
  readonly already_published: boolean;
}
// A Session holds one prepared research save. Serialize native preparation
// across route lifetimes so an abandoned older request cannot replace a newer
// preview. Confirm/cancel stay immediate and address their specific request id.
let researchSavePreparation: Promise<void> = Promise.resolve();
export function prepareResearchSave(request: ResearchSavePrepare): Promise<ResearchSavePreview> {
  const captured = { scope: { ...request.scope }, destination: request.destination };
  const result = researchSavePreparation.then(() =>
    invoke<ResearchSavePreview>(COMMANDS.prepareResearchSave, { request: captured }),
  );
  researchSavePreparation = result.then(() => undefined, () => undefined);
  return result;
}
export function confirmResearchSave(requestId: string): Promise<ResearchSaveReceipt> {
  return invoke<ResearchSaveReceipt>(COMMANDS.confirmResearchSave, { requestId });
}
export function cancelResearchSave(requestId: string): Promise<ResearchSaveCancel> {
  return invoke<ResearchSaveCancel>(COMMANDS.cancelResearchSave, { requestId });
}

/** The audit chain, played back and checked by the store. */
export function auditChain(): Promise<AuditChain> {
  return invoke<AuditChain>(COMMANDS.auditChain);
}

/**
 * Whether collection may run, whether it is running, and how much it wrote.
 *
 * Read from the consent ledger and the collector thread, not from a
 * configuration flag: `soulcore::commands::collect` says why, and the short
 * version is that a third copy of the answer is how a user gets shown "off"
 * while something is still writing.
 */
export function collectStatus(): Promise<CollectStatus> {
  return invoke<CollectStatus>(COMMANDS.collectStatus);
}

/**
 * The user said foreground duration may be collected.
 *
 * There is no argument, because there is nothing to configure: what gets
 * collected is fixed by the build. The answer says whether a collector
 * actually started — on a machine with no foreground source the consent is
 * recorded and nothing is watched, and the notice says so.
 */
export function grantCollectConsent(): Promise<CollectStatus> {
  return mutateDraftSource<CollectStatus>(COMMANDS.grantCollectConsent);
}

/** The user took it back. Nothing further is written within a second. */
export function revokeCollectConsent(): Promise<CollectStatus> {
  return mutateDraftSource<CollectStatus>(COMMANDS.revokeCollectConsent);
}
