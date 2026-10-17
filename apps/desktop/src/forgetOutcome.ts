/** Presentation of the core's observations, not a second cleanup algorithm. */
export interface WalCheckpoint {
  readonly busy: number;
  readonly log_frames: number;
  readonly checkpointed_frames: number;
}

export type ForgetCleanup =
  | { readonly state: "complete" }
  | { readonly state: "pending"; readonly checkpoint: WalCheckpoint | null };

export type ForgetAudit = "recorded" | "unconfirmed";

export type ForgetStatus =
  | "complete"
  | "cleanup_pending"
  | "audit_unconfirmed"
  | "cleanup_and_audit_unconfirmed"
  | "preview_mismatch"
  | "unconfirmed";

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

/** Missing or unfamiliar observations never become a successful cleanup. */
export function cleanupStatus(value: unknown): "complete" | "pending" | "unconfirmed" {
  const state = record(value)?.state;
  return state === "complete" || state === "pending" ? state : "unconfirmed";
}

/**
 * An older or malformed receipt cannot silently acquire a success label.
 * Use the state returned by the core; never reinterpret checkpoint counts.
 */
export function forgetStatus(value: unknown): ForgetStatus {
  const receipt = record(value);
  if (
    receipt === null ||
    receipt.logical_committed !== true ||
    typeof receipt.matched_preview !== "boolean"
  ) {
    return "unconfirmed";
  }
  const cleanup = cleanupStatus(receipt.cleanup);
  const audit = receipt.audit;
  if (cleanup === "unconfirmed" || (audit !== "recorded" && audit !== "unconfirmed")) {
    return "unconfirmed";
  }
  if (cleanup === "pending") {
    return audit === "recorded" ? "cleanup_pending" : "cleanup_and_audit_unconfirmed";
  }
  if (audit === "unconfirmed") return "audit_unconfirmed";
  return receipt.matched_preview ? "complete" : "preview_mismatch";
}
