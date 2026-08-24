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
   *  nothing until a generation is approved. */
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
  readonly interaction_count: number;
  readonly outgoing_count: number;
  readonly incoming_count: number;
  readonly conversation_count: number;
  readonly active_day_count: number;
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
export interface IntakeReceipt {
  readonly answered: number;
  readonly axes_known: number;
  /** Axes nobody answered for. They stay `unknown`; nothing is guessed. */
  readonly axes_unknown: number;
  readonly voice_fields_user_set: number;
  readonly stated_entries: number;
  readonly profile_is_empty: boolean;
  readonly evidence_ids: readonly string[];
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
  /** Whether the receipt charges what the preview quoted. */
  readonly matched_preview: boolean;
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
  configSnapshot: "config_snapshot",
  sessionStatus: "session_status",
  completeWizard: "complete_wizard",
  cloudToggle: "cloud_toggle",
  filesView: "files_view",
  authorizeDirectory: "authorize_directory",
  previewPlan: "preview_plan",
  peopleGraph: "people_graph",
  personSummary: "person_summary",
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
  researchPreview: "research_preview",
  auditChain: "audit_chain",
  collectStatus: "collect_status",
  grantCollectConsent: "grant_collect_consent",
  revokeCollectConsent: "revoke_collect_consent",
} as const;

export function configSnapshot(): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.configSnapshot);
}

/** Whether the wizard is behind us, and whether there is a database to read. */
export function sessionStatus(): Promise<SessionStatus> {
  return invoke<SessionStatus>(COMMANDS.sessionStatus);
}

/** Finish the first-run wizard. The core refuses to return an open config. */
export function completeWizard(answers: WizardAnswers): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.completeWizard, { answers });
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
  return invoke<FilesView>(COMMANDS.authorizeDirectory, { path });
}

/**
 * Read one authorized directory and describe what tidying it would mean.
 *
 * There is no companion call that carries the plan out, in this file or in the
 * Rust one. v0.1 promises a preview; v0.1.1 owns the write half.
 */
export function previewPlan(path: string): Promise<PlanPreview> {
  return invoke<PlanPreview>(COMMANDS.previewPlan, { path });
}

/** The people graph, with every tie's evidence already resolved. */
export function peopleGraph(): Promise<PeopleGraph> {
  return invoke<PeopleGraph>(COMMANDS.peopleGraph);
}

/** Everything Soul will say about one person, and what each line rests on. */
export function personSummary(contactId: string): Promise<PersonSummary> {
  return invoke<PersonSummary>(COMMANDS.personSummary, { contactId });
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
  return invoke<Draft>(COMMANDS.draftReply, { pasted });
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
  return invoke<E1DraftPlan>(COMMANDS.prepareDraft, { pasted, includeOriginal });
}

/**
 * Step two: the user read the counts and approved that exact preparation.
 *
 * The approval is the value the core handed over, echoed back unchanged. A
 * shell that assembled one itself would be approving something nobody read,
 * and `soulcore` refuses an approval whose two halves do not match.
 */
export function generateDraft(approval: Approval): Promise<Draft> {
  return invoke<Draft>(COMMANDS.generateDraft, { approval });
}

/** The user read the plan and said no. Returns whether there was one. */
export function discardDraft(): Promise<boolean> {
  return invoke<boolean>(COMMANDS.discardDraft);
}

/**
 * The user's own OpenAI-compatible endpoint, for this run.
 *
 * The address goes one way. What comes back is the snapshot, which says
 * whether there is an endpoint and never what it is — so this shell cannot
 * redisplay it later, and neither can the next launch: the core writes it to
 * no file. Nothing is contacted here; the core parses the address and points
 * its egress guard at it, and the first request waits for an approval on the
 * drafting page.
 */
export function setUserEndpoint(url: string): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.setUserEndpoint, { url });
}

/** The user took the address away. The core goes back to reaching nothing. */
export function clearUserEndpoint(): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.clearUserEndpoint);
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
  return invoke<ImportReceipt>(COMMANDS.commitSoulImportV1, { text });
}

export function commitTelegram(text: string): Promise<ImportReceipt> {
  return invoke<ImportReceipt>(COMMANDS.commitTelegram, { text });
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
  return invoke<IntakeReceipt>(COMMANDS.answerQuestionnaire, { answers });
}

/** The profile: axes, voice, and the pointers to what the user stated. */
export function profileScreen(): Promise<ProfileScreen> {
  return invoke<ProfileScreen>(COMMANDS.profileScreen);
}

/** The user read an axis and said it is wrong. The core pins it. */
export function correctAxis(axisId: string, position: string): Promise<ProfileScreen> {
  return invoke<ProfileScreen>(COMMANDS.correctAxis, { axisId, position });
}

/** The user set one voice field by hand. Inference stops touching it. */
export function setVoice(field: string, option: string): Promise<ProfileScreen> {
  return invoke<ProfileScreen>(COMMANDS.setVoice, { field, option });
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
  return invoke<MemoryDetail>(COMMANDS.createMemory, { memory });
}

export function updateMemory(memoryId: string, change: MemoryChange): Promise<MemoryDetail> {
  return invoke<MemoryDetail>(COMMANDS.updateMemory, { memoryId, change });
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
 * destructive thing v0.1 does. It writes no file: AC-27 is about the file
 * plan, and there is still no command that carries one out.
 */
export function forgetMemory(confirmation: ForgetConfirmation): Promise<ForgetReceipt> {
  return invoke<ForgetReceipt>(COMMANDS.forgetMemory, { confirmation });
}

/** What the research track would see. On screen only. */
export function researchPreview(): Promise<ResearchPreview> {
  return invoke<ResearchPreview>(COMMANDS.researchPreview);
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
  return invoke<CollectStatus>(COMMANDS.grantCollectConsent);
}

/** The user took it back. Nothing further is written within a second. */
export function revokeCollectConsent(): Promise<CollectStatus> {
  return invoke<CollectStatus>(COMMANDS.revokeCollectConsent);
}
