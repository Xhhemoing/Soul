import { useCallback, useEffect, useRef, useState } from "react";
import {
  actAssistanceItem, assistanceItem, assistanceItems, bindAssistanceGoalSource, createAssistanceGoal,
  editAssistanceGoal, getDraftSourceRevision, linkAssistanceObservation, localInsights, refreshAssistanceObservation,
  subscribeDraftSource, type AssistanceGoalFields, type AssistanceItemDetail, type AssistanceItemRow,
  type AssistancePointer, type LocalInsight,
} from "../core";
import { asRefusal } from "../refusal";
import { getRouteAddress, useRouteAddress, type RouteAddress } from "../routeAddress";
import { assistanceHash, assistanceTarget } from "./assistanceTarget";
import { assistanceWrites, useAssistanceWrites } from "./assistanceRequests";
import { draftAssistanceSelection } from "./draftMaterialSelection";
import { isDraftRefusal } from "./draftRequests";
import { AssistanceItemWork, AssistanceWork } from "./AssistanceWork";
import { AssistanceFeedback, AssistanceNotifications, AssistanceReminder } from "./AssistanceNotifications";
import { AssistanceLocalPreparationPanel } from "./AssistanceLocalPreparation";

const HANDLING = { open: "待处理", cancelled: "已取消", resolved_reported: "已自报完成" };
const VALIDITY = { current: "可打开核对", stale: "来源需重新核对", forgotten: "已遗忘" };
const EMPTY: AssistanceGoalFields = { title: "", summary: "", deadline_at: null };
type AssistanceOwner = { readonly address: RouteAddress; readonly source: symbol };
interface Owned<T> { readonly value: T; readonly owner: AssistanceOwner }

