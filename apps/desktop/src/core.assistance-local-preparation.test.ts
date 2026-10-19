import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";
import * as core from "./core";
import { anAssistanceLocalPreparation, anAssistanceLocalTemplatePreview, ASSISTANCE_ITEM_ID, ASSISTANCE_LOCAL_PREFERENCE_REVISION, ASSISTANCE_LOCAL_TEMPLATE_ID, ASSISTANCE_REVISION } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const pointer = { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION };
const previewRequest = { ...pointer, expected_preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, template_id: ASSISTANCE_LOCAL_TEMPLATE_ID };
beforeEach(() => { vi.mocked(invoke).mockReset(); });

it("本地模板metadata、双CAS偏好与精确预览接口不改变已有草稿来源", async () => {
  const source = core.getDraftSourceRevision();
  vi.mocked(invoke).mockResolvedValueOnce(anAssistanceLocalPreparation())
    .mockResolvedValueOnce(anAssistanceLocalPreparation({ enabled: true, preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, state: "pending" }))
    .mockResolvedValueOnce(anAssistanceLocalTemplatePreview());
  await core.assistanceLocalPreparation(pointer.item_id);
  await core.setAssistanceLocalPreparation({ ...pointer, expected_preference_revision: null, enabled: true });
  const result = await core.previewAssistanceLocalTemplate(previewRequest);
  expect(result.body).toContain(core.LOCAL_TEMPLATE_BODY_SLOT);
  expect(vi.mocked(invoke).mock.calls).toEqual([
    ["assistance_local_preparation", { itemId: pointer.item_id }],
    ["set_assistance_local_preparation", { request: { ...pointer, expected_preference_revision: null, enabled: true } }],
    ["preview_assistance_local_template", { request: previewRequest }],
  ]); expect(core.getDraftSourceRevision()).toBe(source);
});
it("正文占位与现有Rust模板保持一致", () => {
  const source = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "crates", "soul-draft", "src", "template.rs"), "utf8");
  expect(source).toContain(`pub const BODY_SLOT: &str = "${core.LOCAL_TEMPLATE_BODY_SLOT}";`);
});
it.each([{ item_id: "other" }, { item_revision: "old" }, { preference_revision: null }, { preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION }, { enabled: false }])("偏好写回必须同itemrev、新prefrev和明确选择：%j", async (changes) => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceLocalPreparation({ enabled: true, state: "pending", preference_revision: "new", ...changes }));
  await expect(core.setAssistanceLocalPreparation({ ...pointer, expected_preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, enabled: true })).rejects.toThrow(/不符/);
});
it("关闭必须撤销模板并返回off，只换偏好revision", async () => {
  const request = { ...pointer, expected_preference_revision: ASSISTANCE_LOCAL_PREFERENCE_REVISION, enabled: false };
  vi.mocked(invoke).mockResolvedValueOnce(anAssistanceLocalPreparation({ preference_revision: "new" }))
    .mockResolvedValueOnce(anAssistanceLocalPreparation({ preference_revision: "new", template_id: ASSISTANCE_LOCAL_TEMPLATE_ID }));
  await expect(core.setAssistanceLocalPreparation(request)).resolves.toMatchObject({ item_revision: pointer.expected_revision, state: "off" });
  await expect(core.setAssistanceLocalPreparation(request)).rejects.toThrow(/不符/);
});
it.each([{ item_id: "other" }, { item_revision: "old" }, { preference_revision: "old-pref" }, { template_id: "old-template" }, { kind: "other" }, { body: "合成无占位正文" }, { prepared_at: Number.NaN }])("精确模板预览拒绝错身份或丢失BODY_SLOT：%j", async (changes) => {
  vi.mocked(invoke).mockResolvedValue({ ...anAssistanceLocalTemplatePreview(), ...changes });
  await expect(core.previewAssistanceLocalTemplate(previewRequest)).rejects.toThrow(/不符/);
});
it("ready metadata必须持有可查看的精确偏好和模板ID", async () => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceLocalPreparation({ state: "ready", enabled: true }));
  await expect(core.assistanceLocalPreparation(pointer.item_id)).rejects.toThrow(/不符/);
});
