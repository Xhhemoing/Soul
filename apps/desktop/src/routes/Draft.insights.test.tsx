/** Real App, drafting wrappers and lane; synthetic statistics only. */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "../App";
import { setUserEndpoint, type InsightContextView, type InsightDraftPlan, type MemoryContextView } from "../core";
import { aLocalInsight, aMemoryContextView, aPersonSummary, anE1Plan, anInsightContext, anInsightDraftPlan, anInsightReference,
  DRAFT_MEMORY_ID, INSIGHT_CONTEXT_ID, installFakeCore, forbidNetwork, LOCAL_INSIGHT_ID, type FakeCoreOptions } from "../test/fakeCore";
import { Draft } from "./Draft";
import { draftInsightSelection, draftMemoryCandidates } from "./draftMaterialSelection";
import { draftLane } from "./draftRequests";

const A = aPersonSummary().contact_id;
const B = "0192f000-0000-7000-8000-000000000003";
const TEXT = "合成对方粘贴消息";
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
async function idle(): Promise<void> { await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); }
async function resetSelection(): Promise<void> {
  draftInsightSelection.clear(); for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id); await idle();
}
beforeEach(async () => { installFakeCore(); await resetSelection(); window.location.hash = "#/draft"; });
afterEach(async () => { cleanup(); await idle(); installFakeCore(); await resetSelection(); });
async function page(options: FakeCoreOptions = {}) {
  const native = installFakeCore(options); draftInsightSelection.select(LOCAL_INSIGHT_ID, A); await idle();
  const user = userEvent.setup(); const view = render(<Draft />);
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: TEXT } });
  return { native, user, ...view };
}
async function preview(user: ReturnType<typeof userEvent.setup>): Promise<void> {
  await user.click(screen.getByRole("button", { name: "预览本次统计与出处" }));
  await screen.findByRole("heading", { name: "本次统计材料出处" }); await idle();
}

