import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { setUserEndpoint, type AssistanceLocalPreparation, type AssistanceLocalTemplatePreview } from "../core";
import { anAssistanceLocalPreparation, anAssistanceLocalTemplatePreview, anAssistanceRow, ASSISTANCE_ITEM_ID, ASSISTANCE_LOCAL_PREFERENCE_REVISION, ASSISTANCE_LOCAL_TEMPLATE_ID, ASSISTANCE_REVISION, forbidNetwork, installFakeCore } from "../test/fakeCore";
import { AssistanceLocalPreparationPanel } from "./AssistanceLocalPreparation";
import { assistanceWrites } from "./assistanceRequests";
import { draftLane } from "./draftRequests";
import { draftAssistanceSelection } from "./draftMaterialSelection";

const item = anAssistanceRow();
const ready = (changes: Partial<AssistanceLocalPreparation> = {}): AssistanceLocalPreparation => anAssistanceLocalPreparation({ enabled: true, state: "ready", preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, template_id: ASSISTANCE_LOCAL_TEMPLATE_ID, ...changes });
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
async function move(hash: string): Promise<void> {
  await act(async () => { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); });
  await act(async () => { await new Promise<void>((resolve) => { window.setTimeout(resolve, 0); }); });
}
async function read(): Promise<void> { await userEvent.click(screen.getByRole("button", { name: "核对本地模板准备" })); }
beforeEach(async () => { installFakeCore(); assistanceWrites.checked(); await move("#/assistance"); });
afterEach(async () => {
  cleanup(); await waitFor(() => expect(assistanceWrites.snapshot().busy || draftLane.snapshot().busy).toBe(false));
  const record = `local-preparation:${item.item_id}`; assistanceWrites.checked(record, assistanceWrites.recovery(record)); assistanceWrites.checked();
});

