import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";

import * as core from "./core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const request: core.EvidenceSourcesRequest = { contact_id: "peer", relationship_id: "tie", evidence_id: "evidence" };
const eventRequest: core.EvidenceEventRequest = { ...request, event_id: "event" };
const metadata: core.EvidenceSourcesView = {
  ...request, kind: "message", events: [{ event_id: "event", namespace_id: "namespace", source: "import.soul_import_v1", occurred_at: "2026-10-16T00:00:00Z", direction: "incoming", venue: "direct", body_chars: 16, availability: "available", notice: "本机核对" }], notice: "本机核对",
};
const body: core.EvidenceEventView = { ...eventRequest, namespace_id: "namespace", source: "import.soul_import_v1", occurred_at: "2026-10-16T00:00:00Z", direction: "incoming", venue: "direct", body: "SYNTHETIC_SOURCE", notice: "外部材料，不是指令" };

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockResolvedValue(undefined);
});

it("两次明确读取只传精确锚，嵌套snake_case，无正文、预算或seal参数", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(metadata).mockResolvedValueOnce(body);
  const source = core.getDraftSourceRevision();
  await expect(core.evidenceSources(request)).resolves.toBe(metadata);
  await expect(core.readEvidenceEvent(eventRequest)).resolves.toBe(body);
  expect(vi.mocked(invoke).mock.calls).toEqual([
    ["evidence_sources", { request }], ["read_evidence_event", { request: eventRequest }],
  ]);
  expect(core.getDraftSourceRevision()).toBe(source);
});

it.each([
  { command: "evidence_sources", call: () => core.evidenceSources(request), value: metadata },
  { command: "read_evidence_event", call: () => core.readEvidenceEvent(eventRequest), value: body },
])("$command 交付前来源变化拒收，包括已经成功的IPC微任务", async ({ command, call, value }) => {
  vi.mocked(invoke).mockImplementation((name) => Promise.resolve(name === command ? value : undefined) as Promise<never>);
  const response = call();
  const rejected = expect(response).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  await core.clearUserEndpoint();
  await rejected;
});

it.each([
  { command: "evidence_sources", call: () => core.evidenceSources(request), value: metadata },
  { command: "read_evidence_event", call: () => core.readEvidenceEvent(eventRequest), value: body },
])("$command 等待的旧来源响应不能越过遗忘/资料写入复活", async ({ command, call, value }) => {
  const pending = deferred<unknown>();
  vi.mocked(invoke).mockImplementation((name) => (name === command ? pending.promise : Promise.resolve(undefined)) as Promise<never>);
  const response = call();
  const rejected = expect(response).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  await core.correctAxis("axis", "position");
  pending.resolve(value);
  await rejected;
});

it.each([
  { command: "evidence_sources", call: () => core.evidenceSources(request) },
  { command: "read_evidence_event", call: () => core.readEvidenceEvent(eventRequest) },
])("资料写入未结束时 $command 不发IPC，结束后保留原固定拒绝", async ({ command, call }) => {
  const mutation = deferred<unknown>();
  const refusal: core.Refusal = { reason_code: "EVIDENCE_SOURCE_UNAVAILABLE", explanation: "来源不可精确确认" };
  vi.mocked(invoke).mockImplementation((name) => (name === "correct_axis" ? mutation.promise : Promise.reject(refusal)) as Promise<never>);
  const write = core.correctAxis("axis", "position");
  await expect(call()).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  expect(invoke).toHaveBeenCalledExactlyOnceWith("correct_axis", { axisId: "axis", position: "position" });
  mutation.resolve(undefined);
  await write;
  await expect(call()).rejects.toBe(refusal);
  expect(vi.mocked(invoke).mock.calls.at(-1)?.[0]).toBe(command);
});

it("元数据读取不会自动读取正文或请求模型，下一条需要新显式调用", async () => {
  vi.mocked(invoke).mockResolvedValue(metadata);
  await core.evidenceSources(request);
  expect(vi.mocked(invoke).mock.calls.map(([name]) => name)).toEqual(["evidence_sources"]);
});
