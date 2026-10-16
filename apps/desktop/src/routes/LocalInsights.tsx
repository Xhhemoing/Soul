import { useEffect, useRef, useState } from "react";
import { getDraftSourceRevision, ignoreLocalInsight, localInsights, recomputeLocalInsight, subscribeDraftSource, type LocalInsight } from "../core";
import { asRefusal } from "../refusal";

/** Automatic local candidates, exact-instance feedback, and explicit recomputation. */
export function LocalInsights(): React.JSX.Element {
  const [rows, setRows] = useState<readonly LocalInsight[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false);
  const mount = useRef<symbol | null>(null); const active = useRef<symbol | null>(null);

  function run(work: () => Promise<readonly LocalInsight[]>): void {
    if (active.current !== null) return;
    const ticket = Symbol(); const life = mount.current; const source = getDraftSourceRevision();
    active.current = ticket; setBusy(true); setError(null);
    void Promise.resolve().then(() => {
      if (life === null || life !== mount.current || active.current !== ticket || source !== getDraftSourceRevision()) throw new Error("已失效的本地查询");
      return work();
    }).then((value) => {
      if (life === mount.current && active.current === ticket && source === getDraftSourceRevision()) { setRows(value); setChanged(false); }
    }, (error: unknown) => {
      if (life === mount.current && source === getDraftSourceRevision()) setError(asRefusal(error).explanation);
    }).finally(() => {
      if (active.current === ticket && life === mount.current) { active.current = null; setBusy(false); }
    });
  }
  useEffect(() => {
    const life = Symbol(); mount.current = life;
    let source = getDraftSourceRevision();
    const unsubscribe = subscribeDraftSource((next) => {
      if (source === next) return; source = next; setRows(null); setChanged(true); setError(null);
    });
    // A separate ticket owns this mount's first read, including StrictMode.
    const ticket = Symbol(); active.current = ticket;
    const initialSource = source;
    void localInsights().then((value) => {
      if (mount.current === life && active.current === ticket && initialSource === getDraftSourceRevision()) setRows(value);
    }, () => {}).finally(() => { if (mount.current === life && active.current === ticket) active.current = null; });
    return () => { if (mount.current === life) mount.current = null; unsubscribe(); };
  }, []);

  return <section className="panel">
    <h2>导入后的本地洞察与反馈</h2>
    <p>成功导入后在本机生成统计候选，复用现有图谱和人物摘要算法。不会因为已配置模型端点就自动出网。
      忽略只作用于这一份候选，不改变事实、不训练模型；纠正仍由图谱中的用户锁生效。</p>
    <button type="button" disabled={busy} onClick={() => run(localInsights)}>核对本地洞察</button>
    {changed ? <p role="status">资料或授权已变更，旧洞察已移出展示。请重新核对；失效候选需明确重新计算。</p> : null}
    {error === null ? null : <p role="alert">{error}</p>}
    {rows?.length === 0 ? <p>还没有持久化本地候选。完成一次新协议导入后会生成。</p> : null}
    {rows?.map((row) => <article key={row.bundle_id}>
      <h3>本地候选 {row.bundle_id}</h3><p>导入操作：{row.operation_id}</p>
      <p>历史回执：新增 {row.counts.events_written} 条事件、跳过重复 {row.counts.duplicates} 条。状态：{row.validity}；{row.ignored ? "已忽略" : "尚未忽略"}。</p>
      <p>{row.notice}</p>
      {row.validity !== "active" || row.ignored ? null : row.summaries.map((summary) => <div key={summary.contact_id}>
        <p>{summary.text}</p><ul>{summary.points.map((point, index) => <li key={index}>{point.statement} <span>依据：{point.evidence_ids.join("、")}</span></li>)}</ul>
      </div>)}
      <button type="button" disabled={busy || row.ignored} onClick={() => run(async () => { await ignoreLocalInsight(row.bundle_id); return localInsights(); })}>忽略候选 {row.bundle_id}</button>
      <button type="button" disabled={busy} onClick={() => run(async () => { await recomputeLocalInsight(row.bundle_id); return localInsights(); })}>重新计算候选 {row.bundle_id}</button>
      {" "}<a href="#/graph">查看证据并纠正关系档</a>
    </article>)}
  </section>;
}