describe("U5明确启用与瞬时本地模板", () => {
  it("默认关闭，挂载零读取；只显式核对metadata不预读正文", async () => {
    const network = forbidNetwork(); const core = installFakeCore(); render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />);
    expect(core.calls).toEqual([]); await read();
    expect(await screen.findByText(/本地模板准备已关闭。偏好：已关闭/)).toBeVisible();
    expect(screen.queryByTestId("assistance-local-template")).toBeNull(); expect(core.calls.map((call) => call.cmd)).toEqual(["assistance_local_preparation"]); expect(network).toEqual([]);
  });
  it("observation没有启用或预览入口，不读取模板metadata", () => {
    const core = installFakeCore(); const observation = anAssistanceRow({ source: { kind: "observation", namespace_id: "namespace", bundle_id: "bundle", contact_id: "contact" } });
    render(<AssistanceLocalPreparationPanel item={observation} getCurrentItem={() => observation} />);
    expect(screen.queryByRole("button")).toBeNull(); expect(core.calls).toEqual([]);
  });
  it("启用和关闭各用item+独立pref CAS；不调度E1或改变Draft选择", async () => {
    const selection = draftAssistanceSelection.snapshot(); const core = installFakeCore();
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read();
    await userEvent.click(await screen.findByRole("button", { name: "启用这项本地模板准备" }));
    expect(await screen.findByText(/等待本地模板准备。偏好：已启用/)).toBeVisible();
    expect(core.callsTo("set_assistance_local_preparation")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, expected_preference_revision: null, enabled: true } });
    await userEvent.click(screen.getByRole("button", { name: "关闭这项本地模板准备" }));
    expect(await screen.findByText(/本地模板准备已关闭。偏好：已关闭/)).toBeVisible();
    expect(core.callsTo("set_assistance_local_preparation")[1]?.payload).toMatchObject({ request: { expected_preference_revision: expect.any(String), enabled: false } });
    expect(core.calls.map((call) => call.cmd)).toEqual(["assistance_local_preparation", "set_assistance_local_preparation", "set_assistance_local_preparation"]); expect(draftAssistanceSelection.snapshot()).toBe(selection);
  });
  it("ready仍不自动读正文；精确模板预览保占位，不采用/批准/发送", async () => {
    const network = forbidNetwork(); const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready() });
    const selection = draftAssistanceSelection.snapshot(); render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read();
    const open = await screen.findByRole("button", { name: "查看本地开场模板" }); expect(core.callsTo("preview_assistance_local_template")).toHaveLength(0);
    await userEvent.click(open); expect(await screen.findByTestId("assistance-local-template")).toHaveTextContent("（这里写你要说的内容）");
    expect(core.callsTo("preview_assistance_local_template")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, expected_preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, template_id: ASSISTANCE_LOCAL_TEMPLATE_ID } });
    expect(core.calls).toHaveLength(2); expect(draftAssistanceSelection.snapshot()).toBe(selection); expect(network).toEqual([]);
  });
  it.each(["none", "pending", "running", "source_unavailable", "failed", "unknown"] as const)("%s仅metadata核对，无旧预览/retry或自动采用", async (state) => {
    const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready({ state }) });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read();
    await screen.findByRole("button", { name: "关闭这项本地模板准备" }); expect(screen.queryByRole("button", { name: "查看本地开场模板" })).toBeNull(); expect(core.calls).toHaveLength(1);
  });
  it("明确重读metadata立即隐藏正文；旧预览按钮不能跨pref/template变更发请求", async () => {
    let reads = 0; const core = installFakeCore({ readingAssistanceLocalPreparation: () => ++reads === 1 ? ready() : ready({ preference_revision: "new-pref", template_id: "new-template" }) });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read();
    const oldOpen = await screen.findByRole("button", { name: "查看本地开场模板" }); await userEvent.click(oldOpen); await screen.findByTestId("assistance-local-template");
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: "核对本地模板准备" })); fireEvent.click(oldOpen); });
    await waitFor(() => expect(core.callsTo("assistance_local_preparation")).toHaveLength(2));
    expect(screen.queryByTestId("assistance-local-template")).toBeNull(); expect(core.callsTo("preview_assistance_local_template")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "查看本地开场模板" }));
    await screen.findByTestId("assistance-local-template"); expect(core.callsTo("preview_assistance_local_template")[1]?.payload).toMatchObject({ request: { expected_preference_revision: "new-pref", template_id: "new-template" } });
  });
  it("failed只有明确重新准备才用同API新pref CAS排刷新，不自动retry", async () => {
    const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready({ state: "failed", template_id: null, failure_code: "ASSISTANCE_LOCAL_PROFILE_UNAVAILABLE" }) });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read();
    const retry = await screen.findByRole("button", { name: "重新准备本地模板" }); expect(core.callsTo("set_assistance_local_preparation")).toHaveLength(0);
    await userEvent.click(retry); await screen.findByText(/等待本地模板准备。偏好：已启用/);
    expect(core.callsTo("set_assistance_local_preparation")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, expected_preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, enabled: true } });
    expect(core.callsTo("retry_assistance_work")).toHaveLength(0); expect(core.callsTo("preview_assistance_local_template")).toHaveLength(0);
  });
  it("慢preview期间可以明确关闭；旧正文不落UI且物理槽直到实际settle才释放", async () => {
    const pending = deferred<AssistanceLocalTemplatePreview>(); const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready(), previewingAssistanceLocalTemplate: () => pending.promise });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "查看本地开场模板" }));
    expect(draftLane.snapshot().busy).toBe(true); await userEvent.click(screen.getByRole("button", { name: "关闭这项本地模板准备" }));
    expect(await screen.findByText(/本地模板准备已关闭。偏好：已关闭/)).toBeVisible(); expect(draftLane.snapshot().busy).toBe(true);
    await act(async () => { pending.resolve(anAssistanceLocalTemplatePreview()); await pending.promise; });
    await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); expect(screen.queryByTestId("assistance-local-template")).toBeNull();
    expect(core.callsTo("discard_draft")).toHaveLength(0); expect(core.callsTo("retry_assistance_work")).toHaveLength(0);
  });
  it("明确关闭busy拒绝不宣称已关闭，也不自动重试", async () => {
    const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready(), settingAssistanceLocalPreparation: () => { throw { reason_code: "ASSISTANCE_BUSY", explanation: "本地核对繁忙，请稍后明确重试。" }; } });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "关闭这项本地模板准备" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("繁忙"); expect(screen.queryByText("本地模板准备已关闭；基础核对与提醒安排保持各自的记录。")).toBeNull(); expect(core.callsTo("set_assistance_local_preparation")).toHaveLength(1); expect(assistanceWrites.snapshot().unknown).toBe(false);
  });
  it("未知偏好写不重放；只有新的相关metadata核对可恢复明确操作", async () => {
    let writes = 0; const core = installFakeCore({ settingAssistanceLocalPreparation: (request) => {
      if (++writes === 1) throw new Error("合成response lost"); return anAssistanceLocalPreparation({ enabled: request.enabled, state: "pending", preference_revision: "new-pref" });
    } });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "启用这项本地模板准备" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("结果尚未确认"); assistanceWrites.checked(); expect(assistanceWrites.snapshot().unknown).toBe(true);
    await read(); await userEvent.click(await screen.findByRole("button", { name: "启用这项本地模板准备" }));
    await screen.findByText(/等待本地模板准备。偏好：已启用/); expect(core.callsTo("set_assistance_local_preparation")).toHaveLength(2);
  });
  it.each(["source", "address_aba", "item_revision", "unmount"])("preview迟到遇%s拒绝正文，实际settle后才释放槽", async (change) => {
    const pending = deferred<AssistanceLocalTemplatePreview>(); installFakeCore({ readingAssistanceLocalPreparation: () => ready(), previewingAssistanceLocalTemplate: () => pending.promise });
    let actual = item; const view = render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => actual} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "查看本地开场模板" }));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "address_aba") { await move("#/memory"); await move("#/assistance"); }
    else if (change === "item_revision") actual = { ...item, revision: "new" }; else view.unmount();
    expect(draftLane.snapshot().busy).toBe(true); await act(async () => { pending.resolve(anAssistanceLocalTemplatePreview()); await pending.promise; });
    await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); expect(screen.queryByTestId("assistance-local-template")).toBeNull();
  });
  it("跨挂载保持preview物理槽，不接第二次请求或丢失真执行结束", async () => {
    const pending = deferred<AssistanceLocalTemplatePreview>(); const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready(), previewingAssistanceLocalTemplate: () => pending.promise });
    const first = render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "查看本地开场模板" })); first.unmount();
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); expect(await screen.findByRole("button", { name: "查看本地开场模板" })).toBeDisabled();
    await act(async () => { pending.resolve(anAssistanceLocalTemplatePreview()); await pending.promise; }); await waitFor(() => expect(draftLane.snapshot().busy).toBe(false));
    expect(screen.queryByTestId("assistance-local-template")).toBeNull(); expect(core.callsTo("preview_assistance_local_template")).toHaveLength(1);
  });
  it("错template/pref正文不显示，要求新的明确核对而不是自动重读或重投", async () => {
    const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready(), previewingAssistanceLocalTemplate: () => anAssistanceLocalTemplatePreview({ preference_revision: "old-pref" }) });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "查看本地开场模板" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("重新核对本地准备"); expect(screen.queryByTestId("assistance-local-template")).toBeNull(); expect(screen.getByRole("button", { name: "查看本地开场模板" })).toBeDisabled(); expect(core.calls).toHaveLength(2);
  });
  it("本地预览不清除既有E1 uncertainty或请求discard/held材料", async () => {
    const old = draftLane.begin("prepare"); expect(old).not.toBeNull(); if (old === null) return; draftLane.finish(old, true);
    const core = installFakeCore({ readingAssistanceLocalPreparation: () => ready() });
    render(<AssistanceLocalPreparationPanel item={item} getCurrentItem={() => item} />); await read(); await userEvent.click(await screen.findByRole("button", { name: "查看本地开场模板" }));
    await screen.findByTestId("assistance-local-template"); expect(draftLane.snapshot()).toEqual({ busy: false, uncertain: true });
    expect(core.calls.map((call) => call.cmd)).toEqual(["assistance_local_preparation", "preview_assistance_local_template"]);
  });
});