export function Assistance(): React.JSX.Element {
  const address = useRouteAddress();
  const target = assistanceTarget(address.hash);
  const [storedRows, setRows] = useState<Owned<readonly AssistanceItemRow[]> | null>(null);
  const [storedDetail, setDetail] = useState<Owned<AssistanceItemDetail> | null>(null);
  const detailRef = useRef<AssistanceItemDetail | null>(null);
  const [storedFields, storeFields] = useState<Owned<AssistanceGoalFields> | null>(null);
  const [createSource, setCreateSource] = useState("");
  const [bindSource, setBindSource] = useState("");
  const [storedCandidates, setCandidates] = useState<Owned<readonly LocalInsight[]> | null>(null);
  const [candidate, setCandidate] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [reading, setReading] = useState(false);
  const [source, setSource] = useState(getDraftSourceRevision);
  const life = useRef<symbol | null>(null);
  const readTicket = useRef<symbol | null>(null);
  const writes = useAssistanceWrites();
  const detailOwner = useRef<AssistanceOwner | null>(null);
  const ownsPayload = (owner: AssistanceOwner | null): boolean => owner !== null && owner.address === address
    && address === getRouteAddress() && owner.source === getDraftSourceRevision();
  const rows = storedRows !== null && ownsPayload(storedRows.owner) ? storedRows.value : null;
  const detail = storedDetail !== null && ownsPayload(storedDetail.owner) ? storedDetail.value : null;
  const candidates = storedCandidates !== null && ownsPayload(storedCandidates.owner) ? storedCandidates.value : null;
  const fields = storedFields !== null && ownsPayload(storedFields.owner) ? storedFields.value : EMPTY;
  const setFields = useCallback((value: AssistanceGoalFields): void => {
    storeFields({ value, owner: { address, source: getDraftSourceRevision() } });
  }, [address]);

  const clearDetail = useCallback((): void => { detailRef.current = null; detailOwner.current = null; setDetail(null); setFields(EMPTY); setBindSource(""); }, [setFields]);
  const current = (): boolean => life.current !== null && address === getRouteAddress() && source === getDraftSourceRevision();
  const acceptDetail = useCallback((value: AssistanceItemDetail): void => {
    const owner = { address, source: getDraftSourceRevision() }; detailOwner.current = owner;
    detailRef.current = value; setDetail({ value, owner });
    setFields(value.goal === null ? EMPTY : { title: value.goal.title, summary: value.goal.summary, deadline_at: value.item.deadline_at });
    setBindSource(value.item.source.kind === "manual_goal" ? value.item.source.observation?.item_id ?? "" : "");
  }, [address, setFields]);

  const read = (kind: "list" | "item" | "candidates", itemId?: string): void => {
    if (!current() || readTicket.current !== null) return;
    const ticket = Symbol("assistance read"); const mount = life.current; const initialSource = source;
    readTicket.current = ticket; setReading(true); setError(null);
    const owns = (): boolean => mount !== null && mount === life.current && readTicket.current === ticket
      && address === getRouteAddress() && initialSource === getDraftSourceRevision();
    void Promise.resolve().then(async () => {
      if (!owns()) return;
      if (kind === "list") {
        const value = await assistanceItems();
        if (owns()) {
          setRows({ value: value.items, owner: { address, source: initialSource } }); clearDetail(); setFields(EMPTY); setCreateSource("");
          assistanceWrites.checked(); setNotice("已重新读取当前事项；未重放任何写入。");
        }
      } else if (kind === "candidates") {
        const value = await localInsights(); if (owns()) { setCandidates({ value, owner: { address, source: initialSource } }); setCandidate(""); }
      } else if (itemId !== undefined) {
        const value = await assistanceItem(itemId); if (owns()) acceptDetail(value);
      }
    }).catch((problem: unknown) => { if (owns()) setError(asRefusal(problem).explanation); }).finally(() => {
      if (life.current === mount && readTicket.current === ticket) { readTicket.current = null; setReading(false); }
    });
  };

  useEffect(() => {
    const mount = Symbol("assistance page"); life.current = mount;
    const unsubscribe = subscribeDraftSource((next) => {
      setSource(next); setRows(null); detailRef.current = null; detailOwner.current = null;
      setDetail(null); storeFields(null); setBindSource(""); setCandidates(null); setCandidate("");
    });
    return () => { if (life.current === mount) life.current = null; unsubscribe(); };
  }, []);

  useEffect(() => {
    const mount = life.current; const expectedSource = getDraftSourceRevision(); const ticket = Symbol("assistance navigation");
    readTicket.current = ticket;
    const owns = (): boolean => mount !== null && mount === life.current && readTicket.current === ticket
      && address === getRouteAddress() && expectedSource === getDraftSourceRevision();
    void Promise.resolve().then(async () => {
      if (!owns()) return;
      setReading(true); clearDetail(); setError(null); setCreateSource(""); setCandidates(null); setCandidate("");
      if (target === "invalid") { setError("事项地址无效，请从收件箱重新选择。"); return; }
      const list = await assistanceItems(); if (owns()) setRows({ value: list.items, owner: { address, source: expectedSource } });
      if (target !== null && owns()) { const value = await assistanceItem(target); if (owns()) acceptDetail(value); }
    }).catch((problem: unknown) => { if (owns()) setError(asRefusal(problem).explanation); }).finally(() => {
      if (life.current === mount && readTicket.current === ticket) { readTicket.current = null; setReading(false); }
    });
    return () => { if (readTicket.current === ticket) readTicket.current = null; };
  }, [address, target, acceptDetail, clearDetail]);

  const write = (work: () => Promise<AssistanceItemDetail>, expected?: AssistanceItemDetail): void => {
    if (!current() || reading || (expected !== undefined && (detailRef.current !== expected || !ownsPayload(detailOwner.current)))) return;
    const slot = assistanceWrites.begin(); if (slot === null) return;
    const mount = life.current; const admittedSource = source; const admittedAddress = address;
    setError(null); setNotice(null);
    let started = false; let unknown = false;
    const owns = (): boolean => mount !== null && mount === life.current && admittedAddress === getRouteAddress();
    void Promise.resolve().then(async () => {
      if (!owns() || admittedSource !== getDraftSourceRevision() || (expected !== undefined && (detailRef.current !== expected || !ownsPayload(detailOwner.current)))) return;
      started = true;
      const value = await work();
      if (owns()) {
        clearDetail(); setRows(null); setFields(EMPTY);
        setNotice(`操作已提交：${value.item.item_id}。请重新读取当前事项，完成只表示你的自报。`);
      }
    }).catch((problem: unknown) => {
      unknown = started && !isDraftRefusal(problem);
      if (owns()) {
        clearDetail(); setRows(null);
        setError(unknown ? "这次写入结果未能确认。请核对事项列表；不会自动重放。" : asRefusal(problem).explanation);
      }
    }).finally(() => { assistanceWrites.finish(slot, unknown); });
  };
  const pointer = (row: AssistanceItemRow): AssistancePointer => ({ item_id: row.item_id, expected_revision: row.revision });
  const selectedObservation = (id: string): AssistancePointer | null => {
    const row = rows?.find((item) => item.item_id === id && item.source.kind === "observation" && item.validity === "current" && item.handling === "open");
    return row === undefined ? null : pointer(row);
  };
  const observations = rows?.filter((row) => row.source.kind === "observation" && row.validity === "current" && row.handling === "open") ?? [];
  const blocked = reading || writes.busy || writes.unknown;
  const editable = detail !== null && detail.item.source.kind === "manual_goal" && detail.item.handling === "open" && detail.item.validity !== "forgotten";
  const deadlineValid = fields.deadline_at === null || (!fields.deadline_at.endsWith("-00:00")
    && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(fields.deadline_at));
  const goalFields = <>
    <label htmlFor="assistance-title">目标标题</label><input id="assistance-title" value={fields.title} disabled={blocked} onChange={(event) => { if (current()) setFields({ ...fields, title: event.target.value }); }} />
    <label htmlFor="assistance-summary">明确目标与完成条件</label><textarea id="assistance-summary" value={fields.summary} disabled={blocked} onChange={(event) => { if (current()) setFields({ ...fields, summary: event.target.value }); }} />
    <label htmlFor="assistance-deadline">明确截止时间（RFC3339，含时区；留空表示未设定）</label><input id="assistance-deadline" placeholder="2026-10-23T16:00:00+08:00" value={fields.deadline_at ?? ""} disabled={blocked} onChange={(event) => { if (current()) setFields({ ...fields, deadline_at: event.target.value || null }); }} />
    {deadlineValid ? null : <p role="alert">请填写带时区的完整 RFC3339 时间，或留空；不会把“明天”自动转换成截止时间。</p>}
  </>;

  return <>
    <AssistanceWork />
    <AssistanceNotifications />
    <section className="panel">
      <h2>辅助事项收件箱</h2>
      <p>列表只读取编号、来源和状态。明确打开一项后才读取目标或统计详情；不会自动准备草稿或出网。</p>
      <button type="button" disabled={reading || writes.busy} onClick={() => read("list")}>核对事项列表</button>
      {target !== null && target !== "invalid" ? <button type="button" disabled={reading || writes.busy} onClick={() => read("item", target)}>重新读取当前事项</button> : null}
      <a href="#/assistance">返回收件箱</a>
      {writes.busy ? <p role="status">正在等待事项操作结束；切页不会取消已经开始的操作。</p> : null}
      {writes.unknown ? <p role="status">有写入结果未知，请先明确核对当前列表；不会重放旧请求。</p> : null}
      {error === null ? null : <p role="alert">{error}</p>}
      {notice === null ? null : <p role="status">{notice}</p>}
      {rows?.length === 0 ? <p>还没有辅助事项。</p> : null}
      <ul>{rows?.map((row) => <li key={row.item_id}>
        <a href={assistanceHash(row.item_id)} onClick={(event) => { if (address.hash === assistanceHash(row.item_id)) { event.preventDefault(); read("item", row.item_id); } }}>打开事项 {row.item_id}</a>；{row.source.kind === "manual_goal" ? "明确目标" : "本地观察"}；{VALIDITY[row.validity]}；{HANDLING[row.handling]}；截止 {row.deadline_at ?? "未设定"}
      </li>)}</ul>
    </section>

    {detail === null ? <section className="panel"><h2>新建明确目标</h2>{goalFields}
      <label htmlFor="assistance-create-source">本次明确关联的观察事项（可选）</label><select id="assistance-create-source" value={createSource} disabled={blocked} onChange={(event) => { if (current()) setCreateSource(event.target.value); }}>
        <option value="">不关联观察来源</option>{observations.map((row) => <option key={row.item_id} value={row.item_id}>{row.item_id}</option>)}
      </select>
      <button type="button" disabled={blocked || !deadlineValid || fields.title.trim() === "" || fields.summary.trim() === ""} onClick={() => {
        const sourceObservation = createSource === "" ? null : selectedObservation(createSource);
        if (createSource !== "" && sourceObservation === null) return;
        write(() => createAssistanceGoal({ ...fields, source_observation: sourceObservation }));
      }}>确认新建我的目标</button>
    </section> : <section className="panel">
      <h2>当前事项详情</h2><p>事项 {detail.item.item_id}；{HANDLING[detail.item.handling]}；{VALIDITY[detail.item.validity]}。</p><p>{detail.notice}</p>
      <AssistanceItemWork key={`${detail.item.item_id}/${detail.item.revision}`} item={detail.item} getCurrentItem={() => {
        return current() && ownsPayload(detailOwner.current) ? detailRef.current?.item ?? null : null;
      }} />
      <AssistanceReminder key={`reminder/${detail.item.item_id}/${detail.item.revision}`} item={detail.item} getCurrentItem={() => current() && ownsPayload(detailOwner.current) ? detailRef.current?.item ?? null : null} />
      <AssistanceFeedback key={`feedback/${detail.item.item_id}/${detail.item.revision}`} item={detail.item} getCurrentItem={() => current() && ownsPayload(detailOwner.current) ? detailRef.current?.item ?? null : null} />
      {detail.item.source.kind !== "manual_goal" ? null : <AssistanceLocalPreparationPanel key={`local/${detail.item.item_id}/${detail.item.revision}`} item={detail.item} getCurrentItem={() => current() && ownsPayload(detailOwner.current) ? detailRef.current?.item ?? null : null} />}
      {detail.goal === null ? null : <><h3>{detail.goal.title}</h3><p className="pre-wrap">{detail.goal.summary}</p></>}
      {detail.observation === null ? null : <><p>统计来源：{detail.observation.bundle_id}；联系人 {detail.observation.contact_id}。</p><p>{detail.observation.summary.text}</p><p>这里只展示本地统计，不读取消息正文。</p></>}
      {editable ? <>{goalFields}<button type="button" disabled={blocked || !deadlineValid || !fields.title.trim() || !fields.summary.trim()} onClick={() => write(() => editAssistanceGoal({ ...pointer(detail.item), ...fields }), detail)}>保存当前版本的目标修改</button>
        <label htmlFor="assistance-bind-source">替换或清除明确关联的观察来源</label><select id="assistance-bind-source" value={bindSource} disabled={blocked} onChange={(event) => { if (current()) setBindSource(event.target.value); }}>
          <option value="">清除观察关联</option>{observations.map((row) => <option key={row.item_id} value={row.item_id}>{row.item_id}</option>)}
        </select><button type="button" disabled={blocked} onClick={() => {
          const observation = bindSource === "" ? null : selectedObservation(bindSource); if (bindSource !== "" && observation === null) return;
          write(() => bindAssistanceGoalSource({ ...pointer(detail.item), source_observation: observation }), detail);
        }}>确认更新目标的来源关联</button>
      </> : null}
      {detail.item.handling !== "open" || detail.item.validity === "forgotten" ? <p>这项已经结束或不可用，旧页面不能重新打开目标。</p> : <>
        <button type="button" disabled={blocked} onClick={() => write(() => actAssistanceItem({ ...pointer(detail.item), action: "complete_self_reported" }), detail)}>我已完成（自报）</button>
        <button type="button" disabled={blocked} onClick={() => write(() => actAssistanceItem({ ...pointer(detail.item), action: "cancel" }), detail)}>取消这项目标</button>
        <button type="button" disabled={blocked || detail.item.validity !== "current"} onClick={() => {
          if (!current() || detailRef.current !== detail) return;
          draftAssistanceSelection.select(pointer(detail.item)); window.location.hash = "#/draft";
        }}>用于本次起草</button>
      </>}
      <button type="button" disabled={reading || writes.busy} onClick={() => read("item", detail.item.item_id)}>重新读取这项的当前版本</button>
    </section>}

    <section className="panel"><h2>明确选择本地观察来源</h2><p>先明确读取当前统计候选，再选择准确的候选与联系人。外部文本不会自动成为你的目标。</p>
      <button type="button" disabled={blocked} onClick={() => read("candidates")}>读取可选的本地统计候选</button>
      <label htmlFor="assistance-candidate">观察来源候选与联系人</label><select id="assistance-candidate" value={candidate} disabled={blocked} onChange={(event) => { if (current()) setCandidate(event.target.value); }}>
        <option value="">明确选择一份有效统计</option>{candidates?.filter((row) => row.validity === "active" && !row.ignored).flatMap((row) => row.summaries.map((summary) => <option key={`${row.bundle_id}/${summary.contact_id}`} value={`${row.bundle_id}/${summary.contact_id}`}>{row.bundle_id} / {summary.contact_id}</option>))}
      </select>
      <button type="button" disabled={blocked || candidate === ""} onClick={() => {
        const [bundle_id, contact_id] = candidate.split("/");
        const selected = candidates?.find((row) => row.bundle_id === bundle_id && row.validity === "active" && !row.ignored && row.summaries.some((summary) => summary.contact_id === contact_id));
        if (bundle_id === undefined || contact_id === undefined || selected === undefined) return;
        write(() => linkAssistanceObservation({ bundle_id, contact_id }, selected.namespace_id));
      }}>把这个来源关联为观察事项</button>
      {detail?.item.source.kind === "observation" ? <button type="button" disabled={blocked || candidate === ""} onClick={() => {
        const [bundle_id, contact_id] = candidate.split("/");
        if (bundle_id === undefined || detail.item.source.kind !== "observation" || contact_id !== detail.item.source.contact_id) return;
        if (!candidates?.some((row) => row.bundle_id === bundle_id && row.validity === "active" && !row.ignored && row.summaries.some((summary) => summary.contact_id === contact_id))) return;
        write(() => refreshAssistanceObservation({ ...pointer(detail.item), bundle_id }), detail);
      }}>用选定候选刷新同一观察事项</button> : null}
    </section>
  </>;
}
