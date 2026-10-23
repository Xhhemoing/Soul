/** Real core wrapper and route; native responses are synthetic and never read files. */
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ImportNamespace, ImportOperationView } from "../core";
import { aPreparedImport, anImportOperation, anImportOperationView,
  IMPORT_NAMESPACE_ID, installFakeCore } from "../test/fakeCore";
import { Import } from "./Import";
import { DurableImport } from "./durableImport";
import { updateMemory } from "../core";

const SOURCE: ImportNamespace = { namespace_id: IMPORT_NAMESPACE_ID,
  source: "soul-import-v1", state: "active", coverage: "full_from_creation" };
const option = (): HTMLOptionElement | null => screen.queryByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) });
function committed(): ImportOperationView {
  return anImportOperationView({ operation: anImportOperation({ state: "committed",
    receipt: anImportOperation().preview, audit_state: "recorded" }), prepared_here: false });
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
async function stage(): Promise<void> {
  const file = new File(["SYNTHETIC"], "source.jsonl");
  Object.defineProperty(file, "text", { value: vi.fn().mockResolvedValue("SYNTHETIC") });
  fireEvent.change(screen.getByLabelText("选择导出文件"), { target: { files: [file] } });
  await screen.findByRole("heading", { name: "确认这份不可变导入准备" });
}
beforeEach(() => { localStorage.clear(); });

describe("提交后的明确来源选择", () => {
  it("其他写入结束后刷新来源，不自动准备或确认导入", async () => {
    let sources: readonly ImportNamespace[] = [];
    const core = installFakeCore({ listingImportNamespaces: () => sources });
    render(<Import lane={new DurableImport()} />);
    await waitFor(() => expect(core.callsTo("list_import_namespaces")).toHaveLength(1));
    await act(async () => { sources = [SOURCE]; await updateMemory("synthetic-memory", { summary: "synthetic" }); });
    expect(await screen.findByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) })).toBeEnabled();
    expect(core.callsTo("prepare_import")).toHaveLength(0);
    expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
  it("提交后立即提供新来源，仍须明确选择才用于下一份准备", async () => {
    let sources: readonly ImportNamespace[] = [];
    const core = installFakeCore({ preparingImport: () => aPreparedImport(),
      listingImportNamespaces: () => sources,
      confirmingImport: () => { sources = [SOURCE]; return committed(); } });
    const user = userEvent.setup();
    render(<Import lane={new DurableImport()} />);
    await waitFor(() => expect(core.callsTo("list_import_namespaces")).toHaveLength(1));
    await stage();
    await user.click(screen.getByRole("button", { name: "确认导入这份准备" }));
    await screen.findByText(/已提交：新增事件/);
    expect(await screen.findByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) })).toBeEnabled();
    expect(screen.getByLabelText("来源身份")).toHaveValue("");
    expect(core.callsTo("prepare_import")).toHaveLength(1);
    expect(core.callsTo("confirm_import")).toHaveLength(1);
    await user.selectOptions(screen.getByLabelText("来源身份"), IMPORT_NAMESPACE_ID);
    await stage();
    expect(core.callsTo("prepare_import")[1]?.payload).toEqual({ correlationId: expect.any(String), request: {
      text: "SYNTHETIC", source: "soul-import-v1", namespace_id: IMPORT_NAMESPACE_ID, legacy_choice: "reject" } });
  });

  it("切页期间提交完成，新挂载也刷新来源且不重新确认", async () => {
    const pending = deferred<ImportOperationView>();
    let sources: readonly ImportNamespace[] = [];
    const core = installFakeCore({ preparingImport: () => aPreparedImport(),
      listingImportNamespaces: () => sources, confirmingImport: () => pending.promise });
    const lane = new DurableImport(); const user = userEvent.setup();
    const old = render(<Import lane={lane} />); await stage();
    await user.click(screen.getByRole("button", { name: "确认导入这份准备" }));
    old.unmount(); render(<Import lane={lane} />);
    await waitFor(() => expect(core.callsTo("list_import_namespaces")).toHaveLength(2));
    expect(option()).toBeNull();
    await act(async () => { sources = [SOURCE]; pending.resolve(committed()); await pending.promise; });
    expect(await screen.findByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) })).toBeEnabled();
    expect(screen.getByLabelText("来源身份")).toHaveValue("");
    expect(core.callsTo("confirm_import")).toHaveLength(1);
    expect(lane.snapshot().busy).toBe(false);
  });

  it.each(["success", "failure"] as const)("提交前的迟到%s不覆盖新来源或新读取状态", async (late) => {
    const initial = deferred<readonly ImportNamespace[]>();
    let reads = 0;
    const core = installFakeCore({ preparingImport: () => aPreparedImport(),
      listingImportNamespaces: () => ++reads === 1 ? initial.promise : [SOURCE], confirmingImport: committed });
    const user = userEvent.setup(); render(<Import lane={new DurableImport()} />);
    await stage(); await user.click(screen.getByRole("button", { name: "确认导入这份准备" }));
    await screen.findByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) });
    await act(async () => {
      if (late === "success") initial.resolve([]); else initial.reject(new Error("synthetic old list failure"));
      await initial.promise.catch(() => {});
    });
    expect(option()).toBeEnabled();
    expect(screen.queryByText(/来源列表暂时无法更新/)).toBeNull();
    expect(core.callsTo("confirm_import")).toHaveLength(1);
  });

  it("刷新失败保留成功回执与可用状态，显式核对后恢复列表", async () => {
    const receipt = committed(); let reads = 0;
    const core = installFakeCore({ preparingImport: () => aPreparedImport(), confirmingImport: () => receipt,
      listingImportNamespaces: () => {
        reads += 1;
        if (reads === 2) throw new Error("synthetic source list refusal");
        return reads === 1 ? [] : [SOURCE];
      }, recentImports: () => [receipt] });
    const lane = new DurableImport(); const user = userEvent.setup();
    render(<Import lane={lane} />); await stage();
    await user.click(screen.getByRole("button", { name: "确认导入这份准备" }));
    expect(await screen.findByText(/来源列表暂时无法更新/)).toBeVisible();
    expect(screen.getByText(/已提交：新增事件/)).toBeVisible();
    expect(screen.getByLabelText("选择导出文件")).toBeEnabled();
    expect(lane.snapshot()).toMatchObject({ busy: false, unknown: false, receipt });
    await user.click(screen.getByRole("button", { name: "核对最近记录" }));
    expect(await screen.findByRole("option", { name: new RegExp(IMPORT_NAMESPACE_ID) })).toBeEnabled();
    expect(screen.queryByText(/来源列表暂时无法更新/)).toBeNull();
    expect(core.callsTo("prepare_import")).toHaveLength(1);
    expect(core.callsTo("confirm_import")).toHaveLength(1);
  });

  it.each(["success", "failure"] as const)("卸载后的迟到%s不能影响新页面", async (late) => {
    const pending = deferred<readonly ImportNamespace[]>(); let reads = 0;
    const core = installFakeCore({ listingImportNamespaces: () => ++reads === 1 ? pending.promise : [] });
    const lane = new DurableImport(); const old = render(<Import lane={lane} />);
    await waitFor(() => expect(core.callsTo("list_import_namespaces")).toHaveLength(1));
    old.unmount(); render(<Import lane={lane} />);
    await waitFor(() => expect(core.callsTo("list_import_namespaces")).toHaveLength(2));
    await act(async () => {
      if (late === "success") pending.resolve([SOURCE]); else pending.reject(new Error("synthetic unmounted read failure"));
      await pending.promise.catch(() => {});
    });
    expect(option()).toBeNull();
    expect(screen.queryByText(/来源列表暂时无法更新/)).toBeNull();
    expect(core.callsTo("prepare_import")).toHaveLength(0);
    expect(core.callsTo("confirm_import")).toHaveLength(0);
  });
});
