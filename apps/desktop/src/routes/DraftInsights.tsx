import { useEffect, useRef, useState } from "react";
import { DraftSourceChangedError, getDraftSourceRevision, prepareInsightContext, type InsightReference } from "../core";
import { asRefusal } from "../refusal";
import { draftInsightSelection, type InsightSelection } from "./draftMaterialSelection";
import { draftLane, isDraftRefusal } from "./draftRequests";
import { getRouteAddress, useRouteAddress } from "../routeAddress";

export interface InsightTask {
  readonly context_id: string | null;
  readonly reference: InsightReference;
  readonly include_counts: boolean;
}

export function DraftInsights({ selection, value, busy, canClear, hasQuote, getHasQuote, getPasteGeneration, onChange, onClear }: {
  readonly selection: InsightSelection;
  readonly value: InsightTask | null;
  readonly busy: boolean;
  readonly canClear: boolean;
  readonly hasQuote: boolean;
  readonly getHasQuote: () => boolean;
  readonly getPasteGeneration: () => number;
  readonly onChange: (value: InsightTask) => void;
  readonly onClear: () => void;
}): React.JSX.Element {
  const [error, setError] = useState<string | null>(null);
  const mount = useRef<symbol | null>(null);
  const address = useRouteAddress();
  const renderedSource = getDraftSourceRevision(); const renderedPaste = getPasteGeneration();
  useEffect(() => { const life = Symbol(); mount.current = life; return () => { if (mount.current === life) mount.current = null; }; }, []);
  const preview = (): void => {
    if (busy || hasQuote || getHasQuote() || address !== getRouteAddress() || selection !== draftInsightSelection.snapshot()
      || renderedSource !== getDraftSourceRevision() || renderedPaste !== getPasteGeneration()) return;
    const slot = draftLane.begin("prepare"); if (slot === null) return;
    const life = mount.current; const paste = renderedPaste;
    const current = (): boolean => life !== null && mount.current === life && selection === draftInsightSelection.snapshot()
      && address === getRouteAddress() && paste === getPasteGeneration();
    setError(null);
    let unknown = false;
    let deliverySource = renderedSource;
    void Promise.resolve().then(async () => {
      if (!current() || renderedSource !== getDraftSourceRevision()) return;
      const result = prepareInsightContext({ bundle_id: selection.bundle_id, contact_id: selection.contact_id });
      deliverySource = getDraftSourceRevision();
      const context = await result;
      if (context.reference.bundle_id !== selection.bundle_id || context.reference.contact_id !== selection.contact_id
        || context.reference.summary.contact_id !== selection.contact_id) throw new Error("统计返回的来源身份不符，请重新核对。");
      if (current() && deliverySource === getDraftSourceRevision()) onChange({ ...context, include_counts: false });
    }).catch((error: unknown) => {
      unknown = !isDraftRefusal(error) && !(error instanceof DraftSourceChangedError);
      if (current() && deliverySource === getDraftSourceRevision()) setError(asRefusal(error).explanation);
    }).finally(() => { draftLane.finish(slot, unknown); });
  };
  return <section className="panel">
    <h2>本次选择的本地统计参考</h2>
    <p>候选 <code>{selection.bundle_id}</code>；联系人 <code>{selection.contact_id}</code>。此选择替换手选记忆；只保留编号，尚未授权端点使用。</p>
    <p><a href="#/insights">重新选择本地洞察或联系人</a>；在记忆页选择记忆会替换这份统计参考。</p>
    <p>统计来自当前整库的有效关系记录，不限于产生这份候选的单次导入。只把统计作为材料，不展示或提供聊天正文。</p>
    <button type="button" disabled={busy || hasQuote} onClick={preview}>预览本次统计与出处</button>
    <button type="button" disabled={!canClear} onClick={() => {
      if (mount.current !== null && selection === draftInsightSelection.snapshot() && address === getRouteAddress()
        && renderedSource === getDraftSourceRevision() && renderedPaste === getPasteGeneration()) onClear();
    }}>清除本次统计，使用普通起草</button>
    {error === null ? null : <p role="alert">{error}</p>}
    {value === null ? <p role="status">请先明确预览这份统计；不会把未取得的材料当成普通有依据起草。</p> : <>
      <InsightReferences reference={value.reference} />
      <label><input type="checkbox" checked={value.include_counts} disabled={busy || hasQuote || value.context_id === null}
        onChange={(event) => {
          if (!hasQuote && !getHasQuote() && !draftLane.snapshot().busy && value.context_id !== null
            && mount.current !== null && selection === draftInsightSelection.snapshot() && address === getRouteAddress()
            && renderedSource === getDraftSourceRevision() && renderedPaste === getPasteGeneration()) onChange({ ...value, include_counts: event.target.checked });
        }} />本次另行允许将这份匿名 A2 派生计数提供给端点（仍需最终确认）</label>
      {value.include_counts ? null : <p>未勾选时模型不会获得这些计数，统计只供你在本地核对。</p>}
      {value.context_id === null ? <p role="status">本次统计用途已消费或失效，请重新预览或明确清除后使用普通起草。</p> : null}
    </>}
  </section>;
}

export function InsightReferences({ reference }: { readonly reference: InsightReference }): React.JSX.Element {
  return <div>
    <h3>本次统计材料出处</h3>
    <p>{reference.notice}</p>
    <p>候选 {reference.bundle_id}；导入操作 {reference.operation_id}；命名空间 {reference.namespace_id}；联系人 {reference.contact_id}；关系 {reference.relationship_id}。</p>
    <p>{reference.summary.text}</p>
    <ul>{reference.summary.points.map((point, index) => <li key={index}>{point.statement} <span>依据：{point.evidence_ids.join("、")}</span></li>)}</ul>
    <details><summary>核对本地证据条目与事件编号</summary><ul>{reference.sources.map((source) => <li key={source.evidence_id}>
      证据 {source.evidence_id}；类别 {source.kind}；事件 {source.event_ids.join("、") || "无关联事件"}。
    </li>)}</ul></details>
    <p>这些出处只说明本次提供的统计材料，不证明整篇草稿每句话都有事实依据。本机模板不会根据统计改写。</p>
  </div>;
}
