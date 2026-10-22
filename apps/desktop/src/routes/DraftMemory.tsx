import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { getDraftSourceRevision, prepareMemoryContext, prepareTaskContext, type MemoryContextView, type MemorySelectionPreview, type TaskAssessment } from "../core";
import { asRefusal } from "../refusal";
import { draftMemoryCandidates } from "./draftMemorySelection";
import { draftLane } from "./draftRequests";
import { draftMaterialSelection } from "./draftMaterialSelection";
import { getRouteAddress, useRouteAddress } from "../routeAddress";

export interface MemoryTask { readonly context: MemoryContextView; readonly owner_ids: readonly string[] }

export function DraftMemory({ value, busy, hasQuote, onChange, onEdit, getPasteGeneration, getHasQuote }: {
  readonly value: MemoryTask | null; readonly busy: boolean; readonly hasQuote: boolean;
  readonly onChange: (value: MemoryTask | null) => void; readonly onEdit: () => void;
  readonly getPasteGeneration: () => number;
  readonly getHasQuote: () => boolean;
}): React.JSX.Element {
  const ids = useSyncExternalStore(draftMemoryCandidates.subscribe, draftMemoryCandidates.snapshot);
  const [query, setQuery] = useState(""); const [kind, setKind] = useState(""); const [time, setTime] = useState(false);
  const [task, setTask] = useState("");
  const [checkedTask, setCheckedTask] = useState<{ readonly context: MemoryContextView; readonly assessment: TaskAssessment } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const mount = useRef<symbol | null>(null); const revision = useRef(0);
  const address = useRouteAddress();
  const renderedSource = getDraftSourceRevision(); const renderedPaste = getPasteGeneration();
  useEffect(() => { const id = Symbol(); mount.current = id; return () => { if (mount.current === id) mount.current = null; }; }, []);
  const ownsControls = (): boolean => mount.current !== null && address === getRouteAddress()
    && ids === draftMemoryCandidates.snapshot() && renderedSource === getDraftSourceRevision() && renderedPaste === getPasteGeneration();
  const edit = (): void => { revision.current += 1; setError(null); setCheckedTask(null); onEdit(); };
  const preview = (checkTask = false): void => {
    if (checkTask !== (task.trim() !== "")) return;
    if (busy || hasQuote || getHasQuote() || !ownsControls() || ids !== draftMemoryCandidates.snapshot()
      || renderedSource !== getDraftSourceRevision() || renderedPaste !== getPasteGeneration()) return;
    const slot = draftLane.begin("prepare"); if (slot === null) return;
    const life = mount.current; const input = revision.current; const paste = getPasteGeneration();
    const selection = draftMaterialSelection.generation();
    const current = (): boolean => life !== null && mount.current === life && input === revision.current && paste === getPasteGeneration()
      && selection === draftMaterialSelection.generation() && address === getRouteAddress();
    let deliverySource = renderedSource;
    setError(null); setCheckedTask(null);
    void Promise.resolve().then(async () => {
      if (!current() || renderedSource !== getDraftSourceRevision()) return;
      const result = checkTask
        ? prepareTaskContext({ candidate_ids: ids, task: task.trim() })
        : prepareMemoryContext({ candidate_ids: ids, query: query.trim() || null, memory_type: kind || null, time_specific: time });
      deliverySource = getDraftSourceRevision();
      const response = await result;
      const context = "memory" in response ? response.memory : response;
      if (context.preview.selected.some((entry) => !ids.includes(entry.memory_id) || entry.citation.memory_id !== entry.memory_id)
        || context.preview.excluded.some((entry) => !ids.includes(entry.memory_id))) throw new Error("记忆返回的候选身份不符，请重新核对。");
      if (current() && deliverySource === getDraftSourceRevision()) {
        onChange({ context, owner_ids: [] });
        if ("assessment" in response) setCheckedTask({ context, assessment: response.assessment });
      }
    }).catch((error: unknown) => {
      if (current() && deliverySource === getDraftSourceRevision()) setError(asRefusal(error).explanation);
    }).finally(() => { draftLane.finish(slot, false); });
  };
  return <section className="panel">
    <h2>本次手选记忆参考</h2>
    <p>在<a href="#/memory">记忆页</a>打开一条记录并选择“用于本次起草”。这里只处理你明确选择的候选，不自动全库检索。候选只在当前窗口保留编号，重启清空。</p>
    <ul>{ids.map((id) => <li key={id}><code>{id}</code> <button type="button" disabled={busy} onClick={() => { if (!ownsControls()) return; edit(); draftMemoryCandidates.remove(id); }}>移出本次候选 {id}</button></li>)}</ul>
    <label htmlFor="task-information">本次需要核对的信息</label>
    <textarea id="task-information" maxLength={256} value={task} disabled={busy || hasQuote}
      placeholder={"周五会议\n需要确认：时间、地点"} onChange={(event) => { edit(); setTask(event.target.value); }} />
    <p>可选：用一行“需要确认：时间、地点”列出本次必需信息。当前只核对候选摘要中逐行“字段：值”的字面线索；事项、主体和适用时间仍需你核对。任务文字保持本地。</p>
    <button type="button" disabled={busy || hasQuote || ids.length === 0 || task.trim() === ""} onClick={() => { preview(true); }}>识别信息要求并核对材料</button>
    <label htmlFor="memory-query">仅在本次候选内查关键词</label><input id="memory-query" maxLength={256} value={query} disabled={busy || hasQuote || task.trim() !== ""} onChange={(event) => { edit(); setQuery(event.target.value); }} />
    <label htmlFor="context-kind">记忆类型筛选</label><select id="context-kind" value={kind} disabled={busy || hasQuote || task.trim() !== ""} onChange={(event) => { edit(); setKind(event.target.value); }}>
      <option value="">全部本次候选</option>{["episodic", "semantic", "procedural", "preference", "commitment"].map((type) => <option key={type} value={type}>{type}</option>)}
    </select>
    <label><input type="checkbox" checked={time} disabled={busy || hasQuote || task.trim() !== ""} onChange={(event) => { edit(); setTime(event.target.checked); }} />这是限定事实发生时间的问题（当前缺事实有效时间，会要求澄清）</label>
    <button type="button" disabled={busy || hasQuote || ids.length === 0 || task.trim() !== ""} onClick={() => { preview(); }}>预览本次记忆与出处</button>
    <button type="button" disabled={busy} onClick={() => { if (!ownsControls()) return; edit(); setTask(""); onChange(null); }}>清除本次记忆，使用普通起草</button>
    {error === null ? null : <p role="alert">{error}</p>}
    {checkedTask !== null && checkedTask.context === value?.context ? <TaskCheck assessment={checkedTask.assessment} /> : null}
    {value === null ? null : <>
      <MemoryReferences preview={value.context.preview} />
      {value.context.context_id === null ? <p role="status">这次没有可用的记忆许可。请先澄清或明确清除本次记忆，选择普通模板；不会把空集合当成有依据的回答。</p> : value.context.preview.selected.map((entry) => entry.privacy_class !== "owner" ? <p key={entry.memory_id}>记录 {entry.memory_id}：仅供本地参考，第三人、混合或未知正文不会发往端点。</p> : <label key={entry.memory_id}>
        <input type="checkbox" checked={value.owner_ids.includes(entry.memory_id)} disabled={busy || hasQuote} onChange={(event) => {
          // There is no held outbound plan while this control is editable.
          if (getHasQuote() || draftLane.snapshot().busy || !ownsControls() || ids !== draftMemoryCandidates.snapshot()
            || renderedSource !== getDraftSourceRevision() || renderedPaste !== getPasteGeneration()) return;
          onChange({ ...value, owner_ids: event.target.checked ? [...value.owner_ids, entry.memory_id] : value.owner_ids.filter((id) => id !== entry.memory_id) });
        }} />本次另行允许把这条本人记录的脱敏节选提供给端点：{entry.title}（仍需最终确认）
      </label>)}
    </>}
  </section>;
}

