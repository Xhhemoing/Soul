/** Exact local source clicks through real Graph/App and IPC wrappers. */
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import { setUserEndpoint, type EvidenceEventView, type EvidenceSourcesView } from "../core";
import { aLocalInsight, aPeopleGraph, aPersonSummary, anEvidenceEvent, anEvidenceEventSource, anEvidenceSources,
  EVIDENCE_EVENT_ID, forbidNetwork, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { Graph } from "./Graph";
import { graphTargetHash } from "./graphTarget";

const graph = aPeopleGraph(); const contact = graph.people[1]!; const tie = graph.ties[0]!;
const E1 = tie.evidence[0]!.evidence_id; const E2 = tie.evidence[1]!.evidence_id;
const EVENT2 = "0192f000-0000-7000-8000-00000000ea32";
const target = graphTargetHash(contact.contact_id, [E1, E2])!;
const anchors = { contact_id: contact.contact_id, relationship_id: tie.relationship_id, evidence_id: E1 };
const metadataName = (id: string) => `核对这条证据的本地来源：${id}`;
const rawName = (id = EVIDENCE_EVENT_ID) => `在本机显示这一条敏感内容：${id}`;
function deferred<T>() { let resolve!: (value: T) => void; let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
function navigate(hash: string) { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); }
afterEach(() => { vi.restoreAllMocks(); });
async function page(options: FakeCoreOptions = {}) {
  const native = installFakeCore({ graph, ...options }); const user = userEvent.setup(); window.location.hash = target;
  const view = render(<Graph />); const button = await screen.findByRole("button", { name: metadataName(E1) });
  return { native, user, button, ...view };
}
async function metadata(user: ReturnType<typeof userEvent.setup>, id = E1) {
  await user.click(screen.getByRole("button", { name: metadataName(id) }));
  return screen.findByRole("button", { name: rawName() });
}

describe("明确单条本地证据来源查看", () => {
  it("App从洞察定位后键盘点击metadata仍不开正文，再明确点击单event只显示plain text且零E1", async () => {
    const network = forbidNetwork(); const text = '<img src="http://127.0.0.1:9/a"> <script>throw 1</script> http://127.0.0.1:9/raw';
    const native = installFakeCore({ graph, insights: [aLocalInsight({ summaries: [aPersonSummary()] })], readingEvidenceEvent: (request) => anEvidenceEvent({ ...request, body: text }) });
    const user = userEvent.setup(); window.location.hash = "#/import"; render(<App />);
    await user.click(await screen.findByRole("link", { name: "核对此联系人的证据条目（元数据）" }));
    const button = await screen.findByRole("button", { name: metadataName(E1) });
    expect(native.callsTo("evidence_sources")).toHaveLength(0); expect(native.callsTo("read_evidence_event")).toHaveLength(0);
    act(() => button.focus()); await user.keyboard("{Enter}");
    const raw = await screen.findByRole("button", { name: rawName() });
    expect(native.callsTo("evidence_sources")[0]?.payload).toEqual({ request: anchors });
    expect(native.callsTo("read_evidence_event")).toHaveLength(0); expect(screen.queryByTestId("evidence-event-body")).toBeNull();
    await user.click(raw); expect(await screen.findByTestId("evidence-event-body")).toHaveTextContent(text);
    expect(native.callsTo("read_evidence_event")[0]?.payload).toEqual({ request: { ...anchors, event_id: EVIDENCE_EVENT_ID } });
    const viewer = screen.getByRole("heading", { name: "本地单条来源查看" }).closest("section")!;
    expect(within(viewer).queryByRole("img")).toBeNull(); expect(within(viewer).queryByRole("link")).toBeNull(); expect(viewer.querySelector("script")).toBeNull();
    expect(within(viewer).queryByRole("button", { name: /复制|导出|起草|记忆/ })).toBeNull();
    expect(native.callsTo("person_summary")).toHaveLength(0); expect(native.callsTo("generate_draft")).toHaveLength(0); expect(native.callsTo("prepare_draft")).toHaveLength(0); expect(network).toEqual([]);
    await user.click(screen.getByRole("button", { name: "关闭并清除本次来源展示" }));
    expect(screen.queryByTestId("evidence-event-body")).toBeNull(); expect(screen.queryByRole("heading", { name: "本地单条来源查看" })).toBeNull(); expect(button).toHaveFocus();
  });

  it("没有正文来源的纠正条目只显示明确notice，不改读别条", async () => {
    const { native, user, button } = await page({ evidenceSources: (request) => anEvidenceSources({ ...request, kind: "user_correction", events: [], notice: "合成纠正没有原始正文。" }) });
    await user.click(button); expect(await screen.findByText("合成纠正没有原始正文。")).toBeVisible();
    expect(screen.queryByRole("button", { name: /在本机显示这一条敏感内容/ })).toBeNull(); expect(native.callsTo("read_evidence_event")).toHaveLength(0);
  });

  it.each(["no_body", "unavailable", "too_large"] as const)("%s事件保留元数据但不能点击读取正文", async (availability) => {
    const { native, user, button } = await page({ evidenceSources: (request) => anEvidenceSources({ ...request, events: [anEvidenceEventSource({ availability })] }) });
    await user.click(button); expect(await screen.findByRole("button", { name: rawName() })).toBeDisabled();
    expect(native.callsTo("read_evidence_event")).toHaveLength(0); expect(screen.queryByTestId("evidence-event-body")).toBeNull();
  });

  it("明确从事件A换B先清A，迟到A不能覆盖B或预取其它正文", async () => {
    const old = deferred<EvidenceEventView>();
    const { native, user } = await page({ evidenceSources: (request) => anEvidenceSources({ ...request, events: [anEvidenceEventSource(), anEvidenceEventSource({ event_id: EVENT2 })] }),
      readingEvidenceEvent: (request) => request.event_id === EVIDENCE_EVENT_ID ? old.promise : anEvidenceEvent({ ...request, body: "当前B单条正文" }) });
    await metadata(user); await user.click(screen.getByRole("button", { name: rawName() }));
    await waitFor(() => expect(native.callsTo("read_evidence_event")).toHaveLength(1));
    await user.click(screen.getByRole("button", { name: rawName(EVENT2) })); expect(await screen.findByTestId("evidence-event-body")).toHaveTextContent("当前B单条正文");
    await act(async () => { old.resolve(anEvidenceEvent({ body: "迟到A不可展示" })); await old.promise; });
    expect(screen.getByTestId("evidence-event-body")).toHaveTextContent("当前B单条正文"); expect(screen.queryByText("迟到A不可展示")).toBeNull();
    expect(native.callsTo("read_evidence_event").map((call) => call.payload)).toEqual([{ request: { ...anchors, event_id: EVIDENCE_EVENT_ID } }, { request: { ...anchors, event_id: EVENT2 } }]);
  });

  it("同关系两条证据切换时旧metadata不能替换新选择，raw始终使用新证据锚", async () => {
    const old = deferred<EvidenceSourcesView>();
    const { native, user, button } = await page({ evidenceSources: (request) => request.evidence_id === E1 ? old.promise : anEvidenceSources({ ...request, notice: "当前第二条证据" }) });
    await user.click(button); await waitFor(() => expect(native.callsTo("evidence_sources")).toHaveLength(1));
    await user.click(screen.getByRole("button", { name: metadataName(E2) })); await screen.findByText("当前第二条证据");
    await act(async () => { old.resolve(anEvidenceSources({ notice: "迟到第一条证据" })); await old.promise; });
    expect(screen.queryByText("迟到第一条证据")).toBeNull();
    await user.click(screen.getByRole("button", { name: rawName() })); await screen.findByTestId("evidence-event-body");
    expect(native.callsTo("read_evidence_event")[0]?.payload).toEqual({ request: { ...anchors, evidence_id: E2, event_id: EVIDENCE_EVENT_ID } });
  });

  it("metadata读取中关闭后旧结果不复活，重开必须再明确点击", async () => {
    const old = deferred<EvidenceSourcesView>(); let reads = 0;
    const { native, user, button } = await page({ evidenceSources: (request) => ++reads === 1 ? old.promise : anEvidenceSources(request) });
    await user.click(button); await waitFor(() => expect(reads).toBe(1));
    await user.click(screen.getByRole("button", { name: "关闭并清除本次来源展示" }));
    await act(async () => { old.resolve(anEvidenceSources()); await old.promise; });
    expect(screen.queryByRole("heading", { name: "本地单条来源查看" })).toBeNull(); expect(native.callsTo("read_evidence_event")).toHaveLength(0);
    await user.click(button); await screen.findByRole("button", { name: rawName() }); expect(reads).toBe(2); expect(native.callsTo("read_evidence_event")).toHaveLength(0);
  });

  it.each(["success", "failure"] as const)("正文读取中关闭，迟到%s不复活正文或错误", async (outcome) => {
    const old = deferred<EvidenceEventView>(); const { native, user } = await page({ readingEvidenceEvent: () => old.promise });
    await metadata(user); await user.click(screen.getByRole("button", { name: rawName() })); await waitFor(() => expect(native.callsTo("read_evidence_event")).toHaveLength(1));
    await user.click(screen.getByRole("button", { name: "关闭并清除本次来源展示" }));
    await act(async () => { if (outcome === "success") old.resolve(anEvidenceEvent()); else old.reject({ reason_code: "ROUTINE", explanation: "迟到旧拒绝" }); try { await old.promise; } catch { /* expected rejected fixture */ } });
    expect(screen.queryByTestId("evidence-event-body")).toBeNull(); expect(screen.queryByText("迟到旧拒绝")).toBeNull(); expect(screen.queryByRole("heading", { name: "本地单条来源查看" })).toBeNull();
  });

  it.each(["source", "hash", "unmount"] as const)("%s改变拒收迟到正文且没有后续自动操作", async (change) => {
    const old = deferred<EvidenceEventView>(); const { native, user, unmount } = await page({ readingEvidenceEvent: () => old.promise });
    await metadata(user); await user.click(screen.getByRole("button", { name: rawName() })); await waitFor(() => expect(native.callsTo("read_evidence_event")).toHaveLength(1));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "hash") act(() => navigate("#/graph")); else unmount();
    expect(screen.queryByRole("heading", { name: "本地单条来源查看" })).toBeNull();
    await act(async () => { old.resolve(anEvidenceEvent()); await old.promise; });
    expect(screen.queryByTestId("evidence-event-body")).toBeNull(); expect(native.callsTo("read_evidence_event")).toHaveLength(1);
    expect(native.callsTo("person_summary")).toHaveLength(0); expect(native.callsTo("generate_draft")).toHaveLength(0);
  });

  it.each(["source", "hash", "close"] as const)("%s变化明确清除已显示的正文和已显示的错误", async (change) => {
    for (const state of ["body", "error"] as const) {
      const { native, user, unmount } = await page({ readingEvidenceEvent: (request) => {
        if (state === "error") throw { reason_code: "ROUTINE", explanation: "已经显示的合成来源拒绝" };
        return anEvidenceEvent({ ...request, body: "已经显示的单条合成正文" });
      } });
      await metadata(user); await user.click(screen.getByRole("button", { name: rawName() }));
      if (state === "body") expect(await screen.findByTestId("evidence-event-body")).toHaveTextContent("已经显示的单条合成正文");
      else expect(await screen.findByRole("alert")).toHaveTextContent("已经显示的合成来源拒绝");
      if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
      else if (change === "hash") act(() => navigate("#/graph"));
      else await user.click(screen.getByRole("button", { name: "关闭并清除本次来源展示" }));
      expect(screen.queryByTestId("evidence-event-body")).toBeNull(); expect(screen.queryByText("已经显示的合成来源拒绝")).toBeNull();
      expect(screen.queryByRole("heading", { name: "本地单条来源查看" })).toBeNull(); expect(native.callsTo("read_evidence_event")).toHaveLength(1);
      unmount();
    }
  });

  it.each(["event", "namespace"] as const)("raw返回%s不匹配拒绝显示且不尝试其它来源", async (field) => {
    const { native, user } = await page({ readingEvidenceEvent: (request) => anEvidenceEvent({ ...request,
      ...(field === "event" ? { event_id: EVENT2 } : { namespace_id: "0192f000-0000-7000-8000-000000000999" }), body: "错误对象绝不展示" }) });
    await metadata(user); await user.click(screen.getByRole("button", { name: rawName() }));
    expect(await screen.findByRole("alert")).toHaveTextContent("与这一次指定来源不一致"); expect(screen.queryByText("错误对象绝不展示")).toBeNull(); expect(native.callsTo("read_evidence_event")).toHaveLength(1);
  });

  it("地址已改变但旧metadata/raw按钮还在DOM时零新IPC", async () => {
    const { native, user, button } = await page(); const raw = await metadata(user);
    act(() => { window.location.hash = "#/graph"; button.click(); raw.click(); });
    await waitFor(() => expect(screen.queryByRole("heading", { name: "本地单条来源查看" })).toBeNull());
    expect(native.callsTo("evidence_sources")).toHaveLength(1); expect(native.callsTo("read_evidence_event")).toHaveLength(0);
  });

  it("明确raw点击后微任务执行前source变更时不能开始旧读取", async () => {
    const { native, user } = await page(); const raw = await metadata(user); let mutation: Promise<unknown> = Promise.resolve();
    act(() => { raw.click(); mutation = setUserEndpoint("http://127.0.0.1:9"); }); await act(async () => { await mutation; });
    expect(native.callsTo("read_evidence_event")).toHaveLength(0); expect(screen.queryByTestId("evidence-event-body")).toBeNull();
  });
});
