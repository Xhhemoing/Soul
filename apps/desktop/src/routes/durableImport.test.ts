import { beforeEach, describe, expect, it, vi } from "vitest";
import { anImportOperation, anImportOperationView, IMPORT_OPERATION_ID } from "../test/fakeCore";
import { DurableImport } from "./durableImport";

function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
beforeEach(() => { localStorage.clear(); });

describe("持久导入意图的WebView lane", () => {
  it("调用factory之前同步占槽，拒绝重入且不提前释放物理任务", async () => {
    const lane = new DurableImport(); const pending = deferred<void>(); const duplicate = vi.fn(async () => {});
    let rejected: Promise<void> | undefined;
    const active = lane.run(() => { rejected = expect(lane.run(duplicate)).rejects.toThrow(); return pending.promise; });
    expect(lane.snapshot().busy).toBe(true); expect(duplicate).not.toHaveBeenCalled();
    pending.resolve(); await active; await rejected; expect(lane.snapshot().busy).toBe(false);
  });
  it("观察者卸载不释放任务，新观察者能读到最终计数回执", async () => {
    const lane = new DurableImport(); const pending = deferred<void>(); const old = vi.fn();
    const stop = lane.subscribe(old); const work = lane.run(() => pending.promise); stop();
    const next = vi.fn(); lane.subscribe(next);
    const committed = anImportOperationView({ operation: anImportOperation({ state: "committed", receipt: anImportOperation().preview }) });
    lane.observed(committed); pending.resolve(); await work;
    expect(lane.snapshot().receipt).toEqual(committed); expect(lane.snapshot().busy).toBe(false); expect(next).toHaveBeenCalled();
  });
  it("未知结果阻止所有新意图，显式查最近记录才可解除；Running仍阻止", async () => {
    const lane = new DurableImport(); const privateError = new Error("PRIVATE EXPORT BODY / credential");
    lane.remember(IMPORT_OPERATION_ID);
    await expect(lane.run(() => Promise.reject(privateError))).rejects.toEqual(privateError);
    expect(lane.snapshot().unknown).toBe(true); const next = vi.fn(async () => {});
    await expect(lane.run(next)).rejects.toThrow(); expect(next).not.toHaveBeenCalled();
    lane.checked([anImportOperationView({ operation: anImportOperation({ state: "running" }) })]);
    expect(lane.snapshot().unknown).toBe(true);
    lane.checked([anImportOperationView({ operation: anImportOperation({ state: "interrupted" }) })]);
    expect(lane.snapshot().unknown).toBe(false); await lane.run(next); expect(next).toHaveBeenCalledTimes(1);
    expect(JSON.stringify(lane.snapshot())).not.toMatch(/PRIVATE|credential/);
  });
  it("有结构化确定拒绝可新建意图，unknown/store-unavailable拒绝保持未知", async () => {
    for (const code of ["IMPORT_OUTCOME_UNKNOWN", "IMPORT_STORE_UNAVAILABLE"]) {
      const lane = new DurableImport(); await expect(lane.run(() => Promise.reject({ reason_code: code }))).rejects.toEqual({ reason_code: code });
      expect(lane.snapshot().unknown).toBe(true);
    }
    const lane = new DurableImport(); await expect(lane.run(() => Promise.reject({ reason_code: "IMPORT_PREVIEW_MISMATCH" }))).rejects.toEqual({ reason_code: "IMPORT_PREVIEW_MISMATCH" });
    expect(lane.snapshot().unknown).toBe(false);
  });
  it("只持久化一个合成UUID，刷新不保存原文、批准或重发闭包", () => {
    const lane = new DurableImport(); lane.remember(IMPORT_OPERATION_ID);
    const fresh = new DurableImport(); expect(fresh.snapshot().operationId).toBe(IMPORT_OPERATION_ID);
    expect(fresh.snapshot().receipt).toBeNull(); expect(fresh.snapshot().busy).toBe(false); expect(fresh.snapshot().unknown).toBe(true);
    expect([...Object.values(localStorage)].join()).toBe(IMPORT_OPERATION_ID);
    expect(() => lane.remember("PRIVATE RAW INPUT")).toThrow();
  });
  it("损坏的成功回执不被接受为成功", () => {
    const lane = new DurableImport();
    expect(() => lane.observed(anImportOperationView({ operation: anImportOperation({ state: "committed", receipt: null }) }))).toThrow();
    expect(lane.snapshot().receipt).toBeNull();
  });
  it("缺失、负数或非整数计数不能解除未知操作或显示为成功", async () => {
    const lane = new DurableImport(); lane.remember(IMPORT_OPERATION_ID);
    await expect(lane.run(() => Promise.reject(new Error("lost confirm")))).rejects.toThrow();
    for (const invalid of [undefined, -1, 0.5, Number.NaN, Number.MAX_SAFE_INTEGER + 1]) {
      const value = anImportOperationView({ operation: anImportOperation({ state: "committed", receipt: { ...anImportOperation().preview, events_written: invalid as number } }) });
      expect(() => lane.checked([value])).toThrow();
      expect(() => lane.observed(value)).toThrow();
      expect(lane.snapshot().unknown).toBe(true); expect(lane.snapshot().receipt).toBeNull();
    }
  });
  it("历史Y回执或列表缺X都不能覆盖或解除未知X，只有X终态可以", async () => {
    const lane = new DurableImport(); lane.remember(IMPORT_OPERATION_ID);
    await expect(lane.run(() => Promise.reject(new Error("synthetic lost confirm")))).rejects.toThrow();
    const history = anImportOperationView({ operation: anImportOperation({ operation_id: "0192f000-0000-7000-8000-00000000dd01", state: "committed", receipt: anImportOperation().preview }) });
    lane.observed(history); lane.checked([]); lane.checked([history]);
    expect(lane.snapshot().operationId).toBe(IMPORT_OPERATION_ID); expect(lane.snapshot().unknown).toBe(true);
    lane.recovered({ active_worker: false, reserved_operations: 0, running_operations: 0, held_operation_id: null, can_start_new_intent: true });
    expect(lane.snapshot().unknown).toBe(true);
    lane.checked([anImportOperationView({ operation: anImportOperation({ state: "interrupted" }) })]);
    expect(lane.snapshot().unknown).toBe(false);
  });
  it("无UUID准备失联只能用无worker无Running无held的服务端证明恢复", async () => {
    const lane = new DurableImport(); await expect(lane.run(() => Promise.reject(new Error("lost prepare")), null)).rejects.toThrow();
    lane.checked([]); expect(lane.snapshot().unknown).toBe(true);
    const proof = { active_worker: false, reserved_operations: 0, running_operations: 0, held_operation_id: null, can_start_new_intent: true } as const;
    for (const unsafe of [{ ...proof, active_worker: true }, { ...proof, running_operations: null }, { ...proof, running_operations: 1 },
      { ...proof, held_operation_id: IMPORT_OPERATION_ID }, { ...proof, can_start_new_intent: false }]) {
      lane.recovered(unsafe); expect(lane.snapshot().unknown).toBe(true);
    }
    lane.recovered(proof); expect(lane.snapshot().unknown).toBe(false);
  });
  it("旧generation的恢复证明和回执不能解除随后新run的未知结果", async () => {
    const lane = new DurableImport(); const old = lane.generation();
    await expect(lane.run(() => Promise.reject(new Error("new intent lost")), null)).rejects.toThrow();
    const proof = { active_worker: false, reserved_operations: 0, running_operations: 0, held_operation_id: null, can_start_new_intent: true };
    lane.recovered(proof, old);
    lane.checked([anImportOperationView({ operation: anImportOperation({ state: "interrupted" }) })], old);
    lane.observed(anImportOperationView({ operation: anImportOperation({ state: "committed", receipt: anImportOperation().preview }) }), old);
    expect(lane.snapshot().unknown).toBe(true); expect(lane.snapshot().operationId).toBeNull(); expect(lane.snapshot().receipt).toBeNull();
    lane.recovered(proof, lane.generation()); expect(lane.snapshot().unknown).toBe(false);
  });
});
