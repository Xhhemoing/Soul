import { StrictMode, useLayoutEffect } from "react";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import * as core from "../core";
import { useRouteAddress } from "../routeAddress";
import { aForgetReceipt, aMemoryDetail, aMemoryList, aTemplateDraft, installFakeCore } from "../test/fakeCore";
import { Memory } from "./Memory";
import { draftMemoryCandidates } from "./draftMemorySelection";
import { draftLane } from "./draftRequests";

const A = "0192f000-0000-7000-8000-0000000000a1";
const B = "0192f000-0000-7000-8000-0000000000b2";
const hash = (id: string): string => `#/memory?memory_id=${id}`;
const detail = (id: string) => aMemoryDetail({ memory_id: id, title: `合成标题${id}`, summary: `合成正文${id}` });
const rows = aMemoryList({ memories: [A, B].map((id) => ({ ...aMemoryList().memories[0]!, memory_id: id })) });
function move(target: string): void { window.location.hash = target; window.dispatchEvent(new HashChangeEvent("hashchange")); }
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
function CommitProbe({ inspect }: { readonly inspect: (hash: string) => void }) {
  const address = useRouteAddress(); useLayoutEffect(() => inspect(address.hash), [address, inspect]); return null;
}
afterEach(() => vi.restoreAllMocks());

