import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as core from "./core";
import { anE1Plan, aTemplateDraft } from "./test/fakeCore";

// Only the native transport is replaced. Every wrapper and its promise
// continuation below is the production core.ts implementation.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

interface Deferred<T> {
  readonly promise: Promise<T>;
  resolve(value: T): void;
  reject(reason: unknown): void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

const mutations: readonly {
  readonly command: string;
  readonly call: () => Promise<unknown>;
  readonly args?: Record<string, unknown>;
}[] = [
  {
    command: "complete_wizard",
    call: () => core.completeWizard({ acknowledged_defaults_are_off: true }),
    args: { answers: { acknowledged_defaults_are_off: true } },
  },
  {
    command: "correct_tie", call: () => core.correctTie("tie", "near"),
    args: { relationshipId: "tie", band: "near" },
  },
  { command: "release_tie", call: () => core.releaseTie("tie"), args: { relationshipId: "tie" } },
  { command: "discard_draft", call: () => core.discardDraft() },
  {
    command: "set_user_endpoint", call: () => core.setUserEndpoint("http://127.0.0.1:8123"),
    args: { url: "http://127.0.0.1:8123" },
  },
  { command: "clear_user_endpoint", call: () => core.clearUserEndpoint() },
  {
    command: "authorize_directory", call: () => core.authorizeDirectory("/synthetic"),
    args: { path: "/synthetic" },
  },
  {
    command: "commit_soul_import_v1", call: () => core.commitSoulImportV1("synthetic export"),
    args: { text: "synthetic export" },
  },
  {
    command: "commit_telegram", call: () => core.commitTelegram("synthetic export"),
    args: { text: "synthetic export" },
  },
  {
    command: "answer_questionnaire", call: () => core.answerQuestionnaire([]),
    args: { answers: [] },
  },
  {
    command: "correct_axis", call: () => core.correctAxis("axis", "position"),
    args: { axisId: "axis", position: "position" },
  },
  {
    command: "set_voice", call: () => core.setVoice("length", "short"),
    args: { field: "length", option: "short" },
  },
  {
    command: "create_memory",
    call: () => core.createMemory({ memory_type: "event", title: "synthetic", summary: "fixture" }),
    args: { memory: { memory_type: "event", title: "synthetic", summary: "fixture" } },
  },
  {
    command: "update_memory", call: () => core.updateMemory("memory", { summary: "changed" }),
    args: { memoryId: "memory", change: { summary: "changed" } },
  },
  {
    command: "forget_memory",
    call: () => core.forgetMemory({ preview_id: "preview", memory_id: "memory" }),
    args: { confirmation: { preview_id: "preview", memory_id: "memory" } },
  },
  { command: "grant_collect_consent", call: () => core.grantCollectConsent() },
  { command: "revoke_collect_consent", call: () => core.revokeCollectConsent() },
];

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(undefined);
});

