import { useEffect, useRef, useState } from "react";
import { DraftSourceChangedError, getDraftSourceRevision, prepareAssistanceContext, type AssistanceContextView, type AssistancePointer } from "../core";
import { asRefusal } from "../refusal";
import { getRouteAddress, useRouteAddress } from "../routeAddress";
import { assistanceHash } from "./assistanceTarget";
import { draftAssistanceSelection } from "./draftMaterialSelection";
import { draftLane, isDraftRefusal } from "./draftRequests";
import { MemoryReferences, type MemoryTask } from "./DraftMemory";
import { InsightReferences, type InsightTask } from "./DraftInsights";

export function DraftAssistance({ selection, context, memory, insight, busy, hasQuote, getHasQuote, getPasteGeneration, onPrepared, onMemory, onInsight }: {
  readonly selection: AssistancePointer; readonly context: AssistanceContextView | null;
  readonly memory: MemoryTask | null; readonly insight: InsightTask | null;
  readonly busy: boolean; readonly hasQuote: boolean; readonly getHasQuote: () => boolean; readonly getPasteGeneration: () => number;
  readonly onPrepared: (value: AssistanceContextView) => void;
  readonly onMemory: (value: MemoryTask) => void; readonly onInsight: (value: InsightTask) => void;
}): React.JSX.Element {
  const address = useRouteAddress(); const renderedSource = getDraftSourceRevision(); const paste = getPasteGeneration();
  const mount = useRef<symbol | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { const life = Symbol("assistance draft"); mount.current = life; return () => { if (mount.current === life) mount.current = null; }; }, []);
  const owns = (): boolean => mount.current !== null && address === getRouteAddress()
    && selection === draftAssistanceSelection.snapshot() && renderedSource === getDraftSourceRevision() && paste === getPasteGeneration();
  const prepare = (): void => {
    if (!owns() || busy || hasQuote || getHasQuote()) return;
    const slot = draftLane.begin("prepare"); if (slot === null) return;
    const life = mount.current; let source = renderedSource; let unknown = false;
    const current = (): boolean => life !== null && life === mount.current && address === getRouteAddress()
      && selection === draftAssistanceSelection.snapshot() && paste === getPasteGeneration();
    setError(null);
    void Promise.resolve().then(async () => {
      if (!current() || source !== getDraftSourceRevision()) return;
      const pending = prepareAssistanceContext(selection); source = getDraftSourceRevision();
      const value = await pending;
      if (current() && source === getDraftSourceRevision()) onPrepared(value);
    }).catch((problem: unknown) => {
      unknown = !isDraftRefusal(problem) && !(problem instanceof DraftSourceChangedError);
      if (current() && source === getDraftSourceRevision()) setError(asRefusal(problem).explanation);
    }).finally(() => { draftLane.finish(slot, unknown); });
  };
  return <section className="panel"><h2>本次辅助事项</h2>
    <p>事项 {selection.item_id}。请核对当前目标与参考材料；尚未授权端点使用。</p>
    <p><a href={assistanceHash(selection.item_id)}>打开事项的当前版本</a>。在事项页编辑目标会修改已保存目标。目标编辑、完成、取消、改期或来源变化后，旧准备不能继续使用。</p>
    <button type="button" disabled={busy || hasQuote} onClick={prepare}>核对并准备</button>
    <button type="button" disabled={busy} onClick={() => { if (owns()) draftAssistanceSelection.clear(); }}>清除事项，使用普通起草</button>
    {error === null ? null : <p role="alert">{error}</p>}
    {context === null ? <p role="status">请先核对当前事项，再决定本次怎样使用参考材料。</p> : null}
    {memory === null ? null : <><MemoryReferences preview={memory.context.preview} />
      {memory.context.context_id === null ? <p role="status">事项记忆用途已消费或失效，请重新核对。</p> : memory.context.preview.selected.map((entry) => entry.privacy_class !== "owner" ? <p key={entry.memory_id}>记录 {entry.memory_id} 仅供本地参考。</p> : <label key={entry.memory_id}>
        <input type="checkbox" checked={memory.owner_ids.includes(entry.memory_id)} disabled={busy || hasQuote} onChange={(event) => {
          if (!owns() || getHasQuote() || draftLane.snapshot().busy) return;
          onMemory({ ...memory, owner_ids: event.target.checked ? [...memory.owner_ids, entry.memory_id] : memory.owner_ids.filter((id) => id !== entry.memory_id) });
        }} />本次另行允许把这条本人记录的脱敏节选提供给端点：{entry.title}（仍需最终确认）
      </label>)}
    </>}
    {insight === null ? null : <><InsightReferences reference={insight.reference} />
      {insight.goal_preview === null ? <p>本次没有可纳入的本人目标正文。</p> : <>
        <MemoryReferences preview={insight.goal_preview} />
        {insight.goal_preview.selected.length === 1 && insight.goal_preview.selected[0]?.privacy_class === "owner" ? <label>
          <input type="checkbox" checked={insight.include_goal} disabled={busy || hasQuote || insight.context_id === null} onChange={(event) => {
            if (!owns() || getHasQuote() || draftLane.snapshot().busy || insight.context_id === null) return;
            onInsight({ ...insight, include_goal: event.target.checked });
          }} />本次另行允许把这条本人目标的脱敏节选提供给端点（仍需最终确认）
        </label> : <p>此目标当前不可纳入端点材料，正文仅供本地核对；匿名计数可独立选择。</p>}
        {insight.include_goal ? null : <p>未勾选时目标正文保持本地，不随计数或粘贴原文的许可提供给端点。</p>}
      </>}
      <label><input type="checkbox" checked={insight.include_counts} disabled={busy || hasQuote || insight.context_id === null} onChange={(event) => {
        if (!owns() || getHasQuote() || draftLane.snapshot().busy || insight.context_id === null) return;
        onInsight({ ...insight, include_counts: event.target.checked });
      }} />本次另行允许将这份匿名派生计数提供给端点（仍需最终确认）</label>
      {insight.context_id === null ? <p role="status">事项统计用途已消费或失效，请重新核对。</p> : null}
    </>}
  </section>;
}
