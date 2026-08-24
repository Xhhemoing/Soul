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
 */
export function prepareDraft(pasted: string): Promise<E1DraftPlan> {
  return invoke<E1DraftPlan>(COMMANDS.prepareDraft, { pasted });
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
