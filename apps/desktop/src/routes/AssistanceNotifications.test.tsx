import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { setUserEndpoint, type AssistanceReminderView, type NotificationWithdrawal } from "../core";
import { anAssistanceFeedback, anAssistanceNotificationAttempt, anAssistanceReminder, anAssistanceRow, ASSISTANCE_FEEDBACK_REVISION, ASSISTANCE_ITEM_ID, ASSISTANCE_NOTIFICATION_ATTEMPT, ASSISTANCE_REVISION, ASSISTANCE_SCHEDULE_REVISION, ASSISTANCE_WITHDRAWAL_ID, forbidNetwork, installFakeCore } from "../test/fakeCore";
import { AssistanceFeedback, AssistanceNotifications, AssistanceReminder } from "./AssistanceNotifications";
import { assistanceWrites } from "./assistanceRequests";

const item = anAssistanceRow(); const time = "2099-10-23T16:00:00.0001+08:00";
function deferred<T>() { let resolve!: (value: T) => void; let reject!: (reason: unknown) => void; const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
async function move(hash: string): Promise<void> {
  await act(async () => { window.location.hash = hash; window.dispatchEvent(new HashChangeEvent("hashchange")); });
  await act(async () => { await new Promise<void>((resolve) => { window.setTimeout(resolve, 0); }); });
}
async function readReminder(): Promise<void> { await userEvent.click(screen.getByRole("button", { name: "核对这项的提醒" })); await screen.findByLabelText("新的提醒时间（未来 RFC3339，含时区）"); }
beforeEach(async () => { installFakeCore(); assistanceWrites.checked(); await move("#/assistance"); });
afterEach(async () => {
  cleanup(); await waitFor(() => expect(assistanceWrites.snapshot().busy).toBe(false));
  for (const record of ["categories", `reminder:${item.item_id}`, `feedback:${item.item_id}`]) assistanceWrites.checked(record, assistanceWrites.recovery(record));
  assistanceWrites.checked();
});

describe("U4/U6明确提醒与固定反馈", () => {
  it("挂载零读取和投递；全局核对只读metadata，不说明系统权限", async () => {
    const network = forbidNetwork(); const core = installFakeCore({ notificationRuntimeStatus: () => ({ state: "unsupported", last_error: null }) });
    render(<><AssistanceNotifications /><AssistanceReminder item={item} getCurrentItem={() => item} /><AssistanceFeedback item={item} getCurrentItem={() => item} /></>);
    expect(core.calls).toEqual([]);
    await userEvent.click(screen.getByRole("button", { name: "核对通知运行状态" }));
    expect(await screen.findByText(/通知服务：此平台暂不支持/)).toHaveTextContent("不代表系统通知权限");
    expect(core.calls.map((call) => call.cmd)).toEqual(["assistance_notification_runtime_status"]); expect(network).toEqual([]);
  });
  it("默认off；新安排和关闭分别双CAS，毫秒以下和显式时区原样提交", async () => {
    const core = installFakeCore(); render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    expect(screen.getByText(/提醒已关闭/)).toBeVisible();
    fireEvent.change(screen.getByLabelText("新的提醒时间（未来 RFC3339，含时区）"), { target: { value: time } });
    await userEvent.click(screen.getByRole("button", { name: "确认新安排未来提醒" }));
    expect(core.callsTo("set_assistance_reminder")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, expected_schedule_revision: null, remind_at: time } });
    expect(await screen.findByText(/已安排提醒。/)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "关闭这项提醒" }));
    expect(core.callsTo("set_assistance_reminder")[1]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, expected_schedule_revision: expect.any(String), remind_at: null } });
    expect(await screen.findByText(/提醒已关闭。/)).toBeVisible(); expect(core.callsTo("set_assistance_reminder")).toHaveLength(2);
  });
  it.each(["2099-10-23T16:00", "2099-10-23T16:00:00", "2099-10-23T16:00:00-00:00", "2000-01-01T00:00:00Z"])("不默补时区或接收非未来提醒：%s", async (value) => {
    const core = installFakeCore(); render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    fireEvent.change(screen.getByLabelText("新的提醒时间（未来 RFC3339，含时区）"), { target: { value } });
    expect(screen.getByRole("button", { name: "确认新安排未来提醒" })).toBeDisabled(); expect(core.callsTo("set_assistance_reminder")).toHaveLength(0);
  });
  it("旧unknown不是新意图的死路；旧schedule历史和撤除独立保留", async () => {
    const attempt = anAssistanceNotificationAttempt({ delivery: "unknown", schedule_revision: "old-schedule" });
    const core = installFakeCore({ readingAssistanceReminder: () => anAssistanceReminder({ state: "expired_in_inbox", schedule_revision: ASSISTANCE_SCHEDULE_REVISION, latest_attempt: attempt }),
      settingAssistanceReminder: (request) => anAssistanceReminder({ state: "scheduled", schedule_revision: "new-schedule", remind_at: request.remind_at, latest_attempt: attempt }) });
    render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    expect(screen.getByText(/下方是之前一次提醒/)).toBeVisible(); expect(screen.getByText(/旧通知提交结果未知/)).toBeVisible();
    fireEvent.change(screen.getByLabelText("新的提醒时间（未来 RFC3339，含时区）"), { target: { value: time } });
    await userEvent.click(screen.getByRole("button", { name: "确认新安排未来提醒" }));
    await screen.findByText(/已安排提醒。/); expect(screen.getByText(/旧通知提交结果未知/)).toBeVisible();
    expect(core.callsTo("set_assistance_reminder")).toHaveLength(1); expect(core.callsTo("retry_assistance_notification_withdrawal")).toHaveLength(0);
    await userEvent.click(screen.getByRole("button", { name: "尝试撤除原通知（不会重投）" }));
    expect(core.callsTo("retry_assistance_notification_withdrawal")[0]?.payload).toEqual({ request: { attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, expected_withdrawal_id: null } });
    expect(await screen.findByText(/撤除记录：等待撤除/)).toBeVisible(); expect(core.callsTo("set_assistance_reminder")).toHaveLength(1);
  });
  it.each(["pending", "claimed", "withdrawn", "not_present"] as const)("撤除%s不是可重投或重撤回状态", async (state) => {
    const core = installFakeCore({ readingAssistanceReminder: () => anAssistanceReminder({ latest_attempt: anAssistanceNotificationAttempt({ latest_withdrawal: { attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, withdrawal_id: ASSISTANCE_WITHDRAWAL_ID, state } }) }) });
    render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    expect(screen.queryByRole("button", { name: "尝试撤除原通知（不会重投）" })).toBeNull(); expect(core.calls).toHaveLength(1);
    if (state === "not_present") expect(screen.getByText(/当前没有可操作的通知图标/)).toHaveTextContent("不证明过去通知已撤回");
  });
  it.each(["failed", "unknown"] as const)("撤除%s的显式新请求携带精确旧WID", async (state) => {
    const core = installFakeCore({ readingAssistanceReminder: () => anAssistanceReminder({ latest_attempt: anAssistanceNotificationAttempt({ latest_withdrawal: { attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, withdrawal_id: ASSISTANCE_WITHDRAWAL_ID, state } }) }) });
    render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    await userEvent.click(screen.getByRole("button", { name: "尝试撤除原通知（不会重投）" })); await screen.findByText(/撤除记录：等待撤除/);
    expect(core.callsTo("retry_assistance_notification_withdrawal")[0]?.payload).toEqual({ request: { attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, expected_withdrawal_id: ASSISTANCE_WITHDRAWAL_ID } });
  });
  it("系统提交、展示、打开和资源期限分别显示，不推断已读或完成", async () => {
    const core = installFakeCore({ readingAssistanceReminder: () => anAssistanceReminder({ latest_attempt: anAssistanceNotificationAttempt({ shown_at: 1, opened_at: 2, tracking_expired_at: 3 }) }) });
    render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    expect(screen.getByText(/通知已提交给系统/)).toHaveTextContent("不代表已展示或已读");
    expect(screen.getByText(/系统展示：/)).toHaveTextContent("已观察到打开回调"); expect(screen.getByText(/本地通知资源跟踪期限已到/)).toHaveTextContent("不等于撤除成功");
    expect(core.calls).toHaveLength(1);
  });
  it("固定反馈独立CAS；明确清除不训练、不改变事项或语气", async () => {
    const core = installFakeCore({ readingAssistanceFeedback: () => anAssistanceFeedback({ feedback_revision: ASSISTANCE_FEEDBACK_REVISION }) });
    render(<AssistanceFeedback item={item} getCurrentItem={() => item} />);
    await userEvent.click(screen.getByRole("button", { name: "核对这项的反馈" }));
    await userEvent.click(await screen.findByRole("button", { name: "这项有帮助" }));
    expect(await screen.findByText("当前选择：有帮助。")).toBeVisible();
    expect(core.callsTo("set_assistance_feedback")[0]?.payload).toEqual({ request: { item_id: ASSISTANCE_ITEM_ID, expected_revision: ASSISTANCE_REVISION, expected_feedback_revision: ASSISTANCE_FEEDBACK_REVISION, value: "helpful" } });
    await userEvent.click(screen.getByRole("button", { name: "清除这项反馈" })); await screen.findByText("当前选择：未反馈。");
    expect(core.callsTo("set_assistance_feedback")[1]?.payload).toMatchObject({ request: { expected_feedback_revision: expect.any(String), value: "clear" } });
    expect(core.calls.map((call) => call.cmd)).toEqual(["assistance_feedback", "set_assistance_feedback", "set_assistance_feedback"]);
  });
  it("两类全局scope可明确静默/恢复，pref revision独立且不补过期积压", async () => {
    const core = installFakeCore(); render(<AssistanceNotifications />);
    await userEvent.click(screen.getByRole("button", { name: "核对两类提醒偏好" }));
    await userEvent.click(await screen.findByRole("button", { name: "静默明确目标类提醒" }));
    expect(core.callsTo("set_assistance_category_preference")[0]?.payload).toEqual({ request: { category: "manual_goal", expected_revision: null, muted: true } });
    await userEvent.click(await screen.findByRole("button", { name: "恢复明确目标类提醒" }));
    expect(await screen.findByText("已恢复此类提醒；不会补发已过期积压。")).toBeVisible();
    expect(core.callsTo("set_assistance_category_preference")[1]?.payload).toMatchObject({ request: { category: "manual_goal", expected_revision: expect.any(String), muted: false } });
    expect(screen.getByRole("button", { name: "静默本地观察类提醒" })).toBeVisible(); expect(core.calls).toHaveLength(3);
  });
  it("未知写只由同记录明确核对解除；无关runtime/feedback读取不能解除或自动重发", async () => {
    let writes = 0; const core = installFakeCore({ settingAssistanceReminder: (request) => {
      if (++writes === 1) throw new Error("合成response lost");
      return anAssistanceReminder({ state: "scheduled", schedule_revision: "new", remind_at: request.remind_at });
    } });
    render(<><AssistanceNotifications /><AssistanceReminder item={item} getCurrentItem={() => item} /><AssistanceFeedback item={item} getCurrentItem={() => item} /></>); await readReminder();
    fireEvent.change(screen.getByLabelText("新的提醒时间（未来 RFC3339，含时区）"), { target: { value: time } });
    await userEvent.click(screen.getByRole("button", { name: "确认新安排未来提醒" })); expect(await screen.findByRole("alert")).toHaveTextContent("结果尚未确认");
    expect(assistanceWrites.snapshot().unknown).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "核对通知运行状态" })); await screen.findByText(/通知服务：运行中/);
    await userEvent.click(screen.getByRole("button", { name: "核对这项的反馈" })); await screen.findByText("当前选择：未反馈。");
    expect(assistanceWrites.snapshot().unknown).toBe(true); expect(screen.getByRole("button", { name: "这项有帮助" })).toBeDisabled();
    assistanceWrites.checked(); expect(assistanceWrites.snapshot().unknown).toBe(true);
    await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    await readReminder(); expect(assistanceWrites.snapshot().unknown).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: "确认新安排未来提醒" })); await screen.findByText(/已安排提醒。/);
    expect(core.callsTo("set_assistance_reminder")).toHaveLength(2);
  });
  it("未知写之前已开始的同记录读取不能解除新unknown；之后的新明确读取才可恢复", async () => {
    const pending = deferred<AssistanceReminderView>(); let reads = 0;
    const core = installFakeCore({ readingAssistanceReminder: () => ++reads === 1 ? pending.promise : anAssistanceReminder(), settingAssistanceReminder: () => { throw new Error("合成response lost"); } });
    render(<><div data-testid="earlier-reader"><AssistanceReminder item={item} getCurrentItem={() => item} /></div><div data-testid="writer"><AssistanceReminder item={item} getCurrentItem={() => item} /></div></>);
    const earlier = within(screen.getByTestId("earlier-reader")); const writer = within(screen.getByTestId("writer"));
    await userEvent.click(earlier.getByRole("button", { name: "核对这项的提醒" }));
    await userEvent.click(writer.getByRole("button", { name: "核对这项的提醒" }));
    fireEvent.change(await writer.findByLabelText("新的提醒时间（未来 RFC3339，含时区）"), { target: { value: time } });
    await userEvent.click(writer.getByRole("button", { name: "确认新安排未来提醒" })); await writer.findByRole("alert");
    expect(assistanceWrites.snapshot().unknown).toBe(true);
    await act(async () => { pending.resolve(anAssistanceReminder()); await pending.promise; });
    expect(assistanceWrites.snapshot().unknown).toBe(true); expect(earlier.getByRole("button", { name: "关闭这项提醒" })).toBeDisabled();
    await userEvent.click(earlier.getByRole("button", { name: "核对这项的提醒" }));
    await waitFor(() => expect(assistanceWrites.snapshot().unknown).toBe(false)); expect(core.callsTo("set_assistance_reminder")).toHaveLength(1);
  });
  it.each(["source", "address_aba", "revision", "unmount"])("迟到提醒遇%s拒收旧身份", async (change) => {
    const pending = deferred<AssistanceReminderView>(); installFakeCore({ readingAssistanceReminder: () => pending.promise });
    let actual = item; const view = render(<AssistanceReminder item={item} getCurrentItem={() => actual} />);
    await userEvent.click(screen.getByRole("button", { name: "核对这项的提醒" }));
    if (change === "source") await act(async () => { await setUserEndpoint("http://127.0.0.1:9"); });
    else if (change === "address_aba") { await move("#/memory"); await move("#/assistance"); }
    else if (change === "revision") actual = { ...item, revision: "new" }; else view.unmount();
    await act(async () => { pending.resolve(anAssistanceReminder()); await pending.promise; });
    expect(screen.queryByLabelText("新的提醒时间（未来 RFC3339，含时区）")).toBeNull();
  });
  it("旧DOM在微任务前换地址不能发提醒写入", async () => {
    const core = installFakeCore(); render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    fireEvent.change(screen.getByLabelText("新的提醒时间（未来 RFC3339，含时区）"), { target: { value: time } });
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: "确认新安排未来提醒" })); window.location.hash = "#/memory"; window.dispatchEvent(new HashChangeEvent("hashchange")); });
    await waitFor(() => expect(assistanceWrites.snapshot().busy).toBe(false)); expect(core.callsTo("set_assistance_reminder")).toHaveLength(0);
  });
  it("撤除跨挂载仍占真实写槽；读取不能释放在途操作，晚回执只结束原请求", async () => {
    const pending = deferred<NotificationWithdrawal>(); const core = installFakeCore({ readingAssistanceReminder: () => anAssistanceReminder({ latest_attempt: anAssistanceNotificationAttempt() }), withdrawingAssistanceNotification: () => pending.promise });
    const first = render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    await userEvent.click(screen.getByRole("button", { name: "尝试撤除原通知（不会重投）" })); first.unmount();
    render(<AssistanceReminder item={item} getCurrentItem={() => item} />); await readReminder();
    expect(assistanceWrites.snapshot().busy).toBe(true); expect(screen.getByRole("button", { name: "尝试撤除原通知（不会重投）" })).toBeDisabled();
    await act(async () => { pending.resolve({ attempt_id: ASSISTANCE_NOTIFICATION_ATTEMPT, withdrawal_id: "new", state: "pending" }); await pending.promise; });
    await waitFor(() => expect(assistanceWrites.snapshot().busy).toBe(false)); expect(core.callsTo("retry_assistance_notification_withdrawal")).toHaveLength(1);
    expect(screen.getByText("撤除记录：尚无请求。")).toBeVisible();
  });
});
