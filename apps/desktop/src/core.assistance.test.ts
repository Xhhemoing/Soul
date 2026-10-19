import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";
import * as core from "./core";
import { anAssistanceContext, anAssistanceDetail, anAssistanceRow, anInsightReference, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const pointer = { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION };
const fields = { title: "合成目标", summary: "由本人明确确认的完成条件", deadline_at: "2026-10-23T16:00:00+08:00" };
beforeEach(() => { vi.mocked(invoke).mockReset(); });

it("事项列表和明确详情分别读取，详情按稳定ID核对", async () => {
  const detail = anAssistanceDetail();
  vi.mocked(invoke).mockResolvedValueOnce({ items: [detail.item] }).mockResolvedValueOnce(detail);
  await core.assistanceItems(); await expect(core.assistanceItem(ASSISTANCE_ITEM_ID)).resolves.toBe(detail);
  expect(vi.mocked(invoke).mock.calls).toEqual([["assistance_items", {}], ["assistance_item", { itemId: ASSISTANCE_ITEM_ID }]]);
});
it("目标CAS传opaque revision及显式时区，完成和取消保持不同动作", async () => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceDetail());
  await core.createAssistanceGoal({ ...fields, source_observation: pointer });
  await core.editAssistanceGoal({ ...pointer, ...fields });
  await core.bindAssistanceGoalSource({ ...pointer, source_observation: null });
  await core.actAssistanceItem({ ...pointer, action: "complete_self_reported" });
  await core.actAssistanceItem({ ...pointer, action: "cancel" });
  expect(vi.mocked(invoke).mock.calls).toEqual([
    ["create_assistance_goal", { request: { ...fields, source_observation: pointer } }],
    ["edit_assistance_goal", { request: { ...pointer, ...fields } }],
    ["bind_assistance_goal_source", { request: { ...pointer, source_observation: null } }],
    ["act_assistance_item", { request: { ...pointer, action: "complete_self_reported" } }],
    ["act_assistance_item", { request: { ...pointer, action: "cancel" } }],
  ]);
});
it("观察创建和刷新只交准确来源ID", async () => {
  const reference = anInsightReference();
  const detail = anAssistanceDetail({ item: anAssistanceRow({ source: { kind: "observation", bundle_id: reference.bundle_id, namespace_id: reference.namespace_id, contact_id: reference.contact_id } }), goal: null, observation: reference });
  vi.mocked(invoke).mockResolvedValue(detail);
  await core.linkAssistanceObservation({ bundle_id: reference.bundle_id, contact_id: reference.contact_id });
  await core.refreshAssistanceObservation({ ...pointer, bundle_id: reference.bundle_id });
  expect(vi.mocked(invoke).mock.calls).toEqual([
    ["link_assistance_observation", { request: { bundle_id: reference.bundle_id, contact_id: reference.contact_id } }],
    ["refresh_assistance_observation", { request: { ...pointer, bundle_id: reference.bundle_id } }],
  ]);
});
it("同来源已有稳定观察返回旧bundle，换用新bundle须另行刷新", async () => {
  const reference = anInsightReference();
  const detail = anAssistanceDetail({ item: anAssistanceRow({ source: { kind: "observation", bundle_id: reference.bundle_id, namespace_id: reference.namespace_id, contact_id: reference.contact_id } }), goal: null, observation: reference });
  vi.mocked(invoke).mockResolvedValue(detail);
  await expect(core.linkAssistanceObservation({ bundle_id: "a-new-bundle", contact_id: reference.contact_id }, reference.namespace_id)).resolves.toBe(detail);
  expect(invoke).toHaveBeenCalledTimes(1);
  await expect(core.linkAssistanceObservation({ bundle_id: "a-new-bundle", contact_id: reference.contact_id }, "wrong-namespace")).rejects.toThrow(/来源身份不符/);
});
it("事项准备直接交付核心持有的view，并核对item和revision", async () => {
  const value = anAssistanceContext(); vi.mocked(invoke).mockResolvedValue(value);
  const old = core.getDraftSourceRevision(); await expect(core.prepareAssistanceContext(pointer)).resolves.toBe(value);
  expect(core.getDraftSourceRevision()).not.toBe(old);
  expect(invoke).toHaveBeenCalledExactlyOnceWith("prepare_assistance_context", { request: pointer });
  vi.mocked(invoke).mockResolvedValue({ ...value, revision: "wrong" });
  await expect(core.prepareAssistanceContext(pointer)).rejects.toThrow(/版本身份不符/);
});
it("错项详情或写回不得作为当前内容显示", async () => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceDetail({ item: anAssistanceRow({ item_id: "another-item" }) }));
  await expect(core.assistanceItem(ASSISTANCE_ITEM_ID)).rejects.toThrow(/身份不符/);
  await expect(core.editAssistanceGoal({ ...pointer, ...fields })).rejects.toThrow(/身份不符/);
});
it("返回goal和观察必须属于当前item来源", async () => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceDetail({ goal: { ...anAssistanceDetail().goal!, memory_id: "other-memory" } }));
  await expect(core.assistanceItem(ASSISTANCE_ITEM_ID)).rejects.toThrow(/身份不符/);
  vi.mocked(invoke).mockResolvedValue(anAssistanceDetail({ observation: anInsightReference() }));
  await expect(core.assistanceItem(ASSISTANCE_ITEM_ID)).rejects.toThrow(/来源不符/);
});
it("来源变更拒收事项准备和详情，写入的未知异常不重放", async () => {
  let resolve!: (value: unknown) => void; const pending = new Promise((yes) => { resolve = yes; });
  vi.mocked(invoke).mockReturnValueOnce(pending).mockResolvedValue(undefined);
  const result = core.prepareAssistanceContext(pointer); const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  await core.setVoice("length", "short"); resolve(anAssistanceContext()); await rejected;
  vi.mocked(invoke).mockReset().mockRejectedValue(new Error("lost reply"));
  await expect(core.createAssistanceGoal(fields)).rejects.toThrow("lost reply");
  expect(invoke).toHaveBeenCalledTimes(1);
});
