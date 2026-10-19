import { useRef, useState, useSyncExternalStore } from "react";
import { assistanceLocalPreparation, previewAssistanceLocalTemplate, setAssistanceLocalPreparation,
  type AssistanceItemRow, type AssistanceLocalPreparation, type AssistanceLocalTemplatePreview } from "../core";
import { asRefusal } from "../refusal";
import { useAssistancePanel } from "./assistancePanels";
import { useAssistanceRequestOwner } from "./AssistanceWork";
import { draftLane } from "./draftRequests";

const STATE = { off: "本地模板准备已关闭", none: "尚无本地模板", pending: "等待本地模板准备", running: "正在本地准备模板", ready: "本地开场模板可查看",
  source_unavailable: "模板来源暂不可用", failed: "本地模板准备失败", unknown: "本地模板准备结果尚未确认" };

export function AssistanceLocalPreparationPanel({ item, getCurrentItem }: { readonly item: AssistanceItemRow; readonly getCurrentItem: () => AssistanceItemRow | null }): React.JSX.Element {
  const current = (): boolean => { const row = getCurrentItem(); return row?.item_id === item.item_id && row.revision === item.revision && row.source.kind === "manual_goal"; };
  const metadata = useAssistancePanel<AssistanceLocalPreparation>(current, `local-preparation:${item.item_id}`);
  const owner = useAssistanceRequestOwner(); const lane = useSyncExternalStore(draftLane.subscribe, draftLane.snapshot);
  const serial = useRef(0); const [epoch, setEpoch] = useState(0);
  const [preview, setPreview] = useState<{ readonly value: AssistanceLocalTemplatePreview; readonly epoch: number; readonly owner: { readonly address: typeof owner.address; readonly source: symbol } } | null>(null);
  const [error, setError] = useState<{ readonly text: string; readonly epoch: number; readonly owner: { readonly address: typeof owner.address; readonly source: symbol } } | null>(null);
  const [previewing, setPreviewing] = useState(false); const [needsRead, setNeedsRead] = useState(false);
  const value = metadata.value !== null && metadata.value.item_id === item.item_id && metadata.value.item_revision === item.revision ? metadata.value : null;
  const canEnable = item.validity === "current" && item.handling === "open";
  const visible = preview !== null && preview.epoch === epoch && owner.ownsPayload(preview.owner) && value?.enabled === true
    && value.preference_revision === preview.value.preference_revision && value.template_id === preview.value.template_id ? preview.value : null;
  const visibleError = error !== null && error.epoch === epoch && owner.ownsPayload(error.owner) ? error.text : null;
  const invalidate = (): void => { serial.current += 1; setEpoch(serial.current); setPreview(null); setError(null); setNeedsRead(false); };
  const read = (): void => {
    if (!owner.current() || !current() || metadata.busy) return;
    invalidate(); metadata.run(async () => {
      const result = await assistanceLocalPreparation(item.item_id);
      if (result.item_revision !== item.revision) throw new Error("事项版本已变化，请先重新读取当前事项。");
      return result;
    });
  };
  const change = (enabled: boolean): void => {
    if (!owner.current() || !current() || serial.current !== epoch || value === null || metadata.blocked || (enabled && !canEnable)) return;
    invalidate(); metadata.run(() => setAssistanceLocalPreparation({ item_id: item.item_id, expected_revision: item.revision,
      expected_preference_revision: value.preference_revision, enabled }), true,
    !enabled ? "本地模板准备已关闭；基础核对与提醒安排保持各自的记录。" : "已记录这项的本地模板准备意图；需要另行核对状态并明确查看。");
  };
  const open = (): void => {
    if (!owner.current() || !current() || serial.current !== epoch || value === null || !value.enabled || value.state !== "ready"
      || value.preference_revision === null || value.template_id === null || !canEnable || metadata.blocked || needsRead) return;
    const ticket = owner.begin(); if (ticket === null) return;
    const slot = draftLane.begin("local"); if (slot === null) { owner.finish(ticket); return; }
    const request = { item_id: item.item_id, expected_revision: item.revision, expected_preference_revision: value.preference_revision, template_id: value.template_id };
    const owns = (): boolean => owner.owns(ticket) && current() && serial.current === epoch;
    setPreviewing(true); setPreview(null); setError(null);
    void Promise.resolve().then(async () => {
      if (!owns()) return;
      const result = await previewAssistanceLocalTemplate(request);
      if (owns()) setPreview({ value: result, epoch, owner: { address: owner.address, source: owner.source } });
    }).catch((problem: unknown) => {
      if (owns()) { setNeedsRead(true); setError({ text: `${asRefusal(problem).explanation} 请重新核对本地准备后再查看。`, epoch, owner: { address: owner.address, source: owner.source } }); }
    }).finally(() => {
      // A route change or opt-out hides the preview immediately; the physical
      // operation still owns this local slot until its promise really settles.
      draftLane.finish(slot, false);
      if (owner.finish(ticket)) setPreviewing(false);
    });
  };
  if (item.source.kind !== "manual_goal") return <p>本地开场模板仅供明确目标选择，观察事项不会自动启用。</p>;
  return <section className="panel"><h3>本地开场模板</h3>
    <p>默认关闭。启用后只在本地准备开场模板，正文留给你填写；不会把目标摘要当作收到的消息，也不会发送或自动采用。</p>
    <button type="button" disabled={metadata.busy} onClick={read}>核对本地模板准备</button>
    {metadata.error === null ? null : <p role="alert">{metadata.error}</p>}{metadata.notice === null ? null : <p role="status">{metadata.notice}</p>}
    {value === null ? <p>请先核对这项的本地准备偏好。</p> : <>
      <p>{STATE[value.state]}。偏好：{value.enabled ? "已启用" : "已关闭"}。</p>
      {value.failure_code === null ? null : <p>本地模板错误代码：{value.failure_code}。</p>}
      <button type="button" disabled={metadata.blocked || (!value.enabled && !canEnable)} onClick={() => change(!value.enabled)}>{value.enabled ? "关闭这项本地模板准备" : "启用这项本地模板准备"}</button>
      {value.enabled && value.state === "failed" ? <button type="button" disabled={metadata.blocked || !canEnable} onClick={() => change(true)}>重新准备本地模板</button> : null}
      {value.enabled && value.state === "ready" && value.preference_revision !== null && value.template_id !== null ? <button type="button" disabled={metadata.blocked || lane.busy || previewing || needsRead || !canEnable} onClick={open}>查看本地开场模板</button> : null}
      <p>关闭只撤销本地模板，不表示事项已完成，也不取消基础核对或独立的提醒。操作繁忙时请明确核对后再决定，不会自动重试。</p>
    </>}
    {previewing ? <p role="status">正在本地核对并查看模板；离开页面不会提早结束在途操作。</p> : null}
    {visibleError === null ? null : <p role="alert">{visibleError}</p>}
    {visible === null ? null : <div data-testid="assistance-local-template"><p>这是一份本地开场模板，请自行填写正文；尚未授权端点使用。</p><p className="pre-wrap">{visible.body}</p></div>}
  </section>;
}
