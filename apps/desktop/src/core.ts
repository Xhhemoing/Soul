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
}

export interface WizardAnswers {
  readonly acknowledged_defaults_are_off: boolean;
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

/** What the drafting screen says before there is a draft on it. */
export interface DraftNotices {
  readonly not_sent: string;
  /** Always false. */
  readonly can_send: boolean;
}

/** A refused command: a code and a sentence, never an endpoint's own words. */
export interface DraftRefusalView {
  readonly reason_code: string;
  readonly explanation: string;
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
  draftReply: "draft_reply",
  draftNotices: "draft_notices",
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
