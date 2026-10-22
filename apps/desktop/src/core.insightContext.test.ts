import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";

import * as core from "./core";
import { anE1Plan, aTemplateDraft } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

const request: core.InsightContextRequest = { bundle_id: "bundle", contact_id: "contact" };
const reference: core.InsightReference = {
  bundle_id: "bundle", operation_id: "operation", namespace_id: "namespace",
  contact_id: "contact", relationship_id: "relationship",
  summary: { contact_id: "contact", source: "counts", text: "可信合成统计", points: [{ statement: "合成统计", evidence_ids: ["evidence"], band: "weak" }], notice: "工作假设，非临床结论", clinical_claim: false },
  sources: [{ evidence_id: "evidence", kind: "message", event_ids: ["event"] }],
  notice: "本次统计材料的出处，不代表生成草稿逐句已获事实验证。",
};
const selected: core.InsightContextView = { context_id: "context", reference };
const local: core.InsightDraftValue = { draft: aTemplateDraft(), reference };
const prepared: core.InsightDraftPlan = {
  prepared: { plan: anE1Plan(), body_preview: "本次统计仅供本地参考", encoded_bytes: 1234, includes_counts: false, includes_goal: false },
  reference,
  goal_preview: null,
};

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(undefined);
});

it("新统计预览撤销旧显示用途，只交ID，不接受UI统计正文", async () => {
  const initial = core.getDraftSourceRevision();
  vi.mocked(invoke).mockImplementation(() => {
    expect(core.getDraftSourceRevision()).not.toBe(initial);
    return Promise.resolve(selected) as Promise<never>;
  });
  await expect(core.prepareInsightContext(request)).resolves.toBe(selected);
  expect(invoke).toHaveBeenCalledExactlyOnceWith("prepare_insight_context", { request });
});

it("本机与端点准备使用外层camelCase，默认不给端点计数，也不确认paste原文", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(local).mockResolvedValueOnce(prepared).mockResolvedValueOnce(prepared);
  await expect(core.localDraftWithInsight("synthetic", "context")).resolves.toBe(local);
  await expect(core.prepareDraftWithInsight("synthetic", "context")).resolves.toBe(prepared);
  await core.prepareDraftWithInsight("synthetic", "context", true, true);
  expect(vi.mocked(invoke).mock.calls).toEqual([
    ["local_draft_with_insight", { pasted: "synthetic", contextId: "context" }],
    ["prepare_draft_with_insight", { pasted: "synthetic", contextId: "context", includeCounts: false, includeOriginal: false, includeGoal: false }],
    ["prepare_draft_with_insight", { pasted: "synthetic", contextId: "context", includeCounts: true, includeOriginal: true, includeGoal: false }],
  ]);
});

it.each([
  { command: "prepare_insight_context", call: () => core.prepareInsightContext(request), value: selected },
  { command: "local_draft_with_insight", call: () => core.localDraftWithInsight("synthetic", "context"), value: local },
  { command: "prepare_draft_with_insight", call: () => core.prepareDraftWithInsight("synthetic", "context"), value: prepared },
])("$command 交付前资料变化拒收统计、引用与批准", async ({ command, call, value }) => {
  const pending = deferred<unknown>();
  vi.mocked(invoke).mockImplementation((name) => (name === command ? pending.promise : Promise.resolve(undefined)) as Promise<never>);
  const result = call();
  const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  await core.setVoice("length", "short");
  pending.resolve(value);
  await rejected;
});

it("同operation的新联系人选择也拒收较早的预览", async () => {
  const first = deferred<core.InsightContextView>();
  vi.mocked(invoke).mockReturnValueOnce(first.promise).mockResolvedValueOnce(selected);
  const earlier = core.prepareInsightContext(request);
  const rejected = expect(earlier).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  await core.prepareInsightContext({ ...request, contact_id: "another-contact" });
  first.resolve(selected);
  await rejected;
});

it.each([
  { command: "ignore_local_insight", call: () => core.ignoreLocalInsight("bundle") },
  { command: "recompute_local_insight", call: () => core.recomputeLocalInsight("bundle") },
])("$command 在写入前后失效，并挡住并行统计准入", async ({ command, call }) => {
  const pending = deferred<unknown>();
  vi.mocked(invoke).mockReturnValue(pending.promise);
  const initial = core.getDraftSourceRevision();
  const mutation = call();
  expect(core.getDraftSourceRevision()).not.toBe(initial);
  await expect(core.prepareInsightContext(request)).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  expect(invoke).toHaveBeenCalledExactlyOnceWith(command, { bundleId: "bundle" });
  const beforeComplete = core.getDraftSourceRevision();
  pending.resolve(undefined);
  await mutation;
  expect(core.getDraftSourceRevision()).not.toBe(beforeComplete);
});
