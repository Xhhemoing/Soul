import { useState } from "react";
import {
  assistanceCategoryPreferences, assistanceFeedback, assistanceNotificationRuntimeStatus, assistanceReminder,
  retryAssistanceNotificationWithdrawal, setAssistanceCategoryPreference, setAssistanceFeedback, setAssistanceReminder,
  type AssistanceCategoryPreferences, type AssistanceFeedbackView, type AssistanceItemRow,
  type AssistanceNotificationRuntimeStatus, type AssistanceReminderView,
} from "../core";
import { useAssistancePanel } from "./assistancePanels";

const RUNTIME = { not_started: "尚未启动", running: "运行中", unsupported: "此平台暂不支持", failed: "运行失败", stopping: "正在停止", stopped: "已停止" };
const CATEGORY = { manual_goal: "明确目标类", observation: "本地观察类" };
const REMINDER = { off: "提醒已关闭", scheduled: "已安排提醒", invalidated: "旧提醒已失效，请明确重新安排", expired_in_inbox: "本次提醒仅保留在收件箱", suppressed: "该类提醒已静默", check_failed: "提醒状态核对失败" };
const DELIVERY = { claimed: "正在处理本次通知提交", submitted_to_os: "通知已提交给系统，不代表已展示或已读", failed: "通知提交失败；如仍需要，请明确新安排未来提醒", unknown: "旧通知提交结果未知；不会重投这次通知" };
const WITHDRAWAL = { pending: "等待撤除", claimed: "正在撤除", withdrawn: "系统已接受撤除，不代表用户未曾看过", failed: "撤除失败", unknown: "撤除结果未知", not_present: "当前没有可操作的通知图标，不证明过去通知已撤回或未被看过" };
function Feedback({ error, notice }: { readonly error: string | null; readonly notice: string | null }): React.JSX.Element {
  return <>{error === null ? null : <p role="alert">{error}</p>}{notice === null ? null : <p role="status">{notice}</p>}</>;
}

export function AssistanceNotifications(): React.JSX.Element {
  const runtime = useAssistancePanel<AssistanceNotificationRuntimeStatus>(); const categories = useAssistancePanel<AssistanceCategoryPreferences>(() => true, "categories");
  return <section className="panel"><h2>提醒与通知设置</h2>
    <p>提醒默认关闭，仅在 Soul 运行期间提供。系统是否接收、展示或打开分别记录，不保证应用退出后提醒或通知中心驻留。</p>
    <button type="button" disabled={runtime.busy} onClick={() => runtime.run(assistanceNotificationRuntimeStatus)}>核对通知运行状态</button>
    <Feedback error={runtime.error} notice={runtime.notice} />
    {runtime.value === null ? null : <p>通知服务：{RUNTIME[runtime.value.state]}；最近错误代码：{runtime.value.last_error ?? "无"}。这是服务状态，不代表系统通知权限或投递成功。</p>}
    <button type="button" disabled={categories.busy} onClick={() => categories.run(assistanceCategoryPreferences)}>核对两类提醒偏好</button>
    <Feedback error={categories.error} notice={categories.notice} />
    <p>类别选择只影响明确目标或本地观察这一类提醒，不推断某个人、人格或语气偏好。恢复后不补发已经过期的积压。</p>
    {categories.value?.preferences.map((preference) => <div key={preference.category}>
      <p>{CATEGORY[preference.category]}：{preference.muted ? "已静默" : "允许已明确安排的提醒"}。</p>
      <button type="button" disabled={categories.blocked} onClick={() => {
        const snapshot = categories.value; if (snapshot === null) return;
        categories.run(async () => {
          const value = await setAssistanceCategoryPreference({ category: preference.category, expected_revision: preference.revision, muted: !preference.muted });
          return { preferences: snapshot.preferences.map((row) => row.category === value.category ? value : row) };
        }, true, preference.muted ? "已恢复此类提醒；不会补发已过期积压。" : "已静默此类提醒；目标和事实仍保留。");
      }}>{preference.muted ? "恢复" : "静默"}{CATEGORY[preference.category]}提醒</button>
    </div>)}
  </section>;
}

function validFuture(value: string, now: number): boolean {
  return !value.endsWith("-00:00") && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value)
    && Number.isFinite(Date.parse(value)) && Date.parse(value) > now;
}

