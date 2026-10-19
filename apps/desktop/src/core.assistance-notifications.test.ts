import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, it, vi } from "vitest";
import * as core from "./core";
import { anAssistanceFeedback, anAssistanceNotificationAttempt, anAssistanceReminder, assistanceCategoryDefaults, ASSISTANCE_ITEM_ID, ASSISTANCE_REVISION, ASSISTANCE_SCHEDULE_REVISION, ASSISTANCE_FEEDBACK_REVISION, ASSISTANCE_NOTIFICATION_ATTEMPT, ASSISTANCE_WITHDRAWAL_ID } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const pointer = { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION };
const time = "2099-10-23T16:00:00.0001+08:00";
beforeEach(() => { vi.mocked(invoke).mockReset(); });

it("八个提醒和反馈接口只交明确ID、独立CAS和固定选择，不改变草稿授权", async () => {
  const source = core.getDraftSourceRevision();
  vi.mocked(invoke).mockResolvedValueOnce(anAssistanceReminder())
    .mockResolvedValueOnce(anAssistanceReminder({ schedule_revision: ASSISTANCE_SCHEDULE_REVISION, state: "scheduled", remind_at: time }))
    .mockResolvedValueOnce(anAssistanceFeedback())
    .mockResolvedValueOnce(anAssistanceFeedback({ feedback_revision: ASSISTANCE_FEEDBACK_REVISION, value: "helpful" }))
    .mockResolvedValueOnce(assistanceCategoryDefaults())
    .mockResolvedValueOnce({ category: "manual_goal", revision: "new-pref", muted: true })
    .mockResolvedValueOnce({ attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, withdrawal_id: ASSISTANCE_WITHDRAWAL_ID, state: "pending" })
    .mockResolvedValueOnce({ state: "running", last_error: null });
  await core.assistanceReminder(pointer.item_id);
  await core.setAssistanceReminder({ ...pointer, expected_schedule_revision: null, remind_at: time });
  await core.assistanceFeedback(pointer.item_id);
  await core.setAssistanceFeedback({ ...pointer, expected_feedback_revision: null, value: "helpful" });
  await core.assistanceCategoryPreferences();
  await core.setAssistanceCategoryPreference({ category: "manual_goal", expected_revision: null, muted: true });
  await core.retryAssistanceNotificationWithdrawal({ attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, expected_withdrawal_id: null });
  await core.assistanceNotificationRuntimeStatus();
  expect(invoke).toHaveBeenNthCalledWith(1, "assistance_reminder", { itemId: pointer.item_id });
  expect(invoke).toHaveBeenNthCalledWith(2, "set_assistance_reminder", { request: { ...pointer, expected_schedule_revision: null, remind_at: time } });
  expect(invoke).toHaveBeenNthCalledWith(4, "set_assistance_feedback", { request: { ...pointer, expected_feedback_revision: null, value: "helpful" } });
  expect(invoke).toHaveBeenNthCalledWith(6, "set_assistance_category_preference", { request: { category: "manual_goal", expected_revision: null, muted: true } });
  expect(invoke).toHaveBeenNthCalledWith(7, "retry_assistance_notification_withdrawal", { request: { attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, expected_withdrawal_id: null } });
  expect(invoke).toHaveBeenCalledTimes(8); expect(core.getDraftSourceRevision()).toBe(source);
});
it.each([
  { item_id: "other" }, { item_revision: "old" }, { schedule_revision: null }, { schedule_revision: ASSISTANCE_SCHEDULE_REVISION },
  { state: "suppressed" as const }, { remind_at: "2099-10-23T16:00:01+08:00" },
])("提醒写回的身份、独立新版本和明确时间必须一致：%j", async (changes) => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceReminder({ state: "scheduled", schedule_revision: "new-schedule", remind_at: time, ...changes }));
  await expect(core.setAssistanceReminder({ ...pointer, expected_schedule_revision: ASSISTANCE_SCHEDULE_REVISION, remind_at: time })).rejects.toThrow(/不符/);
});
it("关闭提醒必须返回新独立版本、off和null；历史旧attempt可保留", async () => {
  const value = anAssistanceReminder({ schedule_revision: "new-schedule", latest_attempt: anAssistanceNotificationAttempt({ item_revision: "old", delivery: "unknown" }) });
  vi.mocked(invoke).mockResolvedValueOnce(value).mockResolvedValueOnce({ ...value, state: "scheduled" });
  const request = { ...pointer, expected_schedule_revision: ASSISTANCE_SCHEDULE_REVISION, remind_at: null };
  await expect(core.setAssistanceReminder(request)).resolves.toBe(value); await expect(core.setAssistanceReminder(request)).rejects.toThrow(/不符/);
});
it.each([{ item_id: "other" }, { item_revision: "old" }, { feedback_revision: null }, { feedback_revision: ASSISTANCE_FEEDBACK_REVISION }, { value: "not_relevant" as const }])("反馈不接受错项、旧版本或相反选择：%j", async (changes) => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceFeedback({ feedback_revision: "new-feedback", value: "helpful", ...changes }));
  await expect(core.setAssistanceFeedback({ ...pointer, expected_feedback_revision: ASSISTANCE_FEEDBACK_REVISION, value: "helpful" })).rejects.toThrow(/不符/);
});
it("clear对应none，不以相反反馈冒充清除", async () => {
  vi.mocked(invoke).mockResolvedValueOnce(anAssistanceFeedback({ feedback_revision: "new-feedback" })).mockResolvedValueOnce(anAssistanceFeedback({ feedback_revision: "new-feedback", value: "helpful" }));
  const request = { ...pointer, expected_feedback_revision: null, value: "clear" as const };
  await expect(core.setAssistanceFeedback(request)).resolves.toMatchObject({ value: "none" }); await expect(core.setAssistanceFeedback(request)).rejects.toThrow(/不符/);
});
it.each([{ category: "observation" }, { revision: "old" }, { revision: null }, { muted: false }])("类别修改必须匹配scope、新revision及选择：%j", async (changes) => {
  vi.mocked(invoke).mockResolvedValue({ category: "manual_goal", revision: "new", muted: true, ...changes });
  await expect(core.setAssistanceCategoryPreference({ category: "manual_goal", expected_revision: "old", muted: true })).rejects.toThrow(/不符/);
});
it.each([{ attempt_id: "other" }, { withdrawal_id: "old" }])("撤除新请求不能借用旧withdrawal或错attempt：%j", async (changes) => {
  vi.mocked(invoke).mockResolvedValue({ attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, withdrawal_id: "new", state: "pending", ...changes });
  await expect(core.retryAssistanceNotificationWithdrawal({ attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, expected_withdrawal_id: "old" })).rejects.toThrow(/不符/);
});
it("读取提醒也核对嵌套attempt和withdrawal身份", async () => {
  vi.mocked(invoke).mockResolvedValue(anAssistanceReminder({ latest_attempt: anAssistanceNotificationAttempt({ latest_withdrawal: { withdrawal_id: "w", attempt_id: "other", state: "unknown" } }) }));
  await expect(core.assistanceReminder(pointer.item_id)).rejects.toThrow(/不符/);
});
