import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it } from "vitest";
import { editAssistanceGoal, type InsightDraftPlan, type MemorySelectionPreview } from "../core";
import { aMemorySelection, aSelectedMemory, anInsightContext, anInsightDraftPlan, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { Draft } from "./Draft";
import { draftAssistanceSelection, draftInsightSelection, draftMemoryCandidates } from "./draftMaterialSelection";
import { draftLane } from "./draftRequests";

const pointer = { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION };
const goalName = /本次另行允许.*本人目标/;
const countsName = /本次另行允许将这份匿名派生计数/;
const goal = aMemorySelection({ selected: [aSelectedMemory({ memory_type: "commitment", title: "合成协调目标", summary: "只约周五下午，不承诺其他时段。" })] });
async function idle() { await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); }
beforeEach(async () => {
  installFakeCore(); draftAssistanceSelection.clear(); draftInsightSelection.clear();
  for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id);
  await idle(); window.location.hash = "#/draft";
});
afterEach(async () => { cleanup(); draftAssistanceSelection.clear(); await idle(); });
async function page(preview: MemorySelectionPreview | null = goal, options: FakeCoreOptions = {}) {
  const native = installFakeCore({ preparingAssistance: () => ({ item_id: ASSISTANCE_ITEM_ID,
    revision: ASSISTANCE_REVISION, kind: "insight", insight: anInsightContext(), goal_preview: preview }), ...options });
  draftAssistanceSelection.select(pointer); await idle();
  const user = userEvent.setup(); const view = render(<Draft />);
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: "合成待回复消息" } });
  await user.click(screen.getByRole("button", { name: "核对并准备" })); await idle();
  return { native, user, ...view };
}
async function edit() {
  await editAssistanceGoal({ ...pointer, title: "持久编辑后的目标", summary: "新约束", deadline_at: null });
}

it.each([[false, false], [false, true], [true, false], [true, true]] as const)(
  "统计%s目标%s独立许可且初始都关闭", async (counts, includeGoal) => {
    const { native, user } = await page();
    const goalBox = screen.getByRole("checkbox", { name: goalName });
    const countsBox = screen.getByRole("checkbox", { name: countsName });
    expect(goalBox).not.toBeChecked(); expect(countsBox).not.toBeChecked();
    if (includeGoal) await user.click(goalBox);
    if (counts) await user.click(countsBox);
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(native.callsTo("prepare_draft_with_insight")[0]?.payload).toMatchObject({ includeCounts: counts, includeGoal, includeOriginal: false });
    expect(native.callsTo("generate_draft")).toHaveLength(0);
    await user.click(screen.getByTestId("e1-carry-original"));
    await waitFor(() => expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(2));
    expect(native.callsTo("prepare_draft_with_insight")[1]?.payload).toMatchObject({ includeCounts: counts, includeGoal, includeOriginal: true });
  });

it.each([null, aMemorySelection({ selected: [aSelectedMemory({ privacy_class: "mixed" })] }),
  aMemorySelection({ selected: [], excluded: [{ memory_id: "oversized-goal", reason: "budget" }], outcome: "abstain" })])(
  "没有可纳入本人目标时不提供许可", async (preview) => {
    const { native, user } = await page(preview);
    expect(screen.queryByRole("checkbox", { name: goalName })).toBeNull();
    await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(native.callsTo("prepare_draft_with_insight")[0]?.payload).toMatchObject({ includeGoal: false });
  });

it("重新核对目标清除两项许可", async () => {
  const { user } = await page();
  await user.click(screen.getByRole("checkbox", { name: goalName }));
  await user.click(screen.getByRole("checkbox", { name: countsName }));
  await user.click(screen.getByRole("button", { name: "核对并准备" })); await idle();
  expect(screen.getByRole("checkbox", { name: goalName })).not.toBeChecked();
  expect(screen.getByRole("checkbox", { name: countsName })).not.toBeChecked();
});

it("目标编辑后旧DOM不能批准或恢复目标许可", async () => {
  const { native, user } = await page();
  const oldGoal = screen.getByRole("checkbox", { name: goalName }); await user.click(oldGoal);
  await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
  const approve = await screen.findByRole("button", { name: "确认，开始生成" });
  await act(async () => { const changed = edit(); approve.click(); oldGoal.click(); await changed; }); await idle();
  expect(native.callsTo("generate_draft")).toHaveLength(0);
  expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
  expect(screen.queryByRole("checkbox", { name: goalName })).toBeNull();
});

it("目标编辑拒收迟到prepare且等待物理请求结束", async () => {
  let resolve!: (value: InsightDraftPlan) => void;
  const pending = new Promise<InsightDraftPlan>((done) => { resolve = done; });
  const { native, user } = await page(goal, { preparingDraftInsight: () => pending });
  await user.click(screen.getByRole("checkbox", { name: goalName }));
  await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
  await waitFor(() => expect(native.callsTo("prepare_draft_with_insight")).toHaveLength(1));
  await act(edit); expect(draftLane.snapshot().busy).toBe(true);
  await act(async () => { resolve(anInsightDraftPlan()); await pending; }); await idle();
  expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
  expect(screen.queryByRole("checkbox", { name: goalName })).toBeNull();
  expect(native.callsTo("generate_draft")).toHaveLength(0);
});

it("最终目标展示采用core实际纳入预览而非最初目标副本", async () => {
  const finalGoal = aMemorySelection({ selected: [aSelectedMemory({ title: "核心最终目标", summary: "核心实际保留的目标约束" })] });
  const plan = { ...anInsightDraftPlan(), goal_preview: finalGoal,
    prepared: { ...anInsightDraftPlan().prepared, includes_goal: true, body_preview: "ACTUAL_REDACTED_JOINT_BODY" } };
  const { user } = await page(goal, { preparingDraftInsight: () => plan });
  await user.click(screen.getByRole("checkbox", { name: goalName }));
  await user.click(screen.getByRole("button", { name: "用你自己的模型端点写" }));
  await screen.findByRole("heading", { name: "确认这一次要生成什么" });
  expect(screen.getAllByText("核心实际保留的目标约束").length).toBeGreaterThan(0);
  expect(screen.queryByText("只约周五下午，不承诺其他时段。")).toBeNull();
  await user.click(screen.getByText("查看实际脱敏统计材料"));
  expect(screen.getByText("ACTUAL_REDACTED_JOINT_BODY")).toBeVisible();
});
