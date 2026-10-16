/** Actual Draft + core wrappers, explicit memory purpose and result ownership. */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { MemoryContextView } from "../core";
import { setUserEndpoint } from "../core";
import { aMemoryContextView, aMemoryDraftPlan, aMemorySelection, anE1Plan, aSelectedMemory,
  DRAFT_CONTEXT_ID, DRAFT_MEMORY_ID, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { Draft } from "./Draft";
import { draftMemoryCandidates } from "./draftMemorySelection";
import { draftLane } from "./draftRequests";

const TEXT = "合成粘贴消息";
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
async function idle(): Promise<void> { await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); }
beforeEach(async () => { installFakeCore(); for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id); await idle(); });
afterEach(async () => { cleanup(); for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id); await idle(); });
async function page(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options); draftMemoryCandidates.add(DRAFT_MEMORY_ID); await idle();
  const view = render(<Draft />); const user = userEvent.setup();
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: TEXT } });
  return { core, user, ...view };
}
async function preview(user: ReturnType<typeof userEvent.setup>): Promise<void> {
  await user.click(screen.getByRole("button", { name: "预览本次记忆与出处" }));
  await screen.findByRole("heading", { name: "合成图书馆偏好" }); await idle();
}

describe("本次手选记忆的真实起草UI", () => {
  it("候选和关键词只提交明确范围；默认本人原文未勾选，最终引用与实际字节独立展示", async () => {
    const { core, user } = await page(); await user.type(screen.getByLabelText("仅在本次候选内查关键词"), "图书馆"); await preview(user);
    expect(core.callsTo("prepare_memory_context")[0]?.payload).toEqual({ request: { candidate_ids: [DRAFT_MEMORY_ID], query: "图书馆", memory_type: null, time_specific: false } });
    const owner = screen.getByRole("checkbox", { name: /本次另行允许/ }); expect(owner).not.toBeChecked();
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(core.callsTo("prepare_draft_with_memory")[0]?.payload).toEqual({ pasted: TEXT, contextId: DRAFT_CONTEXT_ID, ownerIds: [], includeOriginal: false });
    expect(owner).toBeDisabled(); expect(screen.getByText(/完整请求 UTF-8 JSON：1024 字节/)).toBeVisible();
    expect(screen.getByText("明确提供的本人记录：无，记忆正文保持本地")).toBeVisible();
    expect(core.callsTo("generate_draft")).toHaveLength(0);
  });
  it("只有显式本人勾选进入helper；粘贴原文二次确认不扩大记忆范围", async () => {
    const mixed = aSelectedMemory({ memory_id: "0192f000-0000-7000-8000-00000000ee12", title: "合成混合记录", privacy_class: "mixed" });
    const { core, user } = await page({ preparingMemory: () => aMemoryContextView({ preview: aMemorySelection({ selected: [aSelectedMemory(), mixed] }) }) });
    draftMemoryCandidates.add(mixed.memory_id); await idle();
    await preview(user); expect(screen.getAllByRole("checkbox", { name: /本次另行允许/ })).toHaveLength(1);
    await user.click(screen.getByRole("checkbox", { name: /本次另行允许/ }));
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" })); await screen.findByTestId("e1-carry-original");
    await user.click(screen.getByTestId("e1-carry-original"));
    await waitFor(() => expect(core.callsTo("prepare_draft_with_memory")).toHaveLength(2));
    expect(core.callsTo("prepare_draft_with_memory").map((call) => call.payload)).toEqual([
      { pasted: TEXT, contextId: DRAFT_CONTEXT_ID, ownerIds: [DRAFT_MEMORY_ID], includeOriginal: false },
      { pasted: TEXT, contextId: DRAFT_CONTEXT_ID, ownerIds: [DRAFT_MEMORY_ID], includeOriginal: true },
    ]); expect(core.callsTo("generate_draft")).toHaveLength(0);
  });
  it("Clarify没有grant时两种生成都阻止，须明确清除后才能普通fallback", async () => {
    const { core, user } = await page({ preparingMemory: () => aMemoryContextView({ context_id: null, preview: aMemorySelection({ selected: [], outcome: "clarify", excluded: [{ memory_id: DRAFT_MEMORY_ID, reason: "temporal_scope_unavailable" }] }) }) });
    await user.click(screen.getByRole("button", { name: "预览本次记忆与出处" })); await screen.findByText(/这次没有可用的记忆许可/);
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled(); expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "清除本次记忆，使用普通起草" })); await idle();
    await user.click(screen.getByRole("button", { name: "写一版草稿" })); await screen.findByRole("heading", { name: "草稿" });
    expect(core.callsTo("draft_reply")).toHaveLength(1); expect(core.callsTo("draft_reply_with_memory")).toHaveLength(0);
  });
  it("关键词编辑清旧context并排队native discard，物理完成前不准新prepare", async () => {
    const drain = deferred<boolean>(); let delay = false;
    const { core, user } = await page({ discardingDraft: () => delay ? drain.promise : true }); await preview(user); delay = true;
    await user.type(screen.getByLabelText("仅在本次候选内查关键词"), "新");
    await waitFor(() => expect(draftLane.snapshot().busy).toBe(true));
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
    expect(screen.getByRole("button", { name: "预览本次记忆与出处" })).toBeDisabled(); expect(core.callsTo("prepare_memory_context")).toHaveLength(1);
    await act(async () => { drain.resolve(true); await drain.promise; }); await idle();
    expect(core.callsTo("discard_draft").length).toBeGreaterThanOrEqual(2);
  });
  it.each(["source", "unmount"] as const)("%s变化让迟到记忆preview失去展示归属", async (change) => {
    const pending = deferred<MemoryContextView>(); const { core, user, unmount } = await page({ preparingMemory: () => pending.promise });
    await user.click(screen.getByRole("button", { name: "预览本次记忆与出处" }));
    await waitFor(() => expect(core.callsTo("prepare_memory_context")).toHaveLength(1));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); }); else unmount();
    await act(async () => { pending.resolve(aMemoryContextView()); await pending.promise; }); await idle();
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull(); expect(core.callsTo("prepare_draft_with_memory")).toHaveLength(0);
  });
  it("paste编辑使正在进行的旧记忆preview失效并在结束后drain", async () => {
    const pending = deferred<MemoryContextView>(); let previews = 0;
    const { core, user } = await page({ preparingMemory: () => previews++ === 0 ? pending.promise : aMemoryContextView() });
    await user.click(screen.getByRole("button", { name: "预览本次记忆与出处" })); await waitFor(() => expect(core.callsTo("prepare_memory_context")).toHaveLength(1));
    fireEvent.change(screen.getByLabelText("原文"), { target: { value: "新的合成原文" } });
    expect(draftLane.snapshot().busy).toBe(true); expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeDisabled();
    await act(async () => { pending.resolve(aMemoryContextView()); await pending.promise; }); await idle();
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
    expect(core.callsTo("discard_draft").length).toBeGreaterThanOrEqual(2);
    await preview(user); expect(core.callsTo("prepare_memory_context")).toHaveLength(2);
  });
  it("最终预算裁减后的引用面板只显示实际保留材料", async () => {
    const removed = aSelectedMemory({ memory_id: "0192f000-0000-7000-8000-00000000ee12", title: "超预算被移除的合成材料" });
    const { core, user } = await page({ preparingMemory: () => aMemoryContextView({ preview: aMemorySelection({ selected: [aSelectedMemory(), removed] }) }),
      preparingDraftMemory: () => aMemoryDraftPlan({ context: aMemorySelection({ excluded: [{ memory_id: removed.memory_id, reason: "final_request_budget" }] }),
        prepared: { ...aMemoryDraftPlan().prepared, plan: anE1Plan(), endpoint_owner_ids: [DRAFT_MEMORY_ID], encoded_bytes: 65530, body_preview: "RETAINED SYNTHETIC MEMORY BODY" } }) });
    draftMemoryCandidates.add(removed.memory_id); await idle();
    await preview(user);
    for (const checkbox of screen.getAllByRole("checkbox", { name: /本次另行允许/ })) await user.click(checkbox);
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    const heading = await screen.findByRole("heading", { name: "这份准备实际保留的记忆与端点材料" }); const panel = heading.closest("section");
    expect(panel).not.toBeNull(); expect(within(panel!).queryByText(removed.title)).toBeNull();
    expect(within(panel!).getByText(/完整请求 UTF-8 JSON：65530 字节/)).toBeVisible();
    expect(within(panel!).getByText(/final_request_budget/)).toBeVisible();
    expect(screen.queryByRole("heading", { name: removed.title })).toBeNull();
    const owner = screen.getAllByRole("checkbox", { name: /本次另行允许/ }); expect(owner).toHaveLength(1); expect(owner[0]).toBeChecked();
    await user.click(within(panel!).getByText("查看实际脱敏材料")); expect(within(panel!).getByText("RETAINED SYNTHETIC MEMORY BODY")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "确认，开始生成" })); await screen.findByRole("heading", { name: "草稿" });
    expect(core.callsTo("generate_draft")).toHaveLength(1); expect(screen.queryByRole("heading", { name: removed.title })).toBeNull();
  });
});
