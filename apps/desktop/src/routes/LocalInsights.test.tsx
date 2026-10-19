/** Actual local-only insight view and real IPC wrappers. All data is synthetic. */
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { LocalInsight } from "../core";
import { setUserEndpoint } from "../core";
import { aLocalInsight, CLOSED_SNAPSHOT, forbidNetwork, installFakeCore, LOCAL_INSIGHT_ID } from "../test/fakeCore";
import { LocalInsights } from "./LocalInsights";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }

describe("本地洞察与精确实例反馈", () => {
  it("即使端点已配置也只读取本地候选，不触发人物摘要E1或网络", async () => {
    const attempts = forbidNetwork(); const core = installFakeCore({ snapshot: { ...CLOSED_SNAPSHOT, llm_endpoint_configured: true }, insights: [aLocalInsight()] });
    render(<LocalInsights />); await screen.findByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` });
    expect(core.callsTo("local_insights")).toHaveLength(1); expect(core.callsTo("person_summary")).toHaveLength(0);
    expect(core.callsTo("generate_draft")).toHaveLength(0); expect(core.callsTo("prepare_draft")).toHaveLength(0); expect(attempts).toEqual([]);
    expect(screen.getByText(/不改变事实、不训练模型/)).toBeVisible();
  });
  it("忽略仅提交精确bundle UUID；重算产生新bundle但保留旧ignored历史", async () => {
    const core = installFakeCore({ insights: [aLocalInsight()] }); const user = userEvent.setup(); render(<LocalInsights />);
    await user.click(await screen.findByRole("button", { name: `忽略候选 ${LOCAL_INSIGHT_ID}` }));
    await waitFor(() => expect(screen.getByRole("button", { name: `忽略候选 ${LOCAL_INSIGHT_ID}` })).toBeDisabled());
    expect(core.callsTo("ignore_local_insight")[0]?.payload).toEqual({ bundleId: LOCAL_INSIGHT_ID });
    await user.click(screen.getByRole("button", { name: `重新计算候选 ${LOCAL_INSIGHT_ID}` }));
    const newId = "0192f000-0000-7000-8000-000000001601";
    await screen.findByRole("heading", { name: `本地候选 ${newId}` });
    const old = screen.getByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` }).closest("article");
    expect(within(old!).getByText(/状态：invalidated；已忽略/)).toBeVisible();
    expect(screen.getByRole("button", { name: `忽略候选 ${newId}` })).toBeEnabled();
    expect(core.callsTo("recompute_local_insight")[0]?.payload).toEqual({ bundleId: LOCAL_INSIGHT_ID });
    expect(core.calls.some((call) => /train|learn|correct_tie|person_summary/.test(call.cmd))).toBe(false);
  });
  it("source变化后迟到的initial read不能使旧洞察复活", async () => {
    const pending = deferred<readonly LocalInsight[]>(); const core = installFakeCore({ readingInsights: () => pending.promise });
    render(<LocalInsights />); await waitFor(() => expect(core.callsTo("local_insights")).toHaveLength(1));
    await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    await act(async () => { pending.resolve([aLocalInsight()]); await pending.promise; });
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull();
    expect(screen.getByText(/旧洞察已移出展示/)).toBeVisible();
  });
  it("已显示洞察遇source写入立即清除，unmount后的显式重读也不交付", async () => {
    const pending = deferred<readonly LocalInsight[]>(); let reads = 0;
    const core = installFakeCore({ readingInsights: () => ++reads === 1 ? [aLocalInsight()] : pending.promise });
    const user = userEvent.setup(); const view = render(<LocalInsights />);
    await screen.findByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` });
    await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull();
    await user.click(screen.getByRole("button", { name: "核对本地洞察" }));
    await waitFor(() => expect(core.callsTo("local_insights")).toHaveLength(2)); view.unmount();
    await act(async () => { pending.resolve([aLocalInsight()]); await pending.promise; });
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull(); expect(core.callsTo("person_summary")).toHaveLength(0);
  });
  it("忽略完成后才明确重读，读取绑定操作完成后的来源代次", async () => {
    const mutation = deferred<LocalInsight>(); const ignored = aLocalInsight({ ignored: true }); let reads = 0;
    const core = installFakeCore({ ignoringInsight: () => mutation.promise, readingInsights: () => ++reads === 1 ? [aLocalInsight()] : [ignored] });
    const user = userEvent.setup(); render(<LocalInsights />);
    await user.click(await screen.findByRole("button", { name: `忽略候选 ${LOCAL_INSIGHT_ID}` }));
    expect(core.callsTo("local_insights")).toHaveLength(1);
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull();
    await act(async () => { mutation.resolve(ignored); await mutation.promise; });
    await waitFor(() => expect(screen.getByRole("button", { name: `忽略候选 ${LOCAL_INSIGHT_ID}` })).toBeDisabled());
    expect(core.callsTo("local_insights")).toHaveLength(2);
    expect(screen.queryByText(/旧洞察已移出展示/)).toBeNull();
    expect(core.callsTo("person_summary")).toHaveLength(0);
  });
  it.each(["source", "unmount"] as const)("操作后的显式重读遇%s变化仍拒迟到结果，不重新授权旧读取", async (change) => {
    const read = deferred<readonly LocalInsight[]>(); let reads = 0;
    const core = installFakeCore({ insights: [aLocalInsight()], readingInsights: () => ++reads === 1 ? [aLocalInsight()] : read.promise });
    const user = userEvent.setup(); const view = render(<LocalInsights />);
    await user.click(await screen.findByRole("button", { name: `忽略候选 ${LOCAL_INSIGHT_ID}` }));
    await waitFor(() => expect(core.callsTo("local_insights")).toHaveLength(2));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); }); else view.unmount();
    await act(async () => { read.resolve([aLocalInsight({ ignored: true })]); await read.promise; });
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull();
    expect(core.callsTo("local_insights")).toHaveLength(2); expect(core.callsTo("generate_draft")).toHaveLength(0);
    if (change === "source") expect(screen.getByText(/旧洞察已移出展示/)).toBeVisible();
  });
  it("忽略拒绝后显示核心原因，不继续读取或自动重发", async () => {
    const core = installFakeCore({ insights: [aLocalInsight()], ignoringInsight: () => { throw { reason_code: "ROUTINE", explanation: "合成忽略拒绝" }; } });
    const user = userEvent.setup(); render(<LocalInsights />);
    await user.click(await screen.findByRole("button", { name: `忽略候选 ${LOCAL_INSIGHT_ID}` }));
    expect(await screen.findByRole("alert")).toHaveTextContent("合成忽略拒绝");
    expect(core.callsTo("ignore_local_insight")).toHaveLength(1); expect(core.callsTo("local_insights")).toHaveLength(1);
  });
  it("当前读取快照筛选不发新IPC、不读取正文、不反馈", async () => {
    const active = aLocalInsight(); const ignored = aLocalInsight({ bundle_id: "0192f000-0000-7000-8000-000000001622", ignored: true });
    const core = installFakeCore({ insights: [active, ignored] }); const user = userEvent.setup(); render(<LocalInsights />);
    await screen.findByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` });
    const before = core.calls.length;
    await user.selectOptions(screen.getByLabelText("候选展示范围"), "ignored");
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull();
    expect(screen.getByRole("heading", { name: `本地候选 ${ignored.bundle_id}` })).toBeVisible();
    await user.type(screen.getByLabelText("在已读取候选中筛联系人编号"), "不存在的合成编号");
    expect(screen.getByText(/没有符合展示筛选的候选/)).toBeVisible();
    expect(core.calls.length).toBe(before);
    expect(core.callsTo("read_evidence_event")).toHaveLength(0); expect(core.callsTo("ignore_local_insight")).toHaveLength(0);
  });
  it("首次读取失败可见且允许明确重读，不默默吞掉错误", async () => {
    let reads = 0; const core = installFakeCore({ readingInsights: () => { if (++reads === 1) throw new Error("合成候选读取失败"); return [aLocalInsight()]; } });
    const user = userEvent.setup(); render(<LocalInsights />);
    expect(await screen.findByRole("alert")).toHaveTextContent("合成候选读取失败");
    await user.click(screen.getByRole("button", { name: "核对本地洞察" }));
    expect(await screen.findByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeVisible();
    expect(core.callsTo("local_insights")).toHaveLength(2);
  });
  it("地址A→B→A拒收旧initial read，也拒绝旧DOM反馈", async () => {
    const pending = deferred<readonly LocalInsight[]>(); const core = installFakeCore({ readingInsights: () => pending.promise });
    const oldHash = window.location.hash; render(<LocalInsights />);
    await waitFor(() => expect(core.callsTo("local_insights")).toHaveLength(1));
    await act(async () => {
      window.location.hash = "#/memory"; window.dispatchEvent(new HashChangeEvent("hashchange"));
      window.location.hash = oldHash; window.dispatchEvent(new HashChangeEvent("hashchange"));
      pending.resolve([aLocalInsight()]); await pending.promise;
    });
    expect(screen.queryByRole("heading", { name: `本地候选 ${LOCAL_INSIGHT_ID}` })).toBeNull();
    expect(core.callsTo("ignore_local_insight")).toHaveLength(0);
  });
});
