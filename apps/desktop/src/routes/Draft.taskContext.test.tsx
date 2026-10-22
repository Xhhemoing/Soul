/** Real Draft controls and core IPC wrapper; only the native boundary is synthetic. */
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { aMemoryContextView, aMemorySelection, DRAFT_MEMORY_ID, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { Draft } from "./Draft";
import { draftMemoryCandidates } from "./draftMemorySelection";
import { draftLane } from "./draftRequests";

const TASK = "周五会议\n需要确认：地点";
function matched() {
  return { memory: aMemoryContextView(), assessment: {
    status: "matched" as const, requirements: [{ field: "地点", status: "matched" as const, source_ids: [DRAFT_MEMORY_ID] }],
    supplemented_ids: [DRAFT_MEMORY_ID], question: null, notice: "仅核对明确字段的字面线索。",
  } };
}
async function idle() { await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); }
beforeEach(async () => { installFakeCore(); for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id); await idle(); });
afterEach(async () => { cleanup(); for (const id of draftMemoryCandidates.snapshot()) draftMemoryCandidates.remove(id); await idle(); });
async function page(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options); draftMemoryCandidates.add(DRAFT_MEMORY_ID); await idle();
  const view = render(<Draft />); const user = userEvent.setup();
  fireEvent.change(await screen.findByLabelText("原文"), { target: { value: "合成会议消息" } });
  fireEvent.change(screen.getByLabelText("本次需要核对的信息"), { target: { value: TASK } });
  await idle();
  return { core, user, ...view };
}

describe("任务所需信息与本次记忆", () => {
  it("提交明确范围和任务，展示补证出处且不自动授权本人正文或调用模型", async () => {
    const { core, user } = await page({ preparingTask: () => matched() });
    await user.click(screen.getByRole("button", { name: "识别信息要求并核对材料" }));
    await screen.findByText(/已从本次候选补入/);
    expect(core.callsTo("prepare_task_context")[0]?.payload).toEqual({ request: { candidate_ids: [DRAFT_MEMORY_ID], task: TASK } });
    expect(screen.getByText(/地点：找到字面线索/)).toBeVisible();
    expect(screen.getByRole("checkbox", { name: /本次另行允许/ })).not.toBeChecked();
    expect(core.callsTo("prepare_memory_context")).toHaveLength(0);
    expect(core.callsTo("generate_draft")).toHaveLength(0);
  });
  it("缺口只显示针对问题并阻止带记忆起草，更新信息要求会清除旧诊断", async () => {
    const { user } = await page({ preparingTask: () => ({
      memory: aMemoryContextView({ context_id: null, preview: aMemorySelection({ selected: [], outcome: "clarify" }) }),
      assessment: { status: "clarify", requirements: [{ field: "地点", status: "missing", source_ids: [] }], supplemented_ids: [], question: "本次已检查的材料未给出地点，请确认地点。", notice: "限定本次范围。" },
    }) });
    await user.click(screen.getByRole("button", { name: "识别信息要求并核对材料" }));
    await screen.findByText("本次已检查的材料未给出地点，请确认地点。");
    expect(screen.getByRole("button", { name: "写一版草稿" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "用你自己的模型端点写" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText("本次需要核对的信息"), { target: { value: "需要确认：时间" } });
    await idle();
    expect(screen.queryByText("本次已检查的材料未给出地点，请确认地点。")).toBeNull();
  });
  it("不支持的输入显示未确定，不把未知改成澄清问题", async () => {
    const { user } = await page({ preparingTask: () => ({
      memory: aMemoryContextView({ context_id: null, preview: aMemorySelection({ selected: [], outcome: "abstain" }) }),
      assessment: { status: "unknown", requirements: [], supplemented_ids: [], question: null, notice: "当前无法识别这段任务的信息要求。" },
    }) });
    await user.click(screen.getByRole("button", { name: "识别信息要求并核对材料" }));
    await screen.findByText("核对状态：未确定");
    expect(screen.queryByRole("heading", { name: "需要你确认" })).toBeNull();
  });
  it("原文在核对期间改变后，迟到结果不能恢复旧材料或诊断", async () => {
    let resolve!: (value: ReturnType<typeof matched>) => void;
    const pending = new Promise<ReturnType<typeof matched>>((yes) => { resolve = yes; });
    const { core, user } = await page({ preparingTask: () => pending });
    await user.click(screen.getByRole("button", { name: "识别信息要求并核对材料" }));
    await waitFor(() => expect(core.callsTo("prepare_task_context")).toHaveLength(1));
    fireEvent.change(screen.getByLabelText("原文"), { target: { value: "另一项合成任务" } });
    await act(async () => { resolve(matched()); await pending; }); await idle();
    expect(screen.queryByText(/已从本次候选补入/)).toBeNull();
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
  });
  it("拒绝诊断带回本次范围外的来源编号", async () => {
    const { user } = await page({ preparingTask: () => ({ ...matched(), assessment: { ...matched().assessment,
      requirements: [{ field: "地点", status: "matched", source_ids: ["0192f000-0000-7000-8000-00000000ffff"] }],
    } }) });
    await user.click(screen.getByRole("button", { name: "识别信息要求并核对材料" }));
    await screen.findByRole("alert"); await idle();
    expect(screen.queryByText(/已从本次候选补入/)).toBeNull();
    expect(screen.queryByRole("heading", { name: "合成图书馆偏好" })).toBeNull();
  });
});