export function AssistanceReminder({ item, getCurrentItem }: { readonly item: AssistanceItemRow; readonly getCurrentItem: () => AssistanceItemRow | null }): React.JSX.Element {
  const current = (): boolean => { const row = getCurrentItem(); return row?.item_id === item.item_id && row.revision === item.revision; };
  const panel = useAssistancePanel<AssistanceReminderView>(current, `reminder:${item.item_id}`); const [time, setTime] = useState(""); const [viewNow] = useState(() => Date.now());
  const value = panel.value !== null && panel.value.item_id === item.item_id && panel.value.item_revision === item.revision ? panel.value : null;
  const attempt = value?.latest_attempt ?? null; const withdrawal = attempt?.latest_withdrawal ?? null;
  const canSchedule = item.handling === "open" && item.validity === "current";
  const set = (remindAt: string | null): void => {
    if (value === null || (remindAt !== null && (!canSchedule || !validFuture(remindAt, Date.now())))) return;
    panel.run(() => setAssistanceReminder({ item_id: item.item_id, expected_revision: item.revision, expected_schedule_revision: value.schedule_revision, remind_at: remindAt }), true,
      remindAt === null ? "已收到关闭提醒的记录；旧通知的撤除结果需另行核对。" : "已记录这次明确的新提醒安排；旧通知不会作为本次安排重投。");
  };
  return <section className="panel"><h3>这项的提醒</h3>
    <button type="button" disabled={panel.busy} onClick={() => panel.run(async () => {
      const result = await assistanceReminder(item.item_id);
      if (result.item_revision !== item.revision) throw new Error("事项版本已变化，请先重新读取当前事项。"); return result;
    })}>核对这项的提醒</button>
    <Feedback error={panel.error} notice={panel.notice} />
    {value === null ? <p>请先明确核对提醒记录，再决定是否安排。</p> : <>
      <p>{REMINDER[value.state]}。提醒时间：{value.remind_at ?? "未设定"}。</p>
      {value.failure_code === null ? null : <p>提醒错误代码：{value.failure_code}。</p>}
      <label htmlFor="assistance-reminder-at">新的提醒时间（未来 RFC3339，含时区）</label><input id="assistance-reminder-at" value={time} placeholder="2026-10-23T16:00:00+08:00" disabled={panel.blocked || !canSchedule} onChange={(event) => setTime(event.target.value)} />
      <button type="button" disabled={panel.blocked || !canSchedule || !validFuture(time, viewNow)} onClick={() => set(time)}>确认新安排未来提醒</button>
      <button type="button" disabled={panel.blocked} onClick={() => set(null)}>关闭这项提醒</button>
      <p>改期或稍后都需要明确填写新的未来时间。关闭、失效、到期或恢复类别偏好都不会自动补投。新的安排是你的新意图，不代表旧通知已撤除。</p>
      {attempt === null ? null : <div data-testid="assistance-notification-attempt">
        {attempt.schedule_revision === value.schedule_revision ? null : <p>下方是之前一次提醒的投递记录，与当前新安排分开保留。</p>}
        <p>{DELIVERY[attempt.delivery]}</p>
        {attempt.delivery === "unknown" ? <p>如明确安排新的未来提醒，旧提交仍保持未知；请另行核对旧通知的撤除结果。</p> : null}
        <p>系统展示：{attempt.shown_at === null ? "未观察" : "已观察到展示回调"}；打开：{attempt.opened_at === null ? "未观察" : "已观察到打开回调"}；隐藏：{attempt.hidden_at === null ? "未观察" : "已观察到隐藏回调"}；超时：{attempt.timed_out_at === null ? "未观察" : "已观察到超时回调"}。</p>
        {attempt.tracking_expired_at === null ? null : <p>本地通知资源跟踪期限已到，不代表用户未看过，也不等于撤除成功。</p>}
        <p>撤除记录：{withdrawal === null ? "尚无请求" : WITHDRAWAL[withdrawal.state]}。</p>
        {withdrawal !== null && !["failed", "unknown"].includes(withdrawal.state) ? null : <button type="button" disabled={panel.blocked} onClick={() => panel.run(async () => {
          const result = await retryAssistanceNotificationWithdrawal({ attempt_id: attempt.attempt_id, expected_withdrawal_id: withdrawal?.withdrawal_id ?? null });
          return { ...value, latest_attempt: { ...attempt, latest_withdrawal: result } };
        }, true, "已收到撤除请求的记录；这里只撤除原通知，不会重投。")}>尝试撤除原通知（不会重投）</button>}
      </div>}
    </>}
  </section>;
}

export function AssistanceFeedback({ item, getCurrentItem }: { readonly item: AssistanceItemRow; readonly getCurrentItem: () => AssistanceItemRow | null }): React.JSX.Element {
  const current = (): boolean => { const row = getCurrentItem(); return row?.item_id === item.item_id && row.revision === item.revision; };
  const panel = useAssistancePanel<AssistanceFeedbackView>(current, `feedback:${item.item_id}`);
  const value = panel.value !== null && panel.value.item_id === item.item_id && panel.value.item_revision === item.revision ? panel.value : null;
  return <section className="panel"><h3>这项帮助的反馈</h3>
    <p>只记录你对这项帮助的明确选择，不改变目标、事实、语气、人格、批准或完成状态。查看、复制和编辑都不会自动反馈。</p>
    <button type="button" disabled={panel.busy} onClick={() => panel.run(async () => {
      const result = await assistanceFeedback(item.item_id); if (result.item_revision !== item.revision) throw new Error("事项版本已变化，请先重新读取当前事项。"); return result;
    })}>核对这项的反馈</button>
    <Feedback error={panel.error} notice={panel.notice} />
    {value === null ? null : <><p>当前选择：{value.value === "none" ? "未反馈" : value.value === "helpful" ? "有帮助" : "与这件事无关"}。</p>
      {(["helpful", "not_relevant", "clear"] as const).map((choice) => <button key={choice} type="button" disabled={panel.blocked} onClick={() => panel.run(() => setAssistanceFeedback({ item_id: item.item_id, expected_revision: item.revision, expected_feedback_revision: value.feedback_revision, value: choice }), true, "已收到本事项的明确反馈记录，不会据此训练或改变语气。")}>{choice === "helpful" ? "这项有帮助" : choice === "not_relevant" ? "这项与目标无关" : "清除这项反馈"}</button>)}
    </>}
  </section>;
}
