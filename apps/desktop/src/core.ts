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
 * The types below mirror `crates/soulcore/src/commands/shell.rs`. They are
 * snake_case because that is what crosses the IPC; renaming them here would
 * mean transforming values, and a transform is a place for a bug to live.
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
