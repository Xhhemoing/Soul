import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";
import * as core from "./core";
import { anAssistanceWork, anAssistanceWorkStatus, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const pointer = { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION };
beforeEach(() => { vi.mocked(invoke).mockReset(); });
it("全局和单项工作仅显式读取metadata；retry原样交当前CAS", async () => {
  const status = anAssistanceWorkStatus(); const work = anAssistanceWork();
  vi.mocked(invoke).mockResolvedValueOnce(status).mockResolvedValueOnce(work).mockResolvedValueOnce(work);
  await expect(core.assistanceWorkStatus()).resolves.toBe(status);
  await expect(core.assistanceItemWork(ASSISTANCE_ITEM_ID)).resolves.toBe(work);
  await expect(core.retryAssistanceWork(pointer)).resolves.toBe(work);
  expect(vi.mocked(invoke).mock.calls).toEqual([
    ["assistance_work_status", {}], ["assistance_item_work", { itemId: ASSISTANCE_ITEM_ID }], ["retry_assistance_work", { request: pointer }],
  ]);
});
it("核对和retry均不修改source代次或丢弃前台准备", async () => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceWork());
  const source = core.getDraftSourceRevision();
  await core.assistanceItemWork(ASSISTANCE_ITEM_ID); await core.retryAssistanceWork(pointer);
  expect(core.getDraftSourceRevision()).toBe(source);
  expect(vi.mocked(invoke).mock.calls.some(([name]) => /discard|prepare_.*context|draft/.test(name))).toBe(false);
});
it("错项metadata和错版本retry不交付为当前状态", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(anAssistanceWork({ item_id: "other" })).mockResolvedValueOnce(anAssistanceWork({ item_revision: "other" }));
  await expect(core.assistanceItemWork(ASSISTANCE_ITEM_ID)).rejects.toThrow(/身份不符/);
  await expect(core.retryAssistanceWork(pointer)).rejects.toThrow(/版本不符/);
});
it.each(["assistance_work_status", "assistance_item_work", "retry_assistance_work"])("%s结果遇来源变化拒收，无自动重放", async (command) => {
  let resolve!: (value: unknown) => void; const pending = new Promise((yes) => { resolve = yes; });
  vi.mocked(invoke).mockImplementation((name) => (name === command ? pending : Promise.resolve(undefined)) as Promise<never>);
  const result = command === "assistance_work_status" ? core.assistanceWorkStatus()
    : command === "assistance_item_work" ? core.assistanceItemWork(ASSISTANCE_ITEM_ID) : core.retryAssistanceWork(pointer);
  const rejected = expect(result).rejects.toBeInstanceOf(core.DraftSourceChangedError);
  await core.setVoice("length", "short"); resolve(command === "assistance_work_status" ? anAssistanceWorkStatus() : anAssistanceWork()); await rejected;
  expect(vi.mocked(invoke).mock.calls.filter(([name]) => name === command)).toHaveLength(1);
});
