/** Real App navigation and existing IPC wrappers; only synthetic graph data. */
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App";
import * as core from "../core";
import type { PeopleGraph } from "../core";
import { aLocalInsight, aPeopleGraph, aPersonSummary, installFakeCore } from "../test/fakeCore";
import { Graph } from "./Graph";
import { LocalInsights } from "./LocalInsights";
import { graphTargetHash, MAX_GRAPH_TARGET_REFERENCES } from "./graphTarget";

const base = aPeopleGraph(); const a = base.people[1]!; const edgeA = base.ties[0]!;
const b = { ...a, contact_id: "0192f000-0000-7000-8000-000000000003", identifier_hint: "cccccccc" };
const edgeB = { ...edgeA, relationship_id: "0192f000-0000-7000-8000-00000000000b", to_contact_id: b.contact_id,
  evidence: [{ ...edgeA.evidence[0]!, evidence_id: "0192f000-0000-7000-8000-0000000000e3" }] };
const graph = aPeopleGraph({ people: [...base.people, b], ties: [edgeA, edgeB] });
const targetA = graphTargetHash(a.contact_id, edgeA.evidence.map((row) => row.evidence_id))!;
const targetB = graphTargetHash(b.contact_id, edgeB.evidence.map((row) => row.evidence_id))!;
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
function navigate(hash: string): void { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); }
afterEach(() => { vi.restoreAllMocks(); });

