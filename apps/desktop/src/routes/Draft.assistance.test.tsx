import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { actAssistanceItem, setUserEndpoint, type AssistanceContextView, type Draft as DraftValue } from "../core";
import { anAssistanceContext, anInsightContext, aTemplateDraft, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION, forbidNetwork, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { Draft } from "./Draft";
import { draftAssistanceSelection, draftInsightSelection, draftMemoryCandidates } from "./draftMaterialSelection";
import { draftLane } from "./draftRequests";

const pointer = { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION };
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
async function idle(): Promise<void> { await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); }
async function move(hash: string): Promise<void> {
  await act(async () => { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); });
  await act(async () => { await new Promise<void>((resolve) => { window.setTimeout(resolve, 0); }); });
}
beforeEach(async () => {
  installFakeCore(); draftAssistanceSelection.clear(); draftInsightSelection.clear(); for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id); await idle();
  await move("#/draft");
});
afterEach(async () => { cleanup(); draftAssistanceSelection.clear(); await idle(); });
async function page(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options); draftAssistanceSelection.select(pointer); await idle();
  const view = render(<Draft />); const user = userEvent.setup();
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: "合成待回复消息" } });
  return { core, user, ...view };
}
async function prepared(user: ReturnType<typeof userEvent.setup>): Promise<void> {
  await user.click(screen.getByRole("button", { name: "核对并准备" })); await screen.findByRole("heading", { name: "合成图书馆偏好" }); await idle();
}

describe("事项绑定的现有起草链", () => {
  it("ID意图不会自动准备或出网，显式准备直接使用绑定view做本地起草", async () => {
    const network = forbidNetwork(); const { core, user } = await page();
    expect(core.callsTo("prepare_assistance_context")).toHaveLength(0);
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled();
    await prepared(user);
    expect(core.callsTo("prepare_assistance_context")[0]?.payload).toEqual({ request: pointer });
    expect(core.callsTo("prepare_memory_context")).toHaveLength(0); expect(core.callsTo("memory_detail")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "写一版草稿" })); await screen.findByRole("heading", { name: "草稿" });
    expect(core.callsTo("draft_reply_with_memory")).toHaveLength(1); expect(core.callsTo("draft_reply")).toHaveLength(0);
    expect(core.callsTo("generate_draft")).toHaveLength(0); expect(network).toEqual([]);
  });
  it("事项统计view直接交现有insight helper，计数默认不进入端点，确认前零生成", async () => {
    const { core, user } = await page({ preparingAssistance: () => ({ item_id: ASSISTANCE_ITEM_ID, revision: ASSISTANCE_REVISION, kind: "insight", insight: anInsightContext() }) });
    await user.click(screen.getByRole("button", { name: "核对并准备" })); await idle();
    expect(core.callsTo("prepare_insight_context")).toHaveLength(0);
    expect(screen.getByRole("checkbox", { name: /本次另行允许将这份匿名派生计数/ })).not.toBeChecked();
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" })); await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.callsTo("prepare_draft_with_insight")[0]?.payload).toMatchObject({ includeCounts: false, includeOriginal: false });
    expect(core.callsTo("prepare_draft")).toHaveLength(0); expect(core.callsTo("generate_draft")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "确认，开始生成" })); await screen.findByRole("heading", { name: "草稿" });
    expect(core.callsTo("generate_draft")).toHaveLength(1);
  });
  it("事项完成使已展示参考和批准同步失效，旧DOM不能继续起草", async () => {
    const { core, user } = await page(); await prepared(user);
    const oldLocal = screen.getByRole("button", { name: "写一版草稿" });
    await act(async () => { await actAssistanceItem({ ...pointer, action: "complete_self_reported" }); });
    fireEvent.click(oldLocal);
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
    expect(core.callsTo("draft_reply_with_memory")).toHaveLength(0);
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled();
  });
  it.each(["source", "address_aba", "selection", "unmount"])("事项准备迟到遇%s拒收且实际完成前保持忙碌", async (change) => {
    const pending = deferred<AssistanceContextView>(); const { core, user, unmount } = await page({ preparingAssistance: () => pending.promise });
    await user.click(screen.getByRole("button", { name: "核对并准备" })); await waitFor(() => expect(core.callsTo("prepare_assistance_context")).toHaveLength(1));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "address_aba") { await move("#/memory"); await move("#/draft"); }
    else if (change === "selection") await act(async () => { draftAssistanceSelection.select({ ...pointer, expected_revision: "0192f000-0000-7000-8000-00000000a513" }); });
    else unmount();
    expect(draftLane.snapshot().busy).toBe(true);
    await act(async () => { pending.resolve(anAssistanceContext()); await pending.promise; }); await idle();
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
    expect(core.callsTo("prepare_memory_context")).toHaveLength(0); expect(core.callsTo("generate_draft")).toHaveLength(0);
  });
  it("准备按钮旧DOM在微任务前导航后不启动IPC", async () => {
    const { core } = await page(); const button = screen.getByRole("button", { name: "核对并准备" });
    await act(async () => { fireEvent.click(button); window.location.hash = "#/memory"; window.dispatchEvent(new HashChangeEvent("hashchange")); });
    await idle(); expect(core.callsTo("prepare_assistance_context")).toHaveLength(0);
  });
  it("生成在途改期后拒收旧草稿，原请求实际结束前不释放忙碌状态", async () => {
    const pending = deferred<DraftValue>(); const { core, user } = await page({ generating: () => pending.promise }); await prepared(user);
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" })); await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    await user.click(screen.getByRole("button", { name: "确认，开始生成" })); await waitFor(() => expect(core.callsTo("generate_draft")).toHaveLength(1));
    await act(async () => { await actAssistanceItem({ ...pointer, action: "cancel" }); });
    expect(draftLane.snapshot().busy).toBe(true);
    await act(async () => { pending.resolve(aTemplateDraft({ source: "user_endpoint", text: "不应显示的合成旧草稿" })); await pending.promise; }); await idle();
    expect(screen.queryByText("不应显示的合成旧草稿")).toBeNull(); expect(core.callsTo("generate_draft")).toHaveLength(1);
  });
});
