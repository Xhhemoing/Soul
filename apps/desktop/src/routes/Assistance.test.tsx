import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useLayoutEffect } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { setUserEndpoint, type AssistanceItemDetail } from "../core";
import { anAssistanceDetail, anAssistanceRow, aLocalInsight, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION, forbidNetwork, installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { Assistance } from "./Assistance";
import { assistanceHash } from "./assistanceTarget";
import { assistanceWrites } from "./assistanceRequests";
import { draftAssistanceSelection } from "./draftMaterialSelection";
import { draftLane } from "./draftRequests";
import { useRouteAddress } from "../routeAddress";

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((yes) => { resolve = yes; }); return { promise, resolve }; }
function CommitProbe({ onCommit }: { readonly onCommit: (hash: string) => void }): null {
  const address = useRouteAddress();
  useLayoutEffect(() => { onCommit(address.hash); }, [address, onCommit]);
  return null;
}
async function move(hash: string): Promise<void> {
  await act(async () => { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); });
  await act(async () => { await new Promise<void>((resolve) => { window.setTimeout(resolve, 0); }); });
}
async function page(options: FakeCoreOptions = {}, item = false, waitUntilRead = true) {
  await move(item ? assistanceHash(ASSISTANCE_ITEM_ID) : "#/assistance");
  const core = installFakeCore(options); const view = render(<Assistance />); const user = userEvent.setup();
  if (waitUntilRead) await waitFor(() => expect(screen.getByRole("button", { name: "核对事项列表" })).toBeEnabled());
  return { core, user, ...view };
}
beforeEach(() => { installFakeCore(); assistanceWrites.checked(); });
afterEach(async () => { cleanup(); draftAssistanceSelection.clear(); await waitFor(() => expect(draftLane.snapshot().busy).toBe(false)); assistanceWrites.checked(); });

