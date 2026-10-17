import type { ImportReceipt, Refusal } from "../core";
import type { ImportFormat } from "./importRequests";

export type ImportDeliveryState =
  | { readonly status: "ready"; readonly receipt: ImportReceipt | null }
  | { readonly status: "pending" | "unknown"; readonly receipt: null };

/** Local admission failure, never a core refusal or evidence of rollback. */
export class ImportCommitBlockedError extends Error {
  constructor() { super("另一份导入仍在处理，或上一次导入的结果尚未确认。"); }
}

/** Does not retain or display the transport's possibly sensitive error text. */
export class ImportCommitUnknownError extends Error {
  constructor() { super("没有收到可确认的导入结果，文件可能已经写入。请先核对记录。"); }
}

function isRefusal(value: unknown): value is Refusal {
  if (typeof value !== "object" || value === null) return false;
  const row = value as Partial<Refusal>;
  return typeof row.reason_code === "string" && typeof row.explanation === "string";
}

function receiptOf(value: unknown, format: ImportFormat): ImportReceipt | null {
  if (typeof value !== "object" || value === null) return null;
  const row = value as Partial<ImportReceipt>;
  const counts = [row.contacts_created, row.contacts_matched, row.events_written,
    row.evidence_written, row.messages_with_injection_markers, row.ties_rebuilt];
  if (row.source !== format || typeof row.notice !== "string"
    || !counts.every((count) => Number.isSafeInteger(count) && (count as number) >= 0)) return null;
  // Copy just the acknowledged fields; never retain arbitrary response metadata.
  return Object.freeze({
    source: row.source, notice: row.notice,
    contacts_created: row.contacts_created!, contacts_matched: row.contacts_matched!,
    events_written: row.events_written!, evidence_written: row.evidence_written!,
    messages_with_injection_markers: row.messages_with_injection_markers!, ties_rebuilt: row.ties_rebuilt!,
  });
}

/**
 * One WebView's Import route, across mounts. No file identity, text, persistence,
 * cancellation, retry or message deduplication. Only the running factory owns
 * the pending slot; a page going away cannot release it.
 */
export class ImportDelivery {
  private state: ImportDeliveryState = { status: "ready", receipt: null };
  private readonly listeners = new Set<(state: ImportDeliveryState) => void>();

  snapshot(): ImportDeliveryState { return this.state; }

  subscribe(listener: (state: ImportDeliveryState) => void): () => void {
    this.listeners.add(listener);
    this.notify(listener);
    return () => { this.listeners.delete(listener); };
  }

  private notify(listener: (state: ImportDeliveryState) => void): void {
    // Observers cannot turn an acknowledged commit into unknown or strand work.
    try { listener(this.state); } catch { /* subscriber owns its rendering failure */ }
  }

  private publish(state: ImportDeliveryState): void {
    this.state = Object.freeze(state);
    for (const listener of [...this.listeners]) this.notify(listener);
  }

  async run(format: ImportFormat, commit: () => Promise<ImportReceipt>): Promise<ImportReceipt> {
    if (this.state.status !== "ready") throw new ImportCommitBlockedError();
    this.publish({ status: "pending", receipt: null });
    let value: unknown;
    try {
      value = await commit();
    } catch (error: unknown) {
      if (isRefusal(error)) {
        this.publish({ status: "ready", receipt: null });
        throw error;
      }
      this.publish({ status: "unknown", receipt: null });
      throw new ImportCommitUnknownError();
    }
    const receipt = receiptOf(value, format);
    if (receipt === null) {
      this.publish({ status: "unknown", receipt: null });
      throw new ImportCommitUnknownError();
    }
    this.publish({ status: "ready", receipt });
    return receipt;
  }
}

export const importDelivery = new ImportDelivery();
