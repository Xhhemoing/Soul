import { useEffect, useRef, useState } from "react";

import {
  cancelResearchSave, confirmResearchSave, prepareResearchSave,
  type ResearchSaveCancel, type ResearchSavePreview, type ResearchSaveReceipt,
} from "../core";

interface Operation {
  id?: string;
  cancelled: boolean;
  finished: boolean;
  cancellation?: Promise<ResearchSaveCancel>;
}

/** Cancel once per known request, including a request received after unmount. */
function cancel(operation: Operation): Promise<ResearchSaveCancel> | undefined {
  operation.cancelled = true;
  if (operation.id !== undefined && operation.cancellation === undefined && !operation.finished) {
    operation.cancellation = cancelResearchSave(operation.id);
  }
  return operation.cancellation;
}

export function ResearchSave(): React.JSX.Element {
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [destination, setDestination] = useState("");
  const [preview, setPreview] = useState<ResearchSavePreview | null>(null);
  const [preparing, setPreparing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [receipts, setReceipts] = useState<readonly ResearchSaveReceipt[]>([]);
  const live = useRef(false);
  const active = useRef<Operation | null>(null);
  const publishing = useRef<Operation | null>(null);
  const revision = useRef(0);

  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
      for (const operation of new Set([active.current, publishing.current])) {
        if (operation !== null) void cancel(operation)?.catch(() => undefined);
      }
      active.current = null;
    };
  }, []);

  function invalidate() {
    revision.current += 1;
    const operation = active.current ?? publishing.current;
    active.current = null;
    if (operation !== null) void cancel(operation)?.catch(() => undefined);
    setPreview(null);
    setPreparing(false);
    setError("");
    setStatus(publishing.current === null ? "输入已改变，请重新准备。" : "输入已改变；等待原请求的保存结果。");
  }

  function prepare() {
    if (active.current !== null || publishing.current !== null) return;
    const operation: Operation = { cancelled: false, finished: false };
    active.current = operation;
    revision.current += 1;
    setPreparing(true);
    setPreview(null);
    setError("");
    setStatus("");
    void prepareResearchSave({ scope: { from_day: from, to_day: to }, destination }).then(
      (value) => {
        operation.id = value.request_id;
        if (!live.current || active.current !== operation || operation.cancelled) {
          void cancel(operation)?.catch(() => undefined);
          return;
        }
        setPreview(value);
        setPreparing(false);
      },
      () => {
        if (!live.current || active.current !== operation) return;
        active.current = null;
        setPreparing(false);
        setError("准备失败，请检查日期、权限和新的绝对路径后重试。");
      },
    );
  }

  function confirm() {
    const operation = active.current;
    if (operation === null || operation.id === undefined || operation.cancelled || publishing.current !== null) return;
    publishing.current = operation;
    setSaving(true);
    setError("");
    setStatus("正在保存；取消请求不能撤回已经发布的文件。");
    void confirmResearchSave(operation.id).then(
      (receipt) => {
        operation.finished = true;
        publishing.current = null;
        if (active.current === operation) active.current = null;
        if (!live.current) return;
        setSaving(false);
        setPreview(null);
        setStatus("");
        // A receipt describes this request's actual outcome, even after edits
        // or cancellation. Do not use the current input path to label it.
        if (receipt.request_id !== operation.id) {
          setError("返回的请求编号不一致，无法确认本次保存结果。");
        } else {
          setReceipts((previous) => [...previous, receipt]);
        }
      },
      () => {
        operation.finished = true;
        publishing.current = null;
        if (active.current === operation) active.current = null;
        if (!live.current) return;
        setSaving(false);
        setPreview(null);
        setStatus("");
        setError("保存请求失败；本页没有成功收据。请重新准备后再确认。");
      },
    );
  }

  function cancelCurrent() {
    const operation = active.current ?? publishing.current;
    if (operation === null) return;
    const currentRevision = ++revision.current;
    const isSaving = publishing.current === operation;
    active.current = null;
    setPreview(null);
    setPreparing(false);
    setStatus(isSaving ? "已请求取消，等待本次保存的真实结果。" : "已停止本次准备；迟到的结果将取消。");
    void cancel(operation)?.then(
      (result) => {
        if (!live.current || operation.finished || revision.current !== currentRevision) return;
        if (result.already_published) {
          setStatus("本次请求已经发布，等待保存收据；取消不会删除文件。");
        } else if (isSaving) {
          setStatus("取消请求已返回，仍在等待保存结果。");
        } else if (result.cancelled) {
          setStatus("本次准备已取消。");
        } else {
          setStatus("未确认取消结果，请重新准备。");
        }
      },
      () => {
        if (!live.current || operation.finished || revision.current !== currentRevision) return;
        setStatus(isSaving ? "取消请求失败，仍在等待保存结果。" : "取消请求失败；旧预览已失效，请重新准备。");
      },
    );
  }

  return (
    <section className="panel" aria-labelledby="research-save-heading">
      <h2 id="research-save-heading">保存本人研究数据</h2>
      <p>用途：本人行为研究。采集同意不等于研究保存同意；准备后还需单独确认。</p>
      <p>只保存事件类型、UTC 小时时间桶、聚合计数。计数不代表活跃时长，缺失不补零；不含正文、个人编号、应用名、时长或特质。</p>
      <p>明文独立副本；原库遗忘不会删除本文件，不能用于恢复 Soul。只写入你指定的本机新文件。</p>
      <form onSubmit={(event) => { event.preventDefault(); prepare(); }}>
        <label>UTC 开始日期<input type="date" required value={from} onChange={(event) => { invalidate(); setFrom(event.target.value); }} /></label>
        <label>UTC 结束日期（不含）<input type="date" required value={to} onChange={(event) => { invalidate(); setTo(event.target.value); }} /></label>
        <label>新的 ZIP 文件绝对路径<input type="text" required value={destination} onChange={(event) => { invalidate(); setDestination(event.target.value); }} /></label>
        <button type="submit" disabled={preparing || saving || preview !== null}>准备研究保存</button>
      </form>
      {(preparing || saving || preview !== null) && <button type="button" onClick={cancelCurrent}>取消本次保存</button>}
      {status !== "" && <p role="status">{status}</p>}
      {error !== "" && <p role="alert">{error}</p>}
      {preview !== null && (
        <div data-testid="research-save-preview">
          <h3>请核对本次保存</h3>
          <p>请求编号：{preview.request_id}</p>
          <p>目的地：{preview.destination}</p>
          <p>UTC 范围：{preview.dataset.scope.from_day} 至 {preview.dataset.scope.to_day}（结束不含），小时精度。</p>
          <p>准备时间：{preview.prepared_at_utc}</p>
          <p>完整范围 {preview.dataset.rows.length} 行；纳入 {preview.dataset.counts.included_events} 个事件。</p>
          <p>准备时统计：候选 {preview.dataset.counts.candidate_events}；用途排除 {preview.dataset.counts.excluded_policy_events}；不可用 {preview.dataset.counts.excluded_unavailable_events}；时间排除 {preview.dataset.counts.excluded_time_events}；范围外 {preview.dataset.counts.out_of_range_events}。</p>
          <p className="muted">{preview.notice}</p>
          <button type="button" disabled={saving} onClick={confirm}>确认保存这份研究数据</button>
        </div>
      )}
      {receipts.map((receipt) => (
        <div key={receipt.request_id} data-testid="research-save-receipt" role="status">
          <p>{receipt.written_to_disk ? "已经保存" : "收据未确认文件已保存"}：{receipt.destination}</p>
          <p>请求编号：{receipt.request_id}；{receipt.row_count} 行，{receipt.included_events} 个事件。</p>
          {!receipt.transaction_closed && <p>事务结束未确认；文件发布结果以本收据为准，不要将取消理解为删除。</p>}
        </div>
      ))}
    </section>
  );
}