describe("稳定辅助事项与明确目标", () => {
  it("默认只读元数据列表，不预读目标、记忆或原消息正文", async () => {
    const network = forbidNetwork(); const { core } = await page({ assistanceRows: [anAssistanceRow()] });
    expect(await screen.findByRole("link", { name: `打开事项 ${ASSISTANCE_ITEM_ID}` })).toBeVisible();
    expect(screen.queryByText("合成明确目标")).toBeNull();
    for (const name of ["assistance_item", "memory_detail", "read_evidence_event", "prepare_assistance_context", "prepare_draft", "generate_draft", "local_insights"]) expect(core.callsTo(name)).toHaveLength(0);
    expect(network).toEqual([]);
  });
  it("稳定地址打开后核对精确item并显示当前详情，终态不提供旧操作", async () => {
    const detail = anAssistanceDetail({ item: anAssistanceRow({ handling: "resolved_reported" }) });
    const { core } = await page({ assistanceRows: [detail.item], readingAssistanceItem: () => detail }, true);
    expect(await screen.findByRole("heading", { name: "合成明确目标" })).toBeVisible();
    expect(core.callsTo("assistance_item")[0]?.payload).toEqual({ itemId: ASSISTANCE_ITEM_ID });
    expect(screen.queryByRole("button", { name: "我已完成（自报）" })).toBeNull();
    expect(screen.queryByRole("button", { name: "用于本次起草" })).toBeNull();
  });
  it("新建只提交本人明确字段和显式带时区时间，不推断相对时间", async () => {
    const { core, user } = await page();
    fireEvent.change(screen.getByLabelText("目标标题"), { target: { value: "合成周五改稿" } });
    fireEvent.change(screen.getByLabelText("明确目标与完成条件"), { target: { value: "我确认提交修改稿后完成。" } });
    fireEvent.change(screen.getByLabelText(/明确截止时间/), { target: { value: "明天" } });
    expect(screen.getByRole("button", { name: "确认新建我的目标" })).toBeDisabled();
    expect(core.callsTo("create_assistance_goal")).toHaveLength(0);
    fireEvent.change(screen.getByLabelText(/明确截止时间/), { target: { value: "2026-10-23T16:00:00+08:00" } });
    await user.click(screen.getByRole("button", { name: "确认新建我的目标" }));
    await waitFor(() => expect(core.callsTo("create_assistance_goal")).toHaveLength(1));
    expect(core.callsTo("create_assistance_goal")[0]?.payload).toEqual({ request: { title: "合成周五改稿", summary: "我确认提交修改稿后完成。", deadline_at: "2026-10-23T16:00:00+08:00", source_observation: null } });
    expect(await screen.findByText(/操作已提交/)).toBeVisible();
    expect(core.callsTo("generate_draft")).toHaveLength(0);
  });
  it.each(["我已完成（自报）", "取消这项目标"])("%s以当前opaque revision做条件写，成功后不展示旧详情", async (name) => {
    const { core, user } = await page({ assistanceRows: [anAssistanceRow()] }, true);
    await user.click(await screen.findByRole("button", { name }));
    await screen.findByText(/操作已提交/);
    expect(core.callsTo("act_assistance_item")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, action: name.startsWith("我") ? "complete_self_reported" : "cancel" } });
    expect(screen.queryByRole("heading", { name: "合成明确目标" })).toBeNull();
  });
  it("未知写入只允许重新读取，不自动重发，核对后清掉旧表单", async () => {
    const { core, user } = await page({ changingAssistance: () => { throw new Error("合成丢失回复"); }, assistanceRows: [anAssistanceRow()] }, true);
    await user.click(await screen.findByRole("button", { name: "我已完成（自报）" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("写入结果未能确认");
    expect(core.callsTo("act_assistance_item")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "确认新建我的目标" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "核对事项列表" }));
    expect(await screen.findByText(/未重放任何写入/)).toBeVisible();
    expect(core.callsTo("act_assistance_item")).toHaveLength(1);
    expect(screen.getByLabelText("目标标题")).toHaveValue("");
  });
  it("只在明确读取后列出可选来源，关联操作只携带bundle/contact IDs", async () => {
    const insight = aLocalInsight(); const summary = insight.summaries[0]!;
    const detail = anAssistanceDetail({ item: anAssistanceRow({ source: { kind: "observation", bundle_id: insight.bundle_id, contact_id: summary.contact_id, namespace_id: insight.namespace_id } }), goal: null });
    const { core, user } = await page({ insights: [insight], changingAssistance: () => detail });
    expect(core.callsTo("local_insights")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "读取可选的本地统计候选" }));
    await user.selectOptions(screen.getByLabelText("观察来源候选与联系人"), `${insight.bundle_id}/${summary.contact_id}`);
    await user.click(screen.getByRole("button", { name: "把这个来源关联为观察事项" }));
    await screen.findByText(/操作已提交/);
    expect(core.callsTo("link_assistance_observation")[0]?.payload).toEqual({ request: { bundle_id: insight.bundle_id, contact_id: summary.contact_id } });
    expect(core.callsTo("read_evidence_event")).toHaveLength(0); expect(core.callsTo("prepare_insight_context")).toHaveLength(0);
  });
  it("用于起草只保留item与revision意图，未直接读取材料或授予端点", async () => {
    const { core, user } = await page({ assistanceRows: [anAssistanceRow()] }, true);
    await user.click(await screen.findByRole("button", { name: "用于本次起草" }));
    expect(draftAssistanceSelection.snapshot()).toEqual({ item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION });
    expect(window.location.hash).toBe("#/draft");
    expect(core.callsTo("prepare_assistance_context")).toHaveLength(0); expect(core.callsTo("generate_draft")).toHaveLength(0);
  });
  it("已显示A切到B时先拒旧DOM动作，旧正文不出现在新地址", async () => {
    const other = "0192f000-0000-7000-8000-00000000a522";
    const { core } = await page({ readingAssistanceItem: (id) => id === ASSISTANCE_ITEM_ID ? anAssistanceDetail() : new Promise(() => {}) }, true);
    const oldComplete = await screen.findByRole("button", { name: "我已完成（自报）" });
    await act(async () => {
      window.location.hash = assistanceHash(other);
      fireEvent.click(oldComplete);
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });
    expect(screen.queryByRole("heading", { name: "合成明确目标" })).toBeNull();
    expect(core.callsTo("act_assistance_item")).toHaveLength(0);
  });
  it("B地址提交DOM后的layout effect已无A正文和操作，不等待passive effect清理", async () => {
    const other = "0192f000-0000-7000-8000-00000000a523"; const targetB = assistanceHash(other);
    await move(assistanceHash(ASSISTANCE_ITEM_ID));
    const core = installFakeCore({ readingAssistanceItem: (id) => id === ASSISTANCE_ITEM_ID ? anAssistanceDetail() : new Promise(() => {}) });
    const observations: { readonly prose: boolean; readonly action: boolean }[] = [];
    const inspectCommit = (hash: string): void => {
      if (hash !== targetB) return;
      const button = screen.queryByRole("button", { name: "我已完成（自报）" });
      observations.push({ prose: screen.queryByRole("heading", { name: "合成明确目标" }) !== null, action: button !== null });
      button?.click();
    };
    render(<><Assistance /><CommitProbe onCommit={inspectCommit} /></>);
    await screen.findByRole("heading", { name: "合成明确目标" });
    await move(targetB);
    expect(observations.length).toBeGreaterThan(0);
    expect(observations.every((value) => !value.prose && !value.action)).toBe(true);
    expect(core.callsTo("act_assistance_item")).toHaveLength(0);
  });
  it.each(["source", "address_aba", "unmount"])("迟到详情遇%s不落回旧页面", async (change) => {
    const pending = deferred<AssistanceItemDetail>(); let reads = 0;
    const { core, unmount } = await page({ readingAssistanceItem: () => ++reads === 1 ? pending.promise : new Promise(() => {}) }, true, false);
    await waitFor(() => expect(core.callsTo("assistance_item")).toHaveLength(1));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "unmount") unmount();
    else { await move("#/assistance"); await move(assistanceHash(ASSISTANCE_ITEM_ID)); }
    await act(async () => { pending.resolve(anAssistanceDetail()); await pending.promise; });
    expect(screen.queryByRole("heading", { name: "合成明确目标" })).toBeNull();
  });
  it("写入跨挂载仍占用操作通道，实际结束前不能再发写入", async () => {
    const pending = deferred<AssistanceItemDetail>();
    const first = await page({ changingAssistance: () => pending.promise, assistanceRows: [anAssistanceRow()] }, true);
    await first.user.click(await screen.findByRole("button", { name: "我已完成（自报）" }));
    await waitFor(() => expect(first.core.callsTo("act_assistance_item")).toHaveLength(1)); first.unmount();
    await move("#/assistance"); const second = installFakeCore(); render(<Assistance />);
    expect(screen.getByRole("button", { name: "读取可选的本地统计候选" })).toBeDisabled();
    await act(async () => { pending.resolve(anAssistanceDetail()); await pending.promise; });
    await waitFor(() => expect(screen.getByRole("button", { name: "读取可选的本地统计候选" })).toBeEnabled());
    expect(second.callsTo("act_assistance_item")).toHaveLength(0);
  });
});