function TaskCheck({ assessment }: { readonly assessment: TaskAssessment }): React.JSX.Element {
  const statuses = { matched: "已找到所列信息的字面线索", clarify: "需要确认", unknown: "未确定", budget_limited: "材料超出本次预算" };
  const needs = { matched: "找到字面线索", missing: "本次已检查范围未找到", conflict: "字面值不一致", unknown: "未确定" };
  return <div aria-label="任务信息核对结果">
    <p role="status">核对状态：{statuses[assessment.status]}</p><p>{assessment.notice}</p>
    <ul>{assessment.requirements.map((need) => <li key={need.field}>{need.field}：{needs[need.status]}{need.source_ids.length === 0 ? "" : `；出处 ${need.source_ids.join("、")}`}</li>)}</ul>
    {assessment.supplemented_ids.length === 0 ? null : <p>已从本次候选补入：{assessment.supplemented_ids.join("、")}。请核对下方材料。</p>}
    {assessment.question === null ? null : <><h3>需要你确认</h3><p>{assessment.question}</p>
      <p>请补充或修正本次候选后重新核对，也可调整本次信息要求。这里不会自动保存回答或写入长期记忆。</p></>}
  </div>;
}

export function MemoryReferences({ preview }: { readonly preview: MemorySelectionPreview }): React.JSX.Element {
  return <div><p>{preview.notice}</p><p>选择结果：{preview.outcome}</p>
    {preview.selected.map((entry) => <article key={entry.memory_id}><h3>{entry.title}</h3><p className="pre-wrap">{entry.summary}</p>
      <p>本地记录出处：记忆 {entry.citation.memory_id}；标题 blob {entry.citation.title_blob_id}；摘要 blob {entry.citation.summary_blob_id}。</p>
    </article>)}
    {preview.excluded.length === 0 ? null : <ul>{preview.excluded.map((entry, index) => <li key={`${entry.memory_id}-${index}`}>{entry.memory_id}：{entry.reason}</li>)}</ul>}
    <p>这些出处只证明提供了哪些保存材料，不代表模型每句话已获事实验证。</p>
  </div>;
}
