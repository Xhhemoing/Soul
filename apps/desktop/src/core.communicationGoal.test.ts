import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";
import * as core from "./core";
import { anAssistanceDetail, anInsightDraftPlan, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => { vi.mocked(invoke).mockReset(); });

it("目标许可默认false，四种目标/统计组合分别传输，不借用原文许可", async () => {
  vi.mocked(invoke).mockResolvedValue(anInsightDraftPlan());
  await core.prepareDraftWithInsight("synthetic", "context");
  expect(invoke).toHaveBeenLastCalledWith("prepare_draft_with_insight", {
    pasted: "synthetic", contextId: "context", includeCounts: false, includeOriginal: false, includeGoal: false,
  });
  for (const [counts, goal] of [[false, false], [false, true], [true, false], [true, true]] as const) {
    await core.prepareDraftWithInsight("synthetic", "context", counts, true, goal);
    expect(invoke).toHaveBeenLastCalledWith("prepare_draft_with_insight", {
      pasted: "synthetic", contextId: "context", includeCounts: counts, includeOriginal: true, includeGoal: goal,
    });
  }
});

it("目标持久编辑在来源版本变化后拒收迟到的共同准备", async () => {
  let resolve!: (value: core.InsightDraftPlan) => void;
  const pending = new Promise<core.InsightDraftPlan>((done) => { resolve = done; });
  vi.mocked(invoke).mockImplementation((command) =>
    (command === "prepare_draft_with_insight" ? pending : Promise.resolve(anAssistanceDetail())) as Promise<never>);
  const result = core.prepareDraftWithInsight("synthetic", "context", true, false, true);
  const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  const revision = core.getDraftSourceRevision();
  await core.editAssistanceGoal({ item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION,
    title: "合成新目标", summary: "合成新约束", deadline_at: null });
  expect(core.getDraftSourceRevision()).not.toBe(revision);
  resolve(anInsightDraftPlan());
  await rejected;
});
