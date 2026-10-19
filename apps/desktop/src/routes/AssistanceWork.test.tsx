import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { setUserEndpoint, type AssistanceWorkMetadata } from "../core";
import { anAssistanceRow, anAssistanceWork, anAssistanceWorkStatus, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION, forbidNetwork, installFakeCore } from "../test/fakeCore";
import { AssistanceItemWork, AssistanceWork } from "./AssistanceWork";
import { assistanceWrites } from "./assistanceRequests";

const item = anAssistanceRow();
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
async function move(hash: string): Promise<void> {
  await act(async () => { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); });
  await act(async () => { await new Promise<void>((resolve) => { window.setTimeout(resolve, 0); }); });
}
beforeEach(async () => { installFakeCore(); assistanceWrites.checked(); await move("#/assistance"); });
afterEach(async () => { cleanup(); await waitFor(() => expect(assistanceWrites.snapshot().busy).toBe(false)); assistanceWrites.checked(); });

describe("U3显式本地工作核对", () => {
  it("挂载不读状态、不调度，明确核对只返回metadata与真实缺测", async () => {
    const network = forbidNetwork(); const core = installFakeCore({ assistanceWorkStatus: () => anAssistanceWorkStatus({ runtime_state: "failed", last_runtime_error: "WORK_DRIVER_FAILED", pending_items: null, claimed_items: null,
      foreground_preparation_held: true, coordination_unknown: true, collector_paused_for_import: true, can_begin: false }) });
    const user = userEvent.setup(); render(<AssistanceWork />);
    expect(core.callsTo("assistance_work_status")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "核对本地工作状态" }));
    const status = await screen.findByTestId("assistance-work-status");
    expect(status).toHaveTextContent("后台运行：运行失败"); expect(status).toHaveTextContent("WORK_DRIVER_FAILED");
    expect(status).toHaveTextContent("等待项：暂不可读；已认领项：暂不可读");
    expect(status).toHaveTextContent("待确认的前台准备"); expect(status).toHaveTextContent("协调结果未知"); expect(status).toHaveTextContent("采集已为导入暂停");
    expect(core.calls).toHaveLength(1); expect(network).toEqual([]);
  });
  it("failed只有本人明确retry才交当前CAS，不清前台准备、不自动采用草稿", async () => {
    const core = installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: "failed", failure_code: "LOCAL_WORK_FAILED" }) });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    expect(core.callsTo("assistance_item_work")).toHaveLength(0); expect(core.callsTo("retry_assistance_work")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" }));
    expect(await screen.findByTestId("assistance-item-work")).toHaveTextContent("本地准备失败");
    await user.click(screen.getByRole("button", { name: "重试本地准备" }));
    expect(await screen.findByText("等待本地准备")).toBeVisible();
    expect(core.callsTo("retry_assistance_work")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION } });
    expect(core.callsTo("assistance_item_work")).toHaveLength(1);
    expect(core.calls.some((call) => /discard_draft|prepare_.*context|generate_draft|draft_reply/.test(call.cmd))).toBe(false);
  });
  it.each(["unknown", "running", "source_unavailable"] as const)("%s仅核对，不提供retry或改变在途状态", async (state) => {
    const core = installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state, claim_id: state === "running" ? "claim" : null }) });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" })); await screen.findByTestId("assistance-item-work");
    expect(screen.queryByRole("button", { name: "重试本地准备" })).toBeNull();
    expect(core.callsTo("retry_assistance_work")).toHaveLength(0);
    expect(core.calls).toHaveLength(1);
  });
  it("ready只说明当前本地准备完成，不自动预览或生成", async () => {
    const core = installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: "ready", result_revision: item.revision }) });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" }));
    expect(await screen.findByText("当前版本的本地准备已完成")).toBeVisible();
    expect(core.calls).toHaveLength(1);
  });
  it.each([
    { result_revision: "old" }, { pending_revision: item.revision }, { claimed_revision: item.revision }, { claim_id: "another-claim" },
  ])("ready有旧结果/新pending/claim时不能冒充当前准备：%j", async (changes) => {
    installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: "ready", result_revision: item.revision, ...changes }) });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" }));
    expect(await screen.findByText(/准备状态需重新核对/)).toBeVisible();
    expect(screen.queryByText("当前版本的本地准备已完成")).toBeNull();
  });
  it("元数据的当前revision已变化时不显示旧failed或retry", async () => {
    installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: "failed", item_revision: "new-revision" }) });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" }));
    expect(await screen.findByText(/事项版本已变化/)).toBeVisible();
    expect(screen.queryByTestId("assistance-item-work")).toBeNull(); expect(screen.queryByRole("button", { name: "重试本地准备" })).toBeNull();
  });
  it("retry未知异常只允许核对，不自动重发", async () => {
    let reads = 0; const core = installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: ++reads === 1 ? "failed" : "unknown" }),
      retryingAssistanceWork: () => { throw new Error("合成lost completion"); } });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" })); await user.click(await screen.findByRole("button", { name: "重试本地准备" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("重试结果尚未确认");
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" }));
    expect(await screen.findByText("本地准备结果尚未确认")).toBeVisible();
    expect(core.callsTo("retry_assistance_work")).toHaveLength(1); expect(screen.queryByRole("button", { name: "重试本地准备" })).toBeNull();
  });
  it.each(["source", "address_aba", "item_revision", "unmount"])("迟到状态遇%s拒收旧身份", async (change) => {
    const pending = deferred<AssistanceWorkMetadata>(); const core = installFakeCore({ readingAssistanceWork: () => pending.promise });
    let actual = item; const user = userEvent.setup(); const view = render(<AssistanceItemWork item={item} getCurrentItem={() => actual} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" })); await waitFor(() => expect(core.callsTo("assistance_item_work")).toHaveLength(1));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "address_aba") { await move("#/memory"); await move("#/assistance"); }
    else if (change === "item_revision") actual = { ...item, revision: "new" };
    else view.unmount();
    await act(async () => { pending.resolve(anAssistanceWork({ state: "ready", result_revision: item.revision })); await pending.promise; });
    expect(screen.queryByText("当前版本的本地准备已完成")).toBeNull(); expect(core.callsTo("retry_assistance_work")).toHaveLength(0);
  });
  it("旧retry按钮在微任务前离开地址不能启动写入", async () => {
    const core = installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: "failed" }) });
    const user = userEvent.setup(); render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" })); const retry = await screen.findByRole("button", { name: "重试本地准备" });
    await act(async () => { fireEvent.click(retry); window.location.hash = "#/memory"; window.dispatchEvent(new HashChangeEvent("hashchange")); });
    await waitFor(() => expect(assistanceWrites.snapshot().busy).toBe(false)); expect(core.callsTo("retry_assistance_work")).toHaveLength(0);
  });
  it("retry跨挂载保持写通道，实际settle前不能再次提交", async () => {
    const pending = deferred<AssistanceWorkMetadata>();
    const core = installFakeCore({ readingAssistanceWork: () => anAssistanceWork({ state: "failed" }), retryingAssistanceWork: () => pending.promise });
    const user = userEvent.setup(); const first = render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    await user.click(screen.getByRole("button", { name: "核对这项的准备状态" })); await user.click(await screen.findByRole("button", { name: "重试本地准备" }));
    await waitFor(() => expect(core.callsTo("retry_assistance_work")).toHaveLength(1)); first.unmount();
    render(<AssistanceItemWork item={item} getCurrentItem={() => item} />);
    expect(screen.getByRole("button", { name: "核对这项的准备状态" })).toBeDisabled();
    await act(async () => { pending.resolve(anAssistanceWork({ state: "pending", pending_revision: item.revision })); await pending.promise; });
    await waitFor(() => expect(screen.getByRole("button", { name: "核对这项的准备状态" })).toBeEnabled());
    expect(core.callsTo("retry_assistance_work")).toHaveLength(1);
  });
});
