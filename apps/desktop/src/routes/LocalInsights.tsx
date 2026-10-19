import { useEffect, useRef, useState } from "react";
import { getDraftSourceRevision, ignoreLocalInsight, localInsights, recomputeLocalInsight, subscribeDraftSource, type LocalInsight } from "../core";
import { asRefusal } from "../refusal";
import { graphTargetHash } from "./graphTarget";
import { draftInsightSelection } from "./draftMaterialSelection";
import { getRouteAddress, useRouteAddress, type RouteAddress } from "../routeAddress";

/** Automatic local candidates, exact-instance feedback, and explicit recomputation. */
export function LocalInsights(): React.JSX.Element {
  const [rows, setRows] = useState<{ readonly value: readonly LocalInsight[]; readonly address: RouteAddress; readonly source: symbol } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false);
  const [filter, setFilter] = useState("all");
  const [contactFilter, setContactFilter] = useState("");
  const address = useRouteAddress();
  const mount = useRef<symbol | null>(null); const active = useRef<symbol | null>(null);
  const [renderedSource, setRenderedSource] = useState(getDraftSourceRevision);

  function run(mutate?: () => Promise<unknown>): void {
    if (active.current !== null || renderedSource !== getDraftSourceRevision() || address !== getRouteAddress()) return;
    const ticket = Symbol(); const life = mount.current; const source = renderedSource;
    const owns = (): boolean => life !== null && life === mount.current && active.current === ticket && address === getRouteAddress();
    let readSource = source;
    active.current = ticket; setBusy(true); setError(null);
    void Promise.resolve().then(async () => {
      if (!owns() || source !== getDraftSourceRevision()) return null;
      if (mutate !== undefined) {
        // Only this explicit mutation's completion starts a fresh read intent.
        // A read that is already in flight never receives a new revision.
        try { await mutate(); } finally { readSource = getDraftSourceRevision(); }
      }
      if (!owns() || readSource !== getDraftSourceRevision()) return null;
      return localInsights();
    }).then((value) => {
      if (value !== null && owns() && readSource === getDraftSourceRevision()) { setRows({ value, address, source: readSource }); setChanged(false); }
    }, (error: unknown) => {
      if (owns() && readSource === getDraftSourceRevision()) setError(asRefusal(error).explanation);
    }).finally(() => {
      if (active.current === ticket && life === mount.current) { active.current = null; setBusy(false); }
    });
  }
  useEffect(() => {
    const life = Symbol(); mount.current = life; const initialAddress = getRouteAddress();
    let source = getDraftSourceRevision();
    const unsubscribe = subscribeDraftSource((next) => {
      setRenderedSource(next);
      if (source === next) return; source = next; setRows(null); setChanged(true); setError(null);
    });
    // A separate ticket owns this mount's first read, including StrictMode.
    const ticket = Symbol(); active.current = ticket;
    const initialSource = source;
    void localInsights().then((value) => {
      if (mount.current === life && active.current === ticket && initialSource === getDraftSourceRevision() && initialAddress === getRouteAddress()) setRows({ value, address: initialAddress, source: initialSource });
    }, (error: unknown) => {
      if (mount.current === life && active.current === ticket && initialSource === getDraftSourceRevision() && initialAddress === getRouteAddress()) setError(asRefusal(error).explanation);
    }).finally(() => { if (mount.current === life && active.current === ticket) active.current = null; });
    return () => { if (mount.current === life) mount.current = null; unsubscribe(); };
  }, []);

  const displayRows = rows?.address === address && address === getRouteAddress() && rows.source === getDraftSourceRevision() ? rows.value : null;
  const visibleRows = displayRows?.filter((row) => (filter === "all" || (filter === "current" && row.validity === "active" && !row.ignored)
    || (filter === "ignored" && row.ignored) || (filter === "invalidated" && row.validity === "invalidated"))
    && (contactFilter.trim() === "" || row.summaries.some((summary) => summary.contact_id.toLowerCase().includes(contactFilter.trim().toLowerCase()))));

  return <section className="panel">
    <h2>导入后的本地洞察与反馈</h2>
    <p>成功导入后在本机生成统计候选，复用现有图谱和人物摘要算法。不会因为已配置模型端点就自动出网。
      忽略只作用于这一份候选，不改变事实、不训练模型；纠正仍由图谱中的用户锁生效。</p>
    <button type="button" disabled={busy} onClick={() => run()}>核对本地洞察</button>
    <p><a href="#/assistance">打开辅助事项收件箱</a>；这里的筛选只改变本次已读取候选的展示，不读取消息正文、不提交反馈。</p>
    <label htmlFor="insight-status">候选展示范围</label><select id="insight-status" value={filter} onChange={(event) => setFilter(event.target.value)}>
      <option value="all">全部已读取候选</option><option value="current">当前且未忽略</option><option value="ignored">已忽略</option><option value="invalidated">已失效</option>
    </select>
    <label htmlFor="insight-contact">在已读取候选中筛联系人编号</label><input id="insight-contact" value={contactFilter} onChange={(event) => setContactFilter(event.target.value)} />
    {changed ? <p role="status">资料或授权已变更，旧洞察已移出展示。请重新核对；失效候选需明确重新计算。</p> : null}
    {error === null ? null : <p role="alert">{error}</p>}
    {displayRows?.length === 0 ? <p>还没有持久化本地候选。完成一次新协议导入后会生成。</p> : null}
    {displayRows !== null && displayRows.length > 0 && visibleRows?.length === 0 ? <p>本次读取中没有符合展示筛选的候选。</p> : null}
    {visibleRows?.map((row) => <article key={row.bundle_id}>
      <h3>本地候选 {row.bundle_id}</h3><p>导入操作：{row.operation_id}</p>
      <p>历史回执：新增 {row.counts.events_written} 条事件、跳过重复 {row.counts.duplicates} 条。状态：{row.validity}；{row.ignored ? "已忽略" : "尚未忽略"}。</p>
      <p>{row.notice}</p>
      {row.validity !== "active" || row.ignored ? null : row.summaries.map((summary) => {
        const target = graphTargetHash(summary.contact_id, summary.points.flatMap((point) => point.evidence_ids));
        return <div key={summary.contact_id}>
          <p>联系对象：<code>{summary.contact_id}</code></p>
          <p>{summary.text}</p><ul>{summary.points.map((point, index) => <li key={index}>{point.statement} <span>依据：{point.evidence_ids.join("、")}</span></li>)}</ul>
          {target === null ? <p>这份摘要的引用为空、格式不支持或超出定位上限，未生成精确定位。请到整个人脉图核对。</p>
            : <a href={target}>核对此联系人的证据条目（元数据）</a>}
          <button type="button" disabled={busy} onClick={() => {
            if (mount.current === null || renderedSource !== getDraftSourceRevision() || address !== getRouteAddress()) return;
            draftInsightSelection.select(row.bundle_id, summary.contact_id);
            window.location.hash = "#/draft";
          }}>选作本次起草统计参考（替换记忆参考）：{summary.contact_id}</button>
        </div>;
      })}
      <button type="button" disabled={busy || row.ignored} onClick={() => run(() => ignoreLocalInsight(row.bundle_id))}>忽略候选 {row.bundle_id}</button>
      <button type="button" disabled={busy} onClick={() => run(() => recomputeLocalInsight(row.bundle_id))}>重新计算候选 {row.bundle_id}</button>
      {" "}<a href="#/graph">查看整个人脉图</a>
    </article>)}
  </section>;
}
