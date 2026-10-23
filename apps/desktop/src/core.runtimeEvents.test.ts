import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as core from "./core";
import { anE1Plan } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

let deliver: (event: { payload: unknown }) => void;
let stop: (() => void) | undefined;
const unlisten = vi.fn();
const epoch = "10000000-0000-4000-8000-000000000001";
const operation = "20000000-0000-4000-8000-000000000001";
function event(sequence: number, phase: "started" | "settled") {
  return { epoch, sequence, operation_id: operation, caller: { transport: "http", webview_label: null },
    correlation_id: null, kind: "memory_edit", phase, outcome: phase === "settled" ? "refused" : null };
}
beforeEach(() => {
  vi.useFakeTimers();
  mockWindows("main");
  vi.mocked(invoke).mockReset(); vi.mocked(listen).mockReset(); unlisten.mockReset();
  vi.mocked(listen).mockImplementation(async (_name, handler) => {
    deliver = handler as typeof deliver;
    return unlisten;
  });
  vi.mocked(invoke).mockImplementation((command) => Promise.resolve(command === "runtime_event_snapshot"
    ? { epoch, sequence: 0, active_operations: [] } : anE1Plan()) as Promise<never>);
});
afterEach(() => { stop?.(); stop = undefined; vi.useRealTimers(); });
async function start() {
  stop = core.startRuntimeEvents();
  await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
}
describe("runtime changes in the real core wrapper", () => {
  it("does not admit new plaintext reads while the native subscription is unknown", async () => {
    vi.mocked(listen).mockRejectedValueOnce(new Error("synthetic listen failure"));
    await start();
    await expect(core.memoryDetail("synthetic-memory")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    expect(vi.mocked(invoke).mock.calls.some(([command]) => command === "memory_detail")).toBe(false);
  });
  it("does not admit plaintext reads in the middle of a known external write", async () => {
    await start();
    deliver({ payload: event(1, "started") });
    await expect(core.memoryDetail("synthetic-memory")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    expect(vi.mocked(invoke).mock.calls.some(([command]) => command === "memory_detail")).toBe(false);
  });
  it.each([
    { transport: "desktop", webview_label: "another-window" },
    { transport: "http", webview_label: null },
  ])("the same correlation UUID from foreign $transport events is never this WebView's echo", async (caller) => {
    await start(); await core.updateMemory("synthetic-memory", { summary: "synthetic" });
    const { correlationId } = vi.mocked(invoke).mock.calls.at(-1)![1] as { correlationId: string };
    const source = core.getDraftSourceRevision();
    deliver({ payload: { ...event(1, "started"), caller, correlation_id: correlationId } });
    expect(core.getDraftSourceRevision()).not.toBe(source);
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    deliver({ payload: { ...event(2, "settled"), caller, correlation_id: correlationId } });
  });
  it.each([
    { transport: "desktop", webview_label: "another-window" },
    { transport: "http", webview_label: null },
  ])("foreign $transport snapshot work stays pending even when it reuses our correlation UUID", async (caller) => {
    await start(); await core.updateMemory("synthetic-memory", { summary: "synthetic" });
    const { correlationId } = vi.mocked(invoke).mock.calls.at(-1)![1] as { correlationId: string };
    vi.mocked(invoke).mockResolvedValueOnce({ epoch, sequence: 1, active_operations: [
      { operation_id: operation, caller, correlation_id: correlationId, kind: "memory_edit" },
    ] });
    window.dispatchEvent(new Event("focus"));
    await Promise.resolve(); await Promise.resolve();
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  });
  it("startup invalidates synchronously and stays unknown until both subscription and snapshot complete", async () => {
    let registered!: (value: () => void) => void;
    vi.mocked(listen).mockImplementationOnce(() => new Promise((yes) => { registered = yes; }));
    const source = core.getDraftSourceRevision();
    stop = core.startRuntimeEvents();
    expect(core.getDraftSourceRevision()).not.toBe(source);
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    registered(unlisten); await Promise.resolve(); await Promise.resolve();
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });
  it("the first fresh snapshot refreshes after a whole HTTP mutation was missed during delayed registration", async () => {
    let registered!: (value: () => void) => void;
    vi.mocked(listen).mockImplementationOnce(() => new Promise((yes) => { registered = yes; }));
    stop = core.startRuntimeEvents();
    const sourceBeforeSnapshot = core.getExternalSourceRevision();
    vi.mocked(invoke).mockResolvedValueOnce({ epoch, sequence: 2, active_operations: [] });
    registered(unlisten); await Promise.resolve(); await Promise.resolve();
    expect(core.getExternalSourceRevision()).not.toBe(sourceBeforeSnapshot);
    expect(core.hasPendingSourceChanges()).toBe(false);
  });
  it("entering startup reconciliation rejects a plaintext read admitted before the listener existed", async () => {
    let resolve!: (value: unknown) => void;
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((yes) => { resolve = yes; }));
    const old = core.memoryDetail("synthetic-memory");
    const rejected = expect(old).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    let registered!: (value: () => void) => void;
    vi.mocked(listen).mockImplementationOnce(() => new Promise((yes) => { registered = yes; }));
    stop = core.startRuntimeEvents();
    resolve({ summary: "synthetic old plaintext" }); await rejected;
    registered(unlisten); await Promise.resolve(); await Promise.resolve();
    expect(core.hasPendingSourceChanges()).toBe(false);
  });
  it("a rejected listener stays fail-closed while preserving local mutation receipts and original errors", async () => {
    vi.mocked(listen).mockRejectedValueOnce(new Error("synthetic listen failure"));
    await start();
    expect(core.hasPendingSourceChanges()).toBe(true);
    expect(core.getRuntimeSyncFailure()).not.toBeNull();
    expect(core.getRuntimeSyncFailure()).not.toContain("synthetic listen failure");
    vi.mocked(invoke).mockResolvedValueOnce({ committed: true });
    await expect(core.updateMemory("synthetic-memory", { summary: "synthetic" })).resolves.toEqual({ committed: true });
    const original = new Error("synthetic original local failure");
    vi.mocked(invoke).mockRejectedValueOnce(original);
    await expect(core.clearUserEndpoint()).rejects.toBe(original);
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  });
  it("failed listen retries within a bounded delay and only a fresh subscribed snapshot reopens reads", async () => {
    vi.mocked(listen).mockRejectedValueOnce(new Error("synthetic listen failure"));
    await start();
    let snapshot!: (value: unknown) => void;
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((yes) => { snapshot = yes; }));
    await vi.advanceTimersByTimeAsync(1_000);
    expect(listen).toHaveBeenCalledTimes(2);
    expect(core.hasPendingSourceChanges()).toBe(true);
    snapshot({ epoch, sequence: 2, active_operations: [] });
    await Promise.resolve(); await Promise.resolve();
    expect(core.hasPendingSourceChanges()).toBe(false);
    expect(core.getRuntimeSyncFailure()).toBeNull();
    deliver({ payload: event(3, "started") });
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  });
  it("cleanup cancels a failed listener's retry timer", async () => {
    vi.mocked(listen).mockRejectedValueOnce(new Error("synthetic listen failure"));
    await start(); stop?.(); stop = undefined;
    await vi.advanceTimersByTimeAsync(30_000);
    expect(listen).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalled();
  });
  it("listens before reconciling; external start rejects old reads and duplicate packets do not unlock it", async () => {
    await start();
    expect(listen).toHaveBeenCalledWith("soul-runtime-change", expect.any(Function));
    expect(vi.mocked(listen).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(invoke).mock.invocationCallOrder[0]!);
    let resolve!: (value: core.E1DraftPlan) => void;
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((yes) => { resolve = yes; }));
    const old = core.prepareDraft("synthetic");
    const rejected = expect(old).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    deliver({ payload: event(1, "started") });
    deliver({ payload: event(1, "started") });
    resolve(anE1Plan()); await rejected;
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    deliver({ payload: event(2, "settled") });
    deliver({ payload: event(2, "settled") });
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });
  it("focus recovery reconciles a missed final event and cleanup releases the native listener", async () => {
    await start(); deliver({ payload: event(1, "started") });
    vi.mocked(invoke).mockResolvedValueOnce({ epoch, sequence: 2, active_operations: [] });
    window.dispatchEvent(new Event("focus"));
    await Promise.resolve(); await Promise.resolve();
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
    stop?.(); stop = undefined;
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
  it("late listener registration after cleanup is removed without snapshot work", async () => {
    let registered!: (value: () => void) => void;
    vi.mocked(listen).mockImplementationOnce(() => new Promise((yes) => { registered = yes; }));
    stop = core.startRuntimeEvents(); stop(); stop = undefined;
    registered(unlisten); await Promise.resolve(); await Promise.resolve();
    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(invoke).not.toHaveBeenCalled();
  });
  it("deduplicates only this wrapper's UUID echoes, including delivery after its successful receipt", async () => {
    await start();
    const listener = vi.fn(); const unsubscribe = core.subscribeDraftSource(listener);
    try {
      const write = core.updateMemory("synthetic-memory", { summary: "synthetic" });
      const args = vi.mocked(invoke).mock.calls.at(-1)![1] as { correlationId: string };
      expect(args.correlationId).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
      deliver({ payload: { ...event(1, "started"), correlation_id: args.correlationId,
        caller: { transport: "desktop", webview_label: "main" } } });
      await write;
      deliver({ payload: { ...event(2, "settled"), correlation_id: args.correlationId,
        caller: { transport: "desktop", webview_label: "main" } } });
      expect(listener).toHaveBeenCalledTimes(3); // initial + JS start/finally
      deliver({ payload: { ...event(3, "started"), correlation_id: crypto.randomUUID(),
        caller: { transport: "desktop", webview_label: "another-window" } } });
      expect(listener).toHaveBeenCalledTimes(4);
      await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
      deliver({ payload: event(4, "settled") });
    } finally { unsubscribe(); }
  });
  it("a sequence gap blocks draft admission until snapshot recovery, even when that first snapshot fails", async () => {
    await start();
    vi.mocked(invoke).mockRejectedValueOnce(new Error("synthetic snapshot failure"));
    deliver({ payload: event(3, "settled") });
    await Promise.resolve(); await Promise.resolve();
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    // Event failure never replaces the actual local mutation's outcome.
    vi.mocked(invoke).mockRejectedValueOnce(new Error("original local refusal"));
    await expect(core.clearUserEndpoint()).rejects.toThrow("original local refusal");
    vi.mocked(invoke).mockResolvedValueOnce({ epoch, sequence: 3, active_operations: [] });
    await vi.advanceTimersByTimeAsync(10_000);
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });
  it("a new epoch reconciles pending work and delayed packets from a retired epoch cannot revive it", async () => {
    await start();
    deliver({ payload: event(1, "started") });
    const nextEpoch = "10000000-0000-4000-8000-000000000002";
    vi.mocked(invoke).mockResolvedValueOnce({ epoch: nextEpoch, sequence: 1, active_operations: [] });
    deliver({ payload: { ...event(1, "settled"), epoch: nextEpoch } });
    await Promise.resolve(); await Promise.resolve();
    const source = core.getDraftSourceRevision();
    deliver({ payload: event(99, "started") });
    expect(core.getDraftSourceRevision()).toBe(source);
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });
  it("a lightweight periodic snapshot repairs a dropped final event without focus", async () => {
    await start(); deliver({ payload: event(1, "started") });
    vi.mocked(invoke).mockResolvedValueOnce({ epoch, sequence: 2, active_operations: [] });
    await vi.advanceTimersByTimeAsync(10_000);
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });
  it("external changes reject late ordinary plaintext reads as well as drafts", async () => {
    await start();
    let resolve!: (value: unknown) => void;
    vi.mocked(invoke).mockImplementationOnce(() => new Promise((yes) => { resolve = yes; }));
    const read = core.memoryDetail("synthetic-memory");
    const rejected = expect(read).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    deliver({ payload: event(1, "started") });
    resolve({ summary: "synthetic old plaintext" }); await rejected;
    deliver({ payload: event(2, "settled") });
  });
});
