import type { ImportOperationView, ImportRecoveryState } from "../core";

const KEY = "soul.last-import-operation";
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
function validateReceipt(value: ImportOperationView): void {
  if (value.operation.state !== "committed") return;
  const receipt = value.operation.receipt;
  const fields = ["participants", "messages", "duplicates", "contacts_created", "contacts_matched", "events_written", "evidence_written", "messages_with_injection_markers", "ties_rebuilt"] as const;
  if (receipt === null || receipt === undefined || fields.some((field) => !Number.isSafeInteger(receipt[field]) || receipt[field] < 0)) {
    throw new Error("核心提交回执不完整");
  }
}
function saved(): string | null {
  try { const id = localStorage.getItem(KEY); return id !== null && UUID.test(id) ? id : null; }
  catch { return null; }
}
interface State { readonly busy: boolean; readonly unknown: boolean; readonly operationId: string | null; readonly receipt: ImportOperationView | null }

/** One WebView owner; durable truth is queried from core, never replayed here. */
export class DurableImport {
  private revision = 0;
  readonly generation = (): number => this.revision;
  private state: State = { busy: false, unknown: saved() !== null, operationId: saved(), receipt: null };
  private readonly listeners = new Set<() => void>();
  readonly snapshot = (): State => this.state;
  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener); return () => { this.listeners.delete(listener); };
  };
  private set(next: Partial<State>): void {
    this.state = { ...this.state, ...next }; this.listeners.forEach((listener) => listener());
  }
  remember(id: string): void {
    if (!UUID.test(id)) throw new Error("核心没有返回有效操作编号");
    try { localStorage.setItem(KEY, id); } catch { /* The visible UUID still permits manual lookup. */ }
    this.set({ operationId: id });
  }
  async run<T>(work: () => Promise<T>, target: string | null = this.state.operationId, recoveryDiscard = false): Promise<T> {
    if (this.state.busy || (this.state.unknown && !(recoveryDiscard && target !== null && target === this.state.operationId))) throw new Error("请先核对在途或未知操作");
    this.revision += 1;
    this.set({ busy: true });
    try { return await work(); }
    catch (error: unknown) {
      const code = (error as { reason_code?: unknown } | null)?.reason_code;
      if (typeof code !== "string" || ["IMPORT_OUTCOME_UNKNOWN", "IMPORT_STORE_UNAVAILABLE"].includes(code)) this.set({ unknown: true, operationId: target });
      throw error;
    } finally { this.set({ busy: false }); }
  }
  observed(value: ImportOperationView, generation?: number): void {
    if (generation !== undefined && (generation !== this.revision || this.state.busy)) return;
    validateReceipt(value);
    const target = this.state.operationId;
    if (!this.state.unknown || target === value.operation.operation_id) this.remember(value.operation.operation_id);
    this.set({ receipt: value });
    this.checked([value]);
  }
  /** Only a completed explicit server query can unlock a new, distinct intent. */
  checked(rows: readonly ImportOperationView[], generation = this.revision): void {
    if (generation !== this.revision || this.state.busy) return;
    rows.forEach(validateReceipt);
    const held = rows.filter((row) => row.prepared_here === true && row.operation.state === "reserved");
    if (this.state.unknown && this.state.operationId === null && held.length === 1) this.remember(held[0]!.operation.operation_id);
    if (rows.some((row) => row.operation.operation_id === this.state.operationId &&
      ["committed", "rejected", "discarded", "invalidated", "interrupted"].includes(row.operation.state))) this.set({ unknown: false });
  }
  recovered(value: ImportRecoveryState, generation = this.revision): void {
    if (generation !== this.revision || this.state.busy) return;
    if (this.state.operationId === null && value.can_start_new_intent === true && value.active_worker === false &&
      value.running_operations === 0 && value.held_operation_id === null) this.set({ unknown: false });
  }
}
export const durableImport = new DurableImport();