describe("起草结果在单 WebView 内的来源归属", () => {
  it.each(mutations)("$command 在 IPC 前同步失效，结束后再次失效，参数不变", async (mutation) => {
    const pending = deferred<unknown>();
    const initial = core.getDraftSourceRevision();
    const seen: core.DraftSourceRevision[] = [];
    const unsubscribe = core.subscribeDraftSource((revision) => seen.push(revision));
    try {
      expect(seen).toEqual([initial]);
      vi.mocked(invoke).mockImplementation(() => {
        expect(core.getDraftSourceRevision()).not.toBe(initial);
        expect(seen.at(-1)).toBe(core.getDraftSourceRevision());
        return pending.promise as Promise<never>;
      });
      const request = mutation.call();
      const args = ["update_memory", "forget_memory"].includes(mutation.command)
        ? { ...mutation.args, correlationId: expect.stringMatching(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i) }
        : mutation.args;
      expect(invoke).toHaveBeenCalledExactlyOnceWith(mutation.command, args);
      const started = core.getDraftSourceRevision();
      expect(seen).toHaveLength(2);
      pending.resolve({ acknowledged: true });
      await expect(request).resolves.toEqual({ acknowledged: true });
      expect(core.getDraftSourceRevision()).not.toBe(started);
      expect(seen).toHaveLength(3);
    } finally {
      unsubscribe();
    }
  });

  it("解除订阅后不再收到变更", async () => {
    const listener = vi.fn();
    const unsubscribe = core.subscribeDraftSource(listener);
    expect(listener).toHaveBeenCalledTimes(1);
    unsubscribe();
    await core.clearUserEndpoint();
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it.each([
    { command: "draft_reply", call: () => core.draftReply("synthetic"), value: aTemplateDraft() },
    { command: "prepare_draft", call: () => core.prepareDraft("synthetic"), value: anE1Plan() },
    { command: "generate_draft", call: () => core.generateDraft(anE1Plan()), value: aTemplateDraft() },
  ])("$command 的 IPC 已成功，交付微任务前来源变化仍拒收", async ({ command, call, value }) => {
    vi.mocked(invoke).mockImplementation((name) => Promise.resolve(name === command ? value : undefined) as Promise<never>);
    // The transport promise already contains a successful value. The native
    // result alone must not allow this continuation to revive stale content.
    const result = call();
    const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    await core.setVoice("length", "short");
    await rejected;
  });

  it("变更 IPC 失败也拒收旧准备，保留原错误且不伪造后端拒绝", async () => {
    const prepared = deferred<core.E1DraftPlan>();
    const changed = deferred<unknown>();
    const backendFailure = { reason_code: "STORE_UNAVAILABLE", explanation: "synthetic refusal" };
    vi.mocked(invoke).mockImplementation((command) => (
      command === "prepare_draft" ? prepared.promise : changed.promise
    ) as Promise<never>);
    const result = core.prepareDraft("synthetic");
    const rejectedResult = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    const update = core.updateMemory("memory", { summary: "changed" });
    const rejectedUpdate = expect(update).rejects.toBe(backendFailure);
    changed.reject(backendFailure);
    await rejectedUpdate;
    prepared.resolve(anE1Plan());
    await rejectedResult;
    const localError = new core.DraftSourceChangedError();
    expect(localError).not.toHaveProperty("reason_code");
    expect(localError.message).toContain("重新准备");
  });

  it.each(["success", "failure"] as const)("写请求等待时拒绝新准备，写入 %s 后才能重新准备", async (outcome) => {
    const changed = deferred<unknown>();
    const prepared = deferred<core.E1DraftPlan>();
    vi.mocked(invoke).mockImplementation((command) => (
      command === "prepare_draft" ? prepared.promise : changed.promise
    ) as Promise<never>);
    const change = core.correctAxis("axis", "position");
    const settled = change.catch(() => undefined);
    const result = core.prepareDraft("synthetic");
    const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    // Even if the draft transport could reply first, it is never admitted
    // while an earlier source mutation is waiting for native acknowledgement.
    prepared.resolve(anE1Plan());
    await rejected;
    expect(invoke).toHaveBeenCalledExactlyOnceWith("correct_axis", { axisId: "axis", position: "position" });
    if (outcome === "success") changed.resolve(undefined);
    else changed.reject(new Error("unknown IPC outcome"));
    await settled;
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });

  it.each([
    { command: "draft_reply", call: () => core.draftReply("synthetic"), value: aTemplateDraft() },
    { command: "prepare_draft", call: () => core.prepareDraft("synthetic"), value: anE1Plan() },
    { command: "generate_draft", call: () => core.generateDraft(anE1Plan()), value: aTemplateDraft() },
  ])("资料写入等待期间 $command 不发 IPC，写入结束后才能开始", async ({ command, call, value }) => {
    const change = deferred<unknown>();
    vi.mocked(invoke).mockImplementation((name) => (
      name === "correct_axis" ? change.promise : Promise.resolve(value)
    ) as Promise<never>);
    const mutation = core.correctAxis("axis", "position");
    await expect(call()).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    expect(vi.mocked(invoke).mock.calls.map(([name]) => name)).toEqual(["correct_axis"]);
    change.resolve(undefined);
    await mutation;
    await expect(call()).resolves.toBe(value);
    expect(vi.mocked(invoke).mock.calls.map(([name]) => name)).toEqual(["correct_axis", command]);
  });

  it("并发变更只有全部结束才允许新的准备，一个回执不会提前解禁", async () => {
    const axis = deferred<unknown>();
    const voice = deferred<unknown>();
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === "correct_axis") return axis.promise as Promise<never>;
      if (command === "set_voice") return voice.promise as Promise<never>;
      return Promise.resolve(anE1Plan()) as Promise<never>;
    });
    const first = core.correctAxis("axis", "position");
    const second = core.setVoice("length", "short");
    const rejectedSecond = expect(second).rejects.toEqual({ explanation: "unknown outcome" });
    axis.resolve(undefined);
    await first;
    await expect(core.prepareDraft("synthetic")).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    expect(vi.mocked(invoke).mock.calls.map(([name]) => name)).toEqual(["correct_axis", "set_voice"]);
    voice.reject({ explanation: "unknown outcome" });
    await rejectedSecond;
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });

  it("传输同步抛错也释放本地变更计数并保留原错误", async () => {
    const failure = new Error("transport unavailable");
    vi.mocked(invoke).mockImplementationOnce(() => { throw failure; });
    await expect(core.clearUserEndpoint()).rejects.toBe(failure);
    vi.mocked(invoke).mockResolvedValue(anE1Plan());
    await expect(core.prepareDraft("synthetic")).resolves.toEqual(anE1Plan());
  });

  it.each(["same origin", "A/B/A"] as const)("端点 %s 不会复活旧结果", async (pattern) => {
    const response = deferred<core.Draft>();
    vi.mocked(invoke).mockImplementation((command) => (
      command === "generate_draft" ? response.promise : Promise.resolve(undefined)
    ) as Promise<never>);
    await core.setUserEndpoint("http://127.0.0.1:8123");
    const result = core.generateDraft(anE1Plan());
    const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
    if (pattern === "A/B/A") await core.setUserEndpoint("http://127.0.0.1:8124");
    await core.setUserEndpoint("http://127.0.0.1:8123");
    response.resolve(aTemplateDraft());
    await rejected;
  });

  it("普通读取、只读预览和仅日志清理不会使准备失效", async () => {
    const response = deferred<core.E1DraftPlan>();
    vi.mocked(invoke).mockImplementation((command) => (
      command === "prepare_draft" ? response.promise : Promise.resolve(undefined)
    ) as Promise<never>);
    const initial = core.getDraftSourceRevision();
    const result = core.prepareDraft("synthetic");
    await Promise.all([
      core.configSnapshot(), core.sessionStatus(), core.cloudToggle(false),
      core.filesView(), core.previewPlan("/synthetic"),
      core.peopleGraph(), core.personSummary("person"), core.draftNotices(),
      core.previewSoulImportV1("synthetic"), core.previewTelegram("synthetic"),
      core.questionnaire(), core.profileScreen(), core.memoryList(), core.memoryDetail("memory"),
      core.previewForget("memory"), core.retryForgetCleanup(), core.researchPreview(),
      core.auditChain(), core.collectStatus(),
    ]);
    expect(core.getDraftSourceRevision()).toBe(initial);
    const plan = anE1Plan();
    response.resolve(plan);
    await expect(result).resolves.toBe(plan);
  });

  it("准备与原文二次确认不自行失效，批准参数原样交付", async () => {
    const plan = anE1Plan();
    const draft = aTemplateDraft();
    vi.mocked(invoke).mockImplementation((command) => Promise.resolve(
      command === "prepare_draft" ? plan : draft,
    ) as Promise<never>);
    const initial = core.getDraftSourceRevision();
    await expect(core.prepareDraft("synthetic")).resolves.toBe(plan);
    await expect(core.prepareDraft("synthetic", true)).resolves.toBe(plan);
    const approval = { preparation_id: plan.preparation_id, plan_hash: plan.plan_hash };
    await expect(core.generateDraft(approval)).resolves.toBe(draft);
    expect(core.getDraftSourceRevision()).toBe(initial);
    expect(invoke).toHaveBeenNthCalledWith(1, "prepare_draft", { pasted: "synthetic", includeOriginal: false });
    expect(invoke).toHaveBeenNthCalledWith(2, "prepare_draft", { pasted: "synthetic", includeOriginal: true });
    expect(invoke).toHaveBeenNthCalledWith(3, "generate_draft", { approval });
  });
});
