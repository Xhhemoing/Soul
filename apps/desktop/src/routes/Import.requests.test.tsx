import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { aPreparedImport, anImportOperation, anImportOperationView, IMPORT_OPERATION_ID, installFakeCore } from "../test/fakeCore";
import { Import } from "./Import";
import { DurableImport } from "./durableImport";

const TEXT = "SYNTHETIC IMMUTABLE BODY";
function file(): File { const value = new File([TEXT], "local.jsonl"); Object.defineProperty(value, "text", { value: vi.fn().mockResolvedValue(TEXT) }); return value; }
async function prepare(): Promise<void> { fireEvent.change(screen.getByLabelText("选择导出文件"), { target: { files: [file()] } }); await screen.findByRole("heading", { name: "确认这份不可变导入准备" }); }
beforeEach(() => { localStorage.clear(); });

describe("未知提交明确核对与不重发", () => {
  it("confirm未知结果后锁住新导入，显式查询单编号不会自动重发", async () => {
    const core = installFakeCore({ preparingImport: () => aPreparedImport(), confirmingImport: () => { throw new Error("synthetic lost IPC"); } });
    const user = userEvent.setup(); const lane = new DurableImport(); render(<Import lane={lane} />); await prepare();
    await user.click(screen.getByRole("button", { name: "确认导入这份准备" }));
    expect(await screen.findByText(/上次结果未知，新的导入暂停/)).toBeVisible();
    expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "查询这个编号" }));
    await waitFor(() => expect(core.callsTo("import_operation")).toHaveLength(1));
    expect(core.callsTo("import_operation")[0]?.payload).toEqual({ operationId: IMPORT_OPERATION_ID });
    expect(core.callsTo("confirm_import")).toHaveLength(1); expect(core.callsTo("prepare_import")).toHaveLength(1);
    expect(lane.snapshot().unknown).toBe(true);
  });
  it("核对最近记录解除未知也不自动恢复旧批准", async () => {
    const core = installFakeCore({ preparingImport: () => aPreparedImport(), confirmingImport: () => { throw { reason_code: "IMPORT_OUTCOME_UNKNOWN", explanation: "合成未知" }; },
      recentImports: () => [anImportOperationView({ operation: anImportOperation({ state: "interrupted", error_code: "INTERRUPTED" }), prepared_here: false })] });
    const user = userEvent.setup(); const lane = new DurableImport(); render(<Import lane={lane} />); await prepare();
    await user.click(screen.getByRole("button", { name: "确认导入这份准备" })); await screen.findByText(/上次结果未知/);
    await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    await waitFor(() => expect(lane.snapshot().unknown).toBe(false));
    expect(screen.getByLabelText("选择导出文件")).toBeEnabled();
    expect(screen.queryByRole("button", { name: "确认导入这份准备" })).toBeNull(); expect(core.callsTo("confirm_import")).toHaveLength(1);
    expect(core.callsTo("prepare_import")).toHaveLength(1);
  });
  it("刷新只带回UUID，用户查询后可见Committed，挂载不查询或重发", async () => {
    const committed = anImportOperationView({ operation: anImportOperation({ state: "committed", receipt: anImportOperation().preview, audit_state: "recorded" }), prepared_here: false });
    const core = installFakeCore({ importOperations: [committed] }); const old = new DurableImport(); old.remember(IMPORT_OPERATION_ID);
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />);
    expect(screen.getByLabelText("操作编号")).toHaveValue(IMPORT_OPERATION_ID);
    expect(core.callsTo("import_operation")).toHaveLength(0); expect(core.callsTo("confirm_import")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "查询这个编号" }));
    expect(await screen.findByText(/已提交：新增事件/)).toBeVisible(); expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
  it("未知X遇到历史Y和截断列表缺X仍锁定，不让Y覆盖待核对编号", async () => {
    const history = anImportOperationView({ operation: anImportOperation({ operation_id: "0192f000-0000-7000-8000-00000000dd01", state: "committed", receipt: anImportOperation().preview }), prepared_here: false });
    const core = installFakeCore({ recentImports: () => [history] }); const lane = new DurableImport(); lane.remember(IMPORT_OPERATION_ID);
    await expect(lane.run(() => Promise.reject(new Error("lost X")))).rejects.toThrow();
    const user = userEvent.setup(); render(<Import lane={lane} />); await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    await screen.findByText(/已提交：新增事件/);
    expect(lane.snapshot().unknown).toBe(true); expect(lane.snapshot().operationId).toBe(IMPORT_OPERATION_ID);
    expect(screen.getByLabelText("选择导出文件")).toBeDisabled(); expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
  it("无UUID的准备失联不能仅凭recent空列表解锁，须显式获得服务端空闲证明", async () => {
    let safe = false;
    const core = installFakeCore({ recentImports: () => [], recoveringImport: () => ({ active_worker: !safe,
      reserved_operations: safe ? 0 : 1, running_operations: safe ? 0 : null,
      held_operation_id: safe ? null : IMPORT_OPERATION_ID, can_start_new_intent: safe }) });
    const lane = new DurableImport(); await expect(lane.run(() => Promise.reject(new Error("lost prepare")), null)).rejects.toThrow();
    const user = userEvent.setup(); render(<Import lane={lane} />); await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    await waitFor(() => expect(core.callsTo("import_recovery_state")).toHaveLength(1));
    expect(lane.snapshot().unknown).toBe(true); expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
    safe = true; await waitFor(() => expect(screen.getByRole("button", { name: "核对最近记录" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    await waitFor(() => expect(lane.snapshot().unknown).toBe(false));
    expect(core.callsTo("prepare_import")).toHaveLength(0); expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
  it("明确查到服务端仍持有的旧reserved准备后，仅精确discard其ID才能建新意图", async () => {
    const core = installFakeCore({ importOperations: [anImportOperationView()] });
    const lane = new DurableImport(); lane.remember(IMPORT_OPERATION_ID);
    await expect(lane.run(() => Promise.reject(new Error("lost confirmation")))).rejects.toThrow();
    const user = userEvent.setup(); render(<Import lane={lane} />); await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    const discard = await screen.findByRole("button", { name: `丢弃操作 ${IMPORT_OPERATION_ID} 的旧准备` });
    expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
    await user.click(discard); await waitFor(() => expect(lane.snapshot().unknown).toBe(false));
    expect(core.callsTo("discard_import")[0]?.payload).toEqual({ operationId: IMPORT_OPERATION_ID, correlationId: expect.any(String) });
    expect(core.callsTo("confirm_import")).toHaveLength(0);
    await prepare(); expect(core.callsTo("prepare_import")).toHaveLength(1);
  });
  it("无UUID失联后明确查到唯一prepared_here旧准备，也可以精确discard而不自动确认", async () => {
    const core = installFakeCore({ importOperations: [anImportOperationView()] });
    const lane = new DurableImport(); await expect(lane.run(() => Promise.reject(new Error("lost prepare response")), null)).rejects.toThrow();
    const user = userEvent.setup(); render(<Import lane={lane} />); await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    await user.click(await screen.findByRole("button", { name: `丢弃操作 ${IMPORT_OPERATION_ID} 的旧准备` }));
    await waitFor(() => expect(lane.snapshot().unknown).toBe(false));
    expect(core.callsTo("discard_import")[0]?.payload).toEqual({ operationId: IMPORT_OPERATION_ID, correlationId: expect.any(String) });
    expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
  it("recent读取之后迟到的旧恢复证明不解除另一shared-lane新意图的未知状态", async () => {
    let resolve!: (value: import("../core").ImportRecoveryState) => void;
    const proof = new Promise<import("../core").ImportRecoveryState>((yes) => { resolve = yes; });
    const core = installFakeCore({ recentImports: () => [], recoveringImport: () => proof });
    const lane = new DurableImport(); const user = userEvent.setup(); render(<Import lane={lane} />);
    await user.click(screen.getByRole("button", { name: "核对最近记录" })); await waitFor(() => expect(core.callsTo("import_recovery_state")).toHaveLength(1));
    await expect(lane.run(() => Promise.reject(new Error("new preparation lost")), null)).rejects.toThrow();
    resolve({ active_worker: false, reserved_operations: 0, running_operations: 0, held_operation_id: null, can_start_new_intent: true }); await proof;
    await waitFor(() => expect(screen.getByRole("button", { name: "核对最近记录" })).toBeEnabled());
    expect(lane.snapshot().unknown).toBe(true); expect(screen.getByLabelText("选择导出文件")).toBeDisabled();
    expect(core.callsTo("prepare_import")).toHaveLength(0); expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
});