describe("精确记忆深链", () => {
  it("真实App路由在明确导航后只打开指定记忆，不自动修改或遗忘", async () => {
    const fake = installFakeCore({ memories: () => rows, opening: detail });
    window.location.hash = hash(A); render(<StrictMode><App /></StrictMode>);
    expect(await screen.findByDisplayValue(`合成正文${A}`)).toBeVisible();
    expect(fake.callsTo("memory_detail")).toHaveLength(1);
    expect(fake.callsTo("memory_detail")[0]?.payload).toEqual({ memoryId: A });
    for (const command of ["create_memory", "update_memory", "preview_forget", "forget_memory"]) expect(fake.callsTo(command)).toHaveLength(0);
  });
  it.each(["#/memory?memory_id=bad", `#/memory?memory_id=${A}&memory_id=${B}`, `#/memory?memory_id=${A}&extra=x`])("非法链接不读取任何详情 %s", async (target) => {
    const fake = installFakeCore({ memories: () => rows, opening: detail });
    window.location.hash = target; render(<Memory />);
    expect(await screen.findByText("记忆地址无效，请从记忆列表重新选择。")).toBeVisible();
    expect(fake.callsTo("memory_list")).toHaveLength(0);
    expect(fake.callsTo("memory_detail")).toHaveLength(0);
    expect(screen.queryByRole("region", { name: "打开的这一条" })).toBeNull();
  });
  it("错误ID响应不会显示其他记忆正文", async () => {
    installFakeCore({ memories: () => rows, opening: () => detail(B) });
    window.location.hash = hash(A); render(<Memory />);
    expect(await screen.findByTestId("memory-refusal-code")).toHaveTextContent("MEMORY_TARGET_MISMATCH");
    expect(screen.queryByDisplayValue(`合成正文${B}`)).toBeNull();
  });
  it("同页A到B切换在提交时已经隐藏A，旧按钮不能写入", async () => {
    const fake = installFakeCore({ memories: () => rows, opening: detail });
    window.location.hash = hash(A);
    // Finish only the initial navigation before capturing A's controls. The
    // A -> B navigation below still attacks the old handlers before commit.
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
    const observations: boolean[] = [];
    const inspect = (target: string): void => { if (target === hash(B)) observations.push(screen.queryByDisplayValue(`合成正文${A}`) === null); };
    render(<><Memory /><CommitProbe inspect={inspect} /></>);
    await screen.findByDisplayValue(`合成正文${A}`);
    const oldSave = screen.getByRole("button", { name: "改好了" });
    const oldCandidate = screen.getByRole("button", { name: "用于本次起草" });
    act(() => { window.location.hash = hash(B); oldSave.click(); oldCandidate.click(); window.dispatchEvent(new HashChangeEvent("hashchange")); });
    await screen.findByDisplayValue(`合成正文${B}`);
    expect(observations.length).toBeGreaterThan(0); expect(observations.every(Boolean)).toBe(true);
    expect(fake.callsTo("update_memory")).toHaveLength(0);
    expect(draftMemoryCandidates.snapshot()).not.toContain(A);
  });
  it("编辑点击后到派发microtask之间来源改变，也不能写回旧正文", async () => {
    const fake = installFakeCore({ memories: () => rows, opening: detail });
    window.location.hash = hash(A);
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
    render(<Memory />);
    await screen.findByDisplayValue(`合成正文${A}`);
    const oldSave = screen.getByRole("button", { name: "改好了" });
    await act(async () => { oldSave.click(); await core.updateMemory(B, { summary: "合成的外部更改" }); });
    await waitFor(() => expect(fake.callsTo("update_memory").length).toBeGreaterThan(0));
    expect(fake.callsTo("update_memory").map((call) => (call.payload as { memoryId: string }).memoryId)).toEqual([B]);
  });
  it.each(["address_aba", "source", "unmount"] as const)("迟到详情遇%s不复活", async (change) => {
    const fake = installFakeCore({ memories: () => rows }); const pending = deferred<core.MemoryDetail>();
    vi.spyOn(core, "memoryDetail").mockImplementationOnce(() => pending.promise).mockImplementation(() => new Promise<core.MemoryDetail>(() => {}));
    window.location.hash = hash(A); const view = render(<StrictMode><Memory /></StrictMode>);
    await waitFor(() => expect(core.memoryDetail).toHaveBeenCalledTimes(1));
    if (change === "address_aba") { act(() => move(hash(B))); act(() => move(hash(A))); }
    else if (change === "source") await act(async () => { await core.updateMemory(A, { summary: "合成修改" }); });
    else view.unmount();
    await act(async () => { pending.resolve(detail(A)); await pending.promise; });
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
    expect(fake.callsTo("preview_forget")).toHaveLength(0); expect(fake.callsTo("forget_memory")).toHaveLength(0);
  });
  it.each(["update", "forget"] as const)("%s未结束时导航不释放物理写入，新目标等回执后才读取", async (operation) => {
    const update = deferred<core.MemoryDetail>(); const forgetting = deferred<core.ForgetReceipt>();
    const fake = installFakeCore({ memories: () => rows, opening: detail,
      editing: () => update.promise, forgetting: () => forgetting.promise });
    const user = userEvent.setup(); window.location.hash = hash(A);
    // Settle the initial browser hashchange before mounting, so this test has
    // one navigation intent rather than an extra same-A notification.
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
    render(<Memory />);
    await screen.findByDisplayValue(`合成正文${A}`);
    expect(fake.callsTo("memory_detail").map((call) => call.payload)).toEqual([{ memoryId: A }]);
    if (operation === "update") await user.click(screen.getByRole("button", { name: "改好了" }));
    else {
      await user.click(within(screen.getByTestId(`memory-${A}`)).getByRole("button", { name: "看遗忘会影响什么" }));
      await user.click(await screen.findByRole("button", { name: "就按上面这些，遗忘它" }));
    }
    const command = operation === "update" ? "update_memory" : "forget_memory";
    await waitFor(() => expect(fake.callsTo(command)).toHaveLength(1));
    // Both the explicit commit probe and the browser's native event happen
    // while the write is still pending; neither may start a B read.
    await act(async () => { move(hash(B)); await new Promise<void>((resolve) => window.setTimeout(resolve, 0)); });
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
    expect(fake.callsTo("memory_detail").map((call) => call.payload)).toEqual([{ memoryId: A }]);
    expect(fake.callsTo(command)).toHaveLength(1);
    await act(async () => { if (operation === "update") update.resolve(detail(A)); else forgetting.resolve(aForgetReceipt({ memory_id: A })); });
    expect(await screen.findByDisplayValue(`合成正文${B}`)).toBeVisible();
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
    expect(fake.callsTo("memory_detail").map((call) => call.payload)).toEqual([{ memoryId: A }, { memoryId: B }]);
    expect(fake.callsTo(command)).toHaveLength(1);
  });
  it("已提交更新的旧回执晚于另一项更改时，保留成功事实但不把旧正文标为当前", async () => {
    const pending = deferred<core.MemoryDetail>(); let writes = 0;
    const fake = installFakeCore({ memories: () => rows, opening: detail,
      editing: (id, change) => ++writes === 1 ? pending.promise : { ...detail(id), summary: change.summary ?? detail(id).summary } });
    const user = userEvent.setup(); window.location.hash = hash(A);
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0)); render(<Memory />);
    await screen.findByDisplayValue(`合成正文${A}`);
    await user.click(screen.getByRole("button", { name: "改好了" }));
    await waitFor(() => expect(fake.callsTo("update_memory")).toHaveLength(1));
    await act(async () => { await core.updateMemory(A, { summary: "合成的后续更新" }); });
    await act(async () => { pending.resolve(detail(A)); });
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
    expect(screen.getByTestId("memory-write-source-changed")).toHaveTextContent("写入已收到核心回执");
    expect(fake.callsTo("update_memory")).toHaveLength(2);
  });
  it("只读详情await已完成但最终投递前收到两次来源通知，不显示旧正文", async () => {
    let source = Symbol("synthetic source"); const listeners = new Set<(value: symbol) => void>();
    vi.spyOn(core, "getDraftSourceRevision").mockImplementation(() => source);
    vi.spyOn(core, "subscribeDraftSource").mockImplementation((listener) => {
      listeners.add(listener); listener(source); return () => { listeners.delete(listener); };
    });
    const changeSource = (): void => { source = Symbol("changed source"); listeners.forEach((listener) => listener(source)); };
    const pending = deferred<core.MemoryDetail>(); vi.spyOn(core, "memoryDetail").mockReturnValue(pending.promise);
    installFakeCore({ memories: () => rows }); const user = userEvent.setup(); window.location.hash = "#/memory";
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0)); render(<Memory />);
    await user.click(within(await screen.findByTestId(`memory-${A}`)).getByRole("button", { name: "打开" }));
    await waitFor(() => expect(core.memoryDetail).toHaveBeenCalledTimes(1));
    await act(async () => {
      pending.resolve(detail(A));
      // The detail await continuation was registered before this one. It has
      // checked its source; the outer act delivery has not run yet.
      await pending.promise;
      changeSource(); changeSource();
    });
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
    expect(screen.queryByTestId("memory-write-source-changed")).toBeNull();
  });
  it.each([A, B, "unmount"])("从已应用偏好链接进入时，等待离页清理后只读取最新地址 %s", async (destination) => {
    const discard = deferred<boolean>(); let discards = 0;
    const PERSON = "11111111-1111-4111-8111-111111111111";
    const fake = installFakeCore({ memories: () => rows,
      scopedRecipients: [{ person_id: PERSON, identifier_hint: "synthetic-person" }],
      scopedDrafting: () => ({ result_id: "result", person_id: PERSON, occasion: "personal", draft: aTemplateDraft(),
        applied_preference: { preference_id: "preference", revision: "revision", memory_id: A, register: "casual" } }),
      discardingDraft: () => ++discards === 1 ? true : discard.promise,
      opening: detail,
    });
    const user = userEvent.setup(); window.location.hash = "#/draft";
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0)); const view = render(<App />);
    await user.click(await screen.findByRole("button", { name: "按联系人和场合本地起草" }));
    await user.selectOptions(await screen.findByLabelText("收件人物"), PERSON);
    await user.selectOptions(screen.getByLabelText("场合"), "personal");
    await user.type(screen.getByLabelText("原文"), "合成原文");
    await user.click(screen.getByRole("button", { name: "写一版本地草稿" }));
    await user.click(await screen.findByRole("link", { name: "查看或遗忘这条偏好" }));
    await waitFor(() => expect(fake.callsTo("discard_draft")).toHaveLength(2));
    if (destination === B) await act(async () => { move(hash(B)); });
    if (destination === "unmount") view.unmount();
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
    expect(screen.queryByDisplayValue(`合成正文${B}`)).toBeNull();
    expect(fake.callsTo("memory_detail")).toHaveLength(0);
    await act(async () => { discard.resolve(true); });
    if (destination === "unmount") {
      expect(fake.callsTo("memory_detail")).toHaveLength(0);
      return;
    }
    expect(await screen.findByDisplayValue(`合成正文${destination}`)).toBeVisible();
    expect(screen.queryByTestId("memory-refusal-code")).toBeNull();
    expect(fake.callsTo("memory_detail").map((call) => call.payload)).toEqual([{ memoryId: destination }]);
    expect(fake.callsTo("discard_draft")).toHaveLength(2);
    expect(fake.callsTo("create_memory")).toHaveLength(0);
    expect(fake.callsTo("update_memory")).toHaveLength(0);
    expect(fake.callsTo("preview_forget")).toHaveLength(0);
    expect(fake.callsTo("forget_memory")).toHaveLength(0);
  });
  it("离页清理结果仍不确定时，不把它当成已完成而读取正文", async () => {
    const fake = installFakeCore({ memories: () => rows, opening: detail });
    vi.spyOn(draftLane, "snapshot").mockReturnValue({ busy: false, uncertain: true });
    window.location.hash = hash(A); render(<Memory />);
    expect(await screen.findByTestId("memory-refusal-code")).toBeVisible();
    expect(fake.callsTo("memory_list")).toHaveLength(0);
    expect(fake.callsTo("memory_detail")).toHaveLength(0);
    expect(screen.queryByDisplayValue(`合成正文${A}`)).toBeNull();
  });
});
