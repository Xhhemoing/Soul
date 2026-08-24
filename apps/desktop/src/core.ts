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
 * The types below mirror `crates/soulcore/src/commands/{shell,draft,fileplan}.rs`.
 * They are snake_case because that is what crosses the IPC; renaming them here
 * would mean transforming values, and a transform is a place for a bug to live.
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
  /** Whether a platform key store holds the key. False in every v0.1 build. */
  readonly kek_protected: boolean;
  /** The core's own sentence about the database key. Render it as it arrives. */
  readonly key_protection: string;
}

export interface WizardAnswers {
  readonly acknowledged_defaults_are_off: boolean;
}

/** Why the core would not authorise a directory. The message is its words. */
export interface RootRefused {
  readonly reason:
    | "empty"
    | "not_found"
    | "not_a_directory"
    | "unreadable"
    | "already_authorized";
  readonly message: string;
}

/** Why a view would not run. The message is the core's, rendered as it arrives. */
export interface ViewRefused {
  readonly reason: "no_store_opened" | "empty_paste" | "refused";
  readonly code: string | null;
  readonly message: string;
}

/**
 * One draft, as the core shaped it.
 *
 * `never_sent` is the literal `true`: there is no code path that sends one,
 * and a type that accepted `false` would let the screen claim otherwise.
 */
export interface DraftView {
  readonly text: string;
  readonly route: string;
  readonly route_label: string;
  readonly turns: number;
  readonly third_party_turns: number;
  readonly placeheld_turns: number;
  readonly carries_exempted_original: boolean;
  readonly never_sent: true;
  readonly notice: string;
}

/** One suggestion in a file plan. The page renders `action_label` as given. */
export interface FilePlanEntryView {
  readonly source_rel: string;
  readonly action: string;
  readonly action_label: string;
  readonly target_rel: string | null;
}

/**
 * A scan and the plan it produced.
 *
 * `written_to_disk` is the literal `false`: v0.1 has no execute path, and a
 * type that accepted `true` would let the screen claim a write that cannot
 * have happened.
 */
export interface FilePlanView {
  readonly scan_id: string;
  readonly file_count: number;
  readonly dir_count: number;
  readonly skipped_escaping_links: number;
  readonly entry_count: number;
  readonly group_count: number;
  readonly move_count: number;
  readonly rename_count: number;
  readonly entries: readonly FilePlanEntryView[];
  readonly written_to_disk: false;
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
  completeWizard: "complete_wizard",
  cloudToggle: "cloud_toggle",
  authorizeRoot: "authorize_root",
  authorizedRoots: "authorized_roots",
  draftView: "draft_view",
  fileplanView: "fileplan_view",
} as const;

export function configSnapshot(): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.configSnapshot);
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

/**
 * Ask the core to authorise a directory for read-only scanning.
 *
 * The string goes over as typed. Whether it exists, whether it is a directory,
 * what it resolves to and whether it is already on the list are four questions
 * this file must not answer: they are decisions, they touch the filesystem,
 * and the WebView is the one place in the product that cannot be tested
 * against a real disk.
 */
export function authorizeRoot(path: string): Promise<ConfigSnapshot> {
  return invoke<ConfigSnapshot>(COMMANDS.authorizeRoot, { path });
}

/** The directories authorised in this session, as the core resolved them. */
export function authorizedRoots(): Promise<readonly string[]> {
  return invoke<readonly string[]>(COMMANDS.authorizedRoots);
}

/**
 * Ask the core to draft a reply from what was pasted.
 *
 * The strings go over as typed. Empty, whitespace-only, and everything that
 * happens after that are questions this file must not answer: they are
 * decisions, they touch the store, and the WebView cannot be tested against
 * a real one.
 */
export function draftView(pasted: readonly string[]): Promise<DraftView> {
  return invoke<DraftView>(COMMANDS.draftView, { pasted });
}

/**
 * Ask the core to scan one directory and show the plan, without carrying it
 * out. The path goes over as typed; whether it is authorised is the core's.
 */
export function fileplanView(target: string): Promise<FilePlanView> {
  return invoke<FilePlanView>(COMMANDS.fileplanView, { target });
}

/**
 * The core's own words for a refusal.
 *
 * Not a translation table: a refusal that reached the screen as
 * `[object Object]` would be the shell losing the one sentence the core wrote
 * for the user, and inventing a replacement here is how "the path is a file"
 * becomes "无效路径".
 */
export function refusalText(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { readonly message: unknown }).message);
  }
  return String(error);
}