describe("洞察到指定关系证据元数据的真实跳转", () => {
  it("从Import内两条相似统计选A，键盘定位A并只向A提交手动纠正", async () => {
    const insight = aLocalInsight({ summaries: [aPersonSummary(), aPersonSummary({ contact_id: b.contact_id,
      points: [{ ...aPersonSummary().points[0]!, evidence_ids: edgeB.evidence.map((row) => row.evidence_id) }] })] });
    const native = installFakeCore({ graph, insights: [insight] }); const user = userEvent.setup(); window.location.hash = "#/import"; render(<App />);
    const links = await screen.findAllByRole("link", { name: "核对此联系人的证据条目（元数据）" });
    expect(links[0]).toHaveAttribute("href", targetA); expect(links[1]).toHaveAttribute("href", targetB);
    act(() => links[0]!.focus()); await user.keyboard("{Enter}");
    const selected = await screen.findByTestId(`tie-${edgeA.relationship_id}`);
    expect(screen.queryByTestId(`tie-${edgeB.relationship_id}`)).toBeNull();
    await waitFor(() => expect(within(selected).getByRole("heading", { name: "关系：本人 ↔ bbbbbbbb" })).toHaveFocus());
    const metadata = within(selected).getByRole("list", { name: "证据条目元数据" });
    for (const row of edgeA.evidence) expect(within(metadata).getByText(row.evidence_id)).toBeVisible();
    expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0);
    expect(native.callsTo("generate_draft")).toHaveLength(0);
    await user.click(within(selected).getByRole("button", { name: "弱" }));
    await waitFor(() => expect(native.callsTo("correct_tie")).toHaveLength(1));
    expect(native.callsTo("correct_tie")[0]?.payload).toEqual({ relationshipId: edgeA.relationship_id, band: "weak" });
    expect(await screen.findByRole("heading", { name: "这次证据定位已失效" })).toBeVisible();
    expect(await screen.findByText("本次手动修改已收到核心回复。请明确清除旧定位，查看更新后的关系。")).toBeVisible();
    await user.click(screen.getByRole("link", { name: "清除定位，查看整个人脉图" }));
    expect(await screen.findByTestId(`tie-locked-${edgeA.relationship_id}`)).toBeVisible();
    expect(screen.queryByTestId(`tie-locked-${edgeB.relationship_id}`)).toBeNull();
    expect(native.callsTo("person_summary")).toHaveLength(0);
  });

  it("同Graph路由A/B快速切换拒收旧A读取，聚焦新的B", async () => {
    installFakeCore({ graph }); const old = deferred<PeopleGraph>();
    vi.spyOn(core, "peopleGraph").mockImplementationOnce(() => old.promise).mockResolvedValue(graph);
    window.location.hash = targetA; render(<Graph />); await waitFor(() => expect(core.peopleGraph).toHaveBeenCalledTimes(1));
    act(() => navigate(targetB)); const selected = await screen.findByTestId(`tie-${edgeB.relationship_id}`);
    await waitFor(() => expect(within(selected).getByRole("heading", { name: "关系：本人 ↔ cccccccc" })).toHaveFocus());
    await act(async () => { old.resolve(base); await old.promise; });
    expect(screen.getByTestId(`tie-${edgeB.relationship_id}`)).toBeVisible(); expect(screen.queryByTestId(`tie-${edgeA.relationship_id}`)).toBeNull();
  });

  it.each(["source", "unmount"] as const)("%s变化拒收迟到的旧目标读取且不产生写或摘要", async (change) => {
    const native = installFakeCore({ graph }); const old = deferred<PeopleGraph>(); vi.spyOn(core, "peopleGraph").mockReturnValue(old.promise);
    window.location.hash = targetA; const view = render(<Graph />); await waitFor(() => expect(core.peopleGraph).toHaveBeenCalledTimes(1));
    if (change === "source") await act(async () => { await core.setUserEndpoint("http://127.0.0.1:9"); }); else view.unmount();
    await act(async () => { old.resolve(graph); await old.promise; });
    expect(screen.queryByTestId(`tie-${edgeA.relationship_id}`)).toBeNull(); expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0);
    if (change === "source") expect(screen.getByRole("heading", { name: "这次证据定位已失效" })).toBeVisible();
  });

  it("普通全图初读被资料变更中断时可明确重读，迟到旧结果不复活", async () => {
    const native = installFakeCore({ graph }); const old = deferred<PeopleGraph>(); const user = userEvent.setup();
    vi.spyOn(core, "peopleGraph").mockImplementationOnce(() => old.promise).mockResolvedValue(graph);
    window.location.hash = "#/graph"; render(<Graph />);
    await waitFor(() => expect(core.peopleGraph).toHaveBeenCalledTimes(1));
    await act(async () => { await core.setUserEndpoint("http://127.0.0.1:9"); });
    expect(screen.getByRole("heading", { name: "资料已变更，这次读取已停止" })).toBeVisible();
    expect(screen.queryByText("正在读本机的人脉图…")).toBeNull();
    await act(async () => { old.resolve(base); await old.promise; });
    expect(screen.queryByTestId(`tie-${edgeA.relationship_id}`)).toBeNull(); expect(core.peopleGraph).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "重新读取人脉图" }));
    expect(await screen.findByTestId(`tie-${edgeB.relationship_id}`)).toBeVisible();
    expect(core.peopleGraph).toHaveBeenCalledTimes(2);
    expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0); expect(native.callsTo("generate_draft")).toHaveLength(0);
  });

  it("hash已换B但旧A尚未提交卸下时，旧按钮不能开始纠正或摘要", async () => {
    const native = installFakeCore({ graph }); window.location.hash = targetA; render(<Graph />);
    const first = await screen.findByTestId(`tie-${edgeA.relationship_id}`);
    const oldCorrection = within(first).getByRole("button", { name: "弱" });
    const oldSummary = screen.getByRole("button", { name: "看这个人的摘要" });
    act(() => {
      window.location.hash = targetB;
      oldSummary.click(); oldCorrection.click();
    });
    expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0); expect(native.callsTo("generate_draft")).toHaveLength(0);
    const second = await screen.findByTestId(`tie-${edgeB.relationship_id}`);
    await waitFor(() => expect(within(second).getByRole("heading", { name: "关系：本人 ↔ cccccccc" })).toHaveFocus());
    expect(screen.queryByTestId(`tie-${edgeA.relationship_id}`)).toBeNull();
    expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0);
  });

  it("旧手动写仍pending时，普通读取的恢复按钮不能启动新读取", async () => {
    installFakeCore({ graph }); const pending = deferred<PeopleGraph>(); const old = deferred<PeopleGraph>(); const user = userEvent.setup();
    vi.spyOn(core, "correctTie").mockReturnValueOnce(pending.promise);
    vi.spyOn(core, "peopleGraph").mockResolvedValueOnce(graph).mockImplementationOnce(() => old.promise).mockResolvedValue(graph);
    window.location.hash = targetA; render(<Graph />);
    const first = await screen.findByTestId(`tie-${edgeA.relationship_id}`);
    await user.click(within(first).getByRole("button", { name: "弱" }));
    act(() => navigate("#/graph")); await waitFor(() => expect(core.peopleGraph).toHaveBeenCalledTimes(2));
    await act(async () => { await core.setUserEndpoint("http://127.0.0.1:9"); });
    const retry = screen.getByRole("button", { name: "重新读取人脉图" }); expect(retry).toBeDisabled();
    await user.click(retry); expect(core.peopleGraph).toHaveBeenCalledTimes(2);
    await act(async () => { pending.resolve(base); old.resolve(base); await Promise.all([pending.promise, old.promise]); });
    await waitFor(() => expect(retry).toBeEnabled());
    expect(screen.queryByTestId(`tie-${edgeA.relationship_id}`)).toBeNull();
  });

  it("旧URL引用缺失不落到统计相似的另一关系，也没有可点的纠正按钮", async () => {
    const native = installFakeCore({ graph: { ...graph, ties: [{ ...edgeA, evidence: [] }, edgeB] } }); window.location.hash = targetA; render(<Graph />);
    expect(await screen.findByRole("heading", { name: "这次证据定位已失效" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "弱" })).toBeNull(); expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0);
  });

  it.each(["unknown", "forgotten", "ambiguous"] as const)("%s目标不退回第一条关系或自动写入", async (reason) => {
    const value = reason === "forgotten" ? { ...graph, people: graph.people.map((person) => person.contact_id === a.contact_id ? { ...person, forgotten: true } : person) }
      : reason === "ambiguous" ? { ...graph, ties: [...graph.ties, { ...edgeA, relationship_id: "0192f000-0000-7000-8000-00000000000c" }] } : graph;
    const native = installFakeCore({ graph: value });
    window.location.hash = reason === "unknown" ? graphTargetHash("0192f000-0000-7000-8000-000000000404", edgeA.evidence.map((row) => row.evidence_id))! : targetA;
    render(<Graph />); expect(await screen.findByRole("heading", { name: "这次证据定位已失效" })).toBeVisible();
    expect(screen.queryByRole("button", { name: "弱" })).toBeNull(); expect(native.callsTo("correct_tie")).toHaveLength(0); expect(native.callsTo("person_summary")).toHaveLength(0);
  });

  it("目标切到B后旧A手动写回复只结束busy，不替换B关系", async () => {
    installFakeCore({ graph }); const pending = deferred<PeopleGraph>();
    const correction = vi.spyOn(core, "correctTie").mockReturnValueOnce(pending.promise);
    window.location.hash = targetA; const user = userEvent.setup(); render(<Graph />);
    const first = await screen.findByTestId(`tie-${edgeA.relationship_id}`); await user.click(within(first).getByRole("button", { name: "弱" }));
    expect(correction).toHaveBeenCalledWith(edgeA.relationship_id, "weak");
    act(() => navigate(targetB)); const second = await screen.findByTestId(`tie-${edgeB.relationship_id}`);
    expect(within(second).getByRole("button", { name: "弱" })).toBeDisabled();
    await act(async () => { pending.resolve(base); await pending.promise; });
    await waitFor(() => expect(within(second).getByRole("button", { name: "弱" })).toBeEnabled());
    expect(screen.queryByTestId(`tie-${edgeA.relationship_id}`)).toBeNull(); expect(screen.getByTestId(`tie-${edgeB.relationship_id}`)).toBeVisible();
  });

  it("超过引用上限整summary没有精确链接，明确只提供普通全图入口", async () => {
    const ids = Array.from({ length: MAX_GRAPH_TARGET_REFERENCES + 1 }, (_, index) => `0192f000-0000-7000-8000-${(index + 100).toString(16).padStart(12, "0")}`);
    installFakeCore({ insights: [aLocalInsight({ summaries: [aPersonSummary({ points: [{ ...aPersonSummary().points[0]!, evidence_ids: ids }] })] })] }); render(<LocalInsights />);
    expect(await screen.findByText(/引用为空、格式不支持或超出定位上限/)).toBeVisible();
    expect(screen.queryByRole("link", { name: "核对此联系人的证据条目（元数据）" })).toBeNull(); expect(screen.getByRole("link", { name: "查看整个人脉图" })).toHaveAttribute("href", "#/graph");
  });
});