describe("明确本地统计参与本次起草", () => {
  it("真实Import页从同操作选B只保留ID且替换memory；显式本地预览/起草没有E1", async () => {
    const network = forbidNetwork();
    const native = installFakeCore({ insights: [aLocalInsight({ summaries: [aPersonSummary(), aPersonSummary({ contact_id: B })] })] });
    draftMemoryCandidates.add(DRAFT_MEMORY_ID); await idle(); window.location.hash = "#/import";
    const user = userEvent.setup(); render(<App />);
    await user.click(await screen.findByRole("button", { name: `选作本次起草统计参考（替换记忆参考）：${B}` })); await idle();
    expect(draftMemoryCandidates.snapshot()).toEqual([]);
    expect(draftInsightSelection.snapshot()).toMatchObject({ bundle_id: LOCAL_INSIGHT_ID, contact_id: B });
    expect(native.callsTo("prepare_insight_context")).toHaveLength(0); expect(native.callsTo("prepare_memory_context")).toHaveLength(0);
    fireEvent.change(await screen.findByLabelText("原文"), { target: { value: TEXT } });
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled();
    await preview(user);
    expect(native.callsTo("prepare_insight_context")[0]?.payload).toEqual({ request: { bundle_id: LOCAL_INSIGHT_ID, contact_id: B } });
    expect(screen.getByRole("checkbox", { name: /匿名 A2 派生计数/ })).not.toBeChecked();
    await user.click(screen.getByRole("button", { name: "写一版草稿" })); await screen.findByRole("heading", { name: "草稿" });
    expect(native.callsTo("local_draft_with_insight")[0]?.payload).toEqual({ pasted: TEXT, contextId: INSIGHT_CONTEXT_ID });
    expect(screen.getByText(/本机模板不会根据统计改写/)).toBeVisible();
    expect(native.callsTo("generate_draft")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0);
    expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(0); expect(network).toEqual([]);
  });

  it("默认false只有显式统计勾选进入helper，原文二次确认保持同一范围并最终只批准一次", async () => {
    const { native, user } = await page(); await preview(user);
    const checkbox = screen.getByRole("checkbox", { name: /匿名 A2 派生计数/ }); expect(checkbox).not.toBeChecked();
    await user.click(checkbox); await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await screen.findByRole("heading", { name: "确认这一次要生成什么" }); expect(checkbox).toBeDisabled();
    expect(native.callsTo("prepare_draft_with_insight")[0]?.payload).toEqual({ pasted: TEXT, contextId: INSIGHT_CONTEXT_ID, includeCounts: true, includeOriginal: false, includeGoal: false });
    await user.click(screen.getByTestId("e1-carry-original"));
    await waitFor(() => expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(2));
    expect(native.callsTo("prepare_draft_with_insight")[1]?.payload).toEqual({ pasted: TEXT, contextId: INSIGHT_CONTEXT_ID, includeCounts: true, includeOriginal: true, includeGoal: false });
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(screen.getByText("本次模型请求包含已勾选的匿名 A2 派生计数。")).toBeVisible();
    expect(screen.getByText(/完整请求 UTF-8 JSON：1200 字节/)).toBeVisible();
    expect(native.callsTo("generate_draft")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "确认，开始生成" })); await screen.findByRole("heading", { name: "草稿" });
    expect(native.callsTo("generate_draft")).toHaveLength(1);
    expect(screen.getByRole("heading", { name: "这份准备实际保留的统计材料" })).toBeVisible();
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled();
  });

  it("默认本地统计不入wire，最终展示采用核心reference/bytes/body而不保留旧副本", async () => {
    const reference = anInsightReference({ summary: aPersonSummary({ text: "最终完整可信统计材料", points: [{ ...aPersonSummary().points[0]!, statement: "最终统计完整项", evidence_ids: [] }] }) });
    const { native, user } = await page({ preparingDraftInsight: () => anInsightDraftPlan({ reference,
      prepared: { plan: anE1Plan(), includes_counts: false, includes_goal: false, encoded_bytes: 65432, body_preview: "FINAL LOCAL REFERENCE MARKER" } }) });
    await preview(user); const oldText = anInsightReference().summary.text;
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await screen.findByRole("heading", { name: "这份准备实际保留的统计材料" });
    expect(native.callsTo("prepare_draft_with_insight")[0]?.payload).toMatchObject({ includeCounts: false });
    expect(screen.queryByText(oldText)).toBeNull(); expect(screen.getAllByText("最终完整可信统计材料")).toHaveLength(2);
    expect(screen.getByText(/未获得统计计数/)).toBeVisible(); expect(screen.getByText(/完整请求 UTF-8 JSON：65432 字节/)).toBeVisible();
    await user.click(screen.getByText("查看实际脱敏统计材料")); expect(screen.getByText("FINAL LOCAL REFERENCE MARKER")).toBeVisible();
    expect(screen.getByRole("checkbox", { name: /匿名 A2 派生计数/ })).not.toBeChecked();
    await user.click(screen.getByRole("button", { name: "确认，开始生成" })); await screen.findByRole("heading", { name: "草稿" });
    expect(screen.queryByText(oldText)).toBeNull(); expect(screen.getAllByText("最终统计完整项")).toHaveLength(2);
  });

  it.each(["source", "paste", "unmount"] as const)("%s改变拒收迟到的统计preview，不恢复旧用途", async (change) => {
    const pending = deferred<InsightContextView>(); const { native, user, unmount } = await page({ preparingInsight: () => pending.promise });
    await user.click(screen.getByRole("button", { name: "预览本次统计与出处" }));
    await waitFor(() => expect(native.callsTo("prepare_insight_context")).toHaveLength(1));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "paste") fireEvent.change(screen.getByLabelText("原文"), { target: { value: "合成新原文" } });
    else unmount();
    await act(async () => { pending.resolve(anInsightContext()); await pending.promise; }); await idle();
    expect(screen.queryByRole("heading", { name: "本次统计材料出处" })).toBeNull();
    expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(0); expect(native.callsTo("generate_draft")).toHaveLength(0);
  });

  it("preview点击后、工厂微任务前发生source变更时不会自动刷新授权", async () => {
    const { native } = await page(); let mutation: Promise<unknown> = Promise.resolve();
    const button = screen.getByRole("button", { name: "预览本次统计与出处" });
    act(() => { button.click(); mutation = setUserEndpoint("http://127.0.0.1:9"); });
    await act(async () => { await mutation; }); await idle();
    expect(native.callsTo("prepare_insight_context")).toHaveLength(0);
    expect(screen.queryByRole("heading", { name: "本次统计材料出处" })).toBeNull();
  });

  it.each(["source", "paste"] as const)("%s已变但旧checkbox尚在DOM时不能复活旧统计出处", async (change) => {
    const { native, user } = await page(); await preview(user);
    const checkbox = screen.getByRole("checkbox", { name: /匿名 A2 派生计数/ }); let mutation: Promise<unknown> = Promise.resolve();
    act(() => {
      if (change === "source") mutation = setUserEndpoint("http://127.0.0.1:9");
      else fireEvent.change(screen.getByLabelText("原文"), { target: { value: "合成新原文" } });
      checkbox.click();
    });
    await act(async () => { await mutation; }); await idle();
    expect(screen.queryByRole("heading", { name: "本次统计材料出处" })).toBeNull();
    expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeDisabled();
    expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(0);
  });

  it("同operation从A换B：旧A preview只收尾，discard物理完成后才能读取B", async () => {
    const pending = deferred<InsightContextView>(); const drain = deferred<boolean>(); let delay = false; let previews = 0;
    const { native, user } = await page({ discardingDraft: () => delay ? drain.promise : true,
      preparingInsight: (request) => ++previews === 1 ? pending.promise : anInsightContext({ reference: anInsightReference({ contact_id: request.contact_id,
        summary: aPersonSummary({ contact_id: request.contact_id, text: "当前B统计" }) }) }) });
    await user.click(screen.getByRole("button", { name: "预览本次统计与出处" })); await waitFor(() => expect(previews).toBe(1)); delay = true;
    act(() => draftInsightSelection.select(LOCAL_INSIGHT_ID, B));
    await act(async () => { pending.resolve(anInsightContext()); await pending.promise; });
    await waitFor(() => expect(native.callsTo("discard_draft").length).toBeGreaterThanOrEqual(2));
    expect(screen.queryByRole("heading", { name: "本次统计材料出处" })).toBeNull();
    expect(screen.getByRole("button", { name: "预览本次统计与出处" })).toBeDisabled(); expect(previews).toBe(1);
    await act(async () => { drain.resolve(true); await drain.promise; }); await idle(); await preview(user);
    expect(screen.getByText("当前B统计")).toBeVisible();
    expect(native.callsTo("prepare_insight_context").map((call) => call.payload)).toEqual([
      { request: { bundle_id: LOCAL_INSIGHT_ID, contact_id: A } }, { request: { bundle_id: LOCAL_INSIGHT_ID, contact_id: B } },
    ]);
  });

  it("memory旧preview迟到不能冒充insight，反向选择memory会移除统计意图", async () => {
    const pending = deferred<MemoryContextView>(); const native = installFakeCore({ preparingMemory: () => pending.promise });
    draftMemoryCandidates.add(DRAFT_MEMORY_ID); await idle(); const user = userEvent.setup(); render(<Draft />);
    fireEvent.change(await screen.findByLabelText("原文"), { target: { value: TEXT } });
    await user.click(screen.getByRole("button", { name: "预览本次记忆与出处" }));
    await waitFor(() => expect(native.callsTo("prepare_memory_context")).toHaveLength(1));
    act(() => draftInsightSelection.select(LOCAL_INSIGHT_ID, A));
    await act(async () => { pending.resolve(aMemoryContextView()); await pending.promise; }); await idle();
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled(); await preview(user);
    act(() => draftMemoryCandidates.add(DRAFT_MEMORY_ID)); await idle();
    expect(draftInsightSelection.snapshot()).toBeNull(); expect(screen.queryByRole("heading", { name: "本次统计材料出处" })).toBeNull();
    expect(screen.getByRole("heading", { name: "本次手选记忆参考" })).toBeVisible(); expect(native.callsTo("generate_draft")).toHaveLength(0);
  });

  it("insight端点prepare迟到且已换memory时不留下旧approval或释放discard槽", async () => {
    const pending = deferred<InsightDraftPlan>(); const drain = deferred<boolean>(); let delay = false;
    const { native, user } = await page({ preparingDraftInsight: () => pending.promise, discardingDraft: () => delay ? drain.promise : true });
    await preview(user); await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await waitFor(() => expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(1)); delay = true;
    act(() => draftMemoryCandidates.add(DRAFT_MEMORY_ID));
    await act(async () => { pending.resolve(anInsightDraftPlan()); await pending.promise; });
    await waitFor(() => expect(screen.getByRole("button", { name: "预览本次记忆与出处" })).toBeDisabled());
    expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull(); expect(native.callsTo("generate_draft")).toHaveLength(0);
    await act(async () => { drain.resolve(true); await drain.promise; }); await idle();
    expect(screen.getByRole("button", { name: "预览本次记忆与出处" })).toBeEnabled();
  });

  it("已显示quote换memory后，尚未卸下的旧确认按钮也不能派发", async () => {
    const { native, user } = await page(); await preview(user);
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    const approve = await screen.findByRole("button", { name: "确认，开始生成" });
    act(() => { draftMemoryCandidates.add(DRAFT_MEMORY_ID); approve.click(); }); await idle();
    expect(native.callsTo("generate_draft")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "本次统计材料出处" })).toBeNull();
  });

  it("整项统计预算拒绝清除旧quote与最终材料，不保留可误确认旧准备或静默fallback", async () => {
    let preparations = 0;
    const { native, user } = await page({ preparingDraftInsight: () => {
      if (++preparations > 1) throw { reason_code: "INSIGHT_CONTEXT_CHANGED", explanation: "完整统计超出最终 JSON 预算，整项拒绝。" };
      return anInsightDraftPlan();
    } });
    await preview(user); await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await user.click(await screen.findByTestId("e1-carry-original"));
    expect(await screen.findByTestId("refusal-code")).toHaveTextContent("INSIGHT_CONTEXT_CHANGED");
    expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "这份准备实际保留的统计材料" })).toBeNull();
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled(); expect(native.callsTo("prepare_draft")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "清除本次统计，使用普通起草" })); await idle();
    await user.click(screen.getByRole("button", { name: "写一版草稿" })); await screen.findByRole("heading", { name: "草稿" });
    expect(native.callsTo("draft_reply")).toHaveLength(1); expect(native.callsTo("generate_draft")).toHaveLength(0);
  });
});
