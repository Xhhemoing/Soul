import { useEffect, useRef, useState } from "react";
import {
  assistanceItemWork, assistanceWorkStatus, getDraftSourceRevision, retryAssistanceWork, subscribeDraftSource,
  type AssistanceItemRow, type AssistanceWorkMetadata, type AssistanceWorkStatus,
} from "../core";
import { asRefusal } from "../refusal";
import { getRouteAddress, useRouteAddress, type RouteAddress } from "../routeAddress";
import { assistanceWrites, useAssistanceWrites } from "./assistanceRequests";
import { isDraftRefusal } from "./draftRequests";

interface Owner { readonly address: RouteAddress; readonly source: symbol }
interface Owned<T> { readonly owner: Owner; readonly value: T }
interface WorkTicket { readonly id: symbol; readonly life: symbol }
const RUNTIME = { not_started: "尚未启动", running: "运行中", stopped: "已停止", failed: "运行失败" };
const WORK = { none: "尚无本地准备记录", pending: "等待本地准备", running: "正在本地准备", ready: "当前版本的本地准备已完成",
  source_unavailable: "来源暂不可用", failed: "本地准备失败", unknown: "本地准备结果尚未确认" };

export function useAssistanceRequestOwner() {
  const address = useRouteAddress(); const [source, setSource] = useState(getDraftSourceRevision);
  const mount = useRef<symbol | null>(null); const active = useRef<symbol | null>(null);
  useEffect(() => {
    const life = Symbol("local work view"); mount.current = life;
    const unsubscribe = subscribeDraftSource(setSource);
    return () => { if (mount.current === life) mount.current = null; unsubscribe(); };
  }, []);
  const current = (): boolean => mount.current !== null && address === getRouteAddress() && source === getDraftSourceRevision();
  const ownsPayload = (owner: Owner): boolean => owner.address === address && address === getRouteAddress() && owner.source === getDraftSourceRevision();
  const begin = (): WorkTicket | null => {
    if (!current() || mount.current === null || active.current !== null) return null;
    const ticket = { id: Symbol("local work request"), life: mount.current }; active.current = ticket.id; return ticket;
  };
  const owns = (ticket: WorkTicket): boolean => mount.current === ticket.life && active.current === ticket.id && current();
  const finish = (ticket: WorkTicket): boolean => {
    if (active.current !== ticket.id) return false;
    active.current = null; return mount.current === ticket.life;
  };
  return { address, source, current, ownsPayload, begin, owns, finish };
}

/** Explicit metadata reads only. The core/Tauri runtime owns scheduling. */
export function AssistanceWork(): React.JSX.Element {
  const owner = useAssistanceRequestOwner();
  const [stored, setStored] = useState<Owned<AssistanceWorkStatus> | null>(null);
  const [reading, setReading] = useState(false); const [error, setError] = useState<string | null>(null);
  const value = stored !== null && owner.ownsPayload(stored.owner) ? stored.value : null;
  const read = (): void => {
    const ticket = owner.begin(); if (ticket === null) return;
    setReading(true); setError(null);
    const current = (): boolean => owner.owns(ticket);
    void Promise.resolve().then(async () => {
      if (!current()) return;
      const result = await assistanceWorkStatus();
      if (current()) setStored({ owner: { address: owner.address, source: owner.source }, value: result });
    }).catch((problem: unknown) => { if (current()) setError(asRefusal(problem).explanation); }).finally(() => {
      if (owner.finish(ticket)) setReading(false);
    });
  };
  return <section className="panel"><h2>本地工作状态</h2>
    <p>这里只核对运行状态和计数，不读取目标正文，也不会自动准备或采用草稿。</p>
    <button type="button" disabled={reading} onClick={read}>核对本地工作状态</button>
    {error === null ? null : <p role="alert">{error}</p>}
    {value === null ? null : <div data-testid="assistance-work-status">
      <p>后台运行：{RUNTIME[value.runtime_state]}。最近错误代码：{value.last_runtime_error ?? "无"}。</p>
      <p>等待项：{value.pending_items ?? "暂不可读"}；已认领项：{value.claimed_items ?? "暂不可读"}。</p>
      <p>{value.active_worker ? "有本地工作正在进行。" : "当前没有本地工作在执行。"}{value.can_begin ? "可以接续本地工作。" : "暂不能开始新的本地工作。"}</p>
      {value.foreground_preparation_held ? <p>有待确认的前台准备，后台会等待；核对状态不会清除它。</p> : null}
      {value.coordination_unknown ? <p>工作协调结果未知，只能继续核对，不会重发旧工作。</p> : null}
      {value.collector_paused_for_import ? <p>采集已为导入暂停，是否恢复由核心核对原授权决定。</p> : null}
    </div>}
  </section>;
}

export function AssistanceItemWork({ item, getCurrentItem }: {
  readonly item: AssistanceItemRow; readonly getCurrentItem: () => AssistanceItemRow | null;
}): React.JSX.Element {
  const owner = useAssistanceRequestOwner(); const writes = useAssistanceWrites();
  const [stored, setStored] = useState<Owned<AssistanceWorkMetadata> | null>(null);
  const [reading, setReading] = useState(false); const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const currentItem = (): boolean => {
    const actual = getCurrentItem();
    return owner.current() && actual?.item_id === item.item_id && actual.revision === item.revision;
  };
  const value = stored !== null && owner.ownsPayload(stored.owner) && stored.value.item_id === item.item_id
    && stored.value.item_revision === item.revision ? stored.value : null;
  const ready = value?.state === "ready" && value.result_revision === item.revision
    && value.pending_revision === null && value.claimed_revision === null && value.claim_id === null;
  const retryable = value?.state === "failed" && value.pending_revision === null && value.claimed_revision === null && value.claim_id === null
    && item.handling === "open" && item.validity === "current";
  const run = (retry: boolean): void => {
    if (!currentItem() || (retry && (!retryable || writes.busy || writes.unknown))) return;
    const ticket = owner.begin(); if (ticket === null) return;
    const slot = retry ? assistanceWrites.begin() : null;
    if (retry && slot === null) { owner.finish(ticket); return; }
    setReading(true); setError(null); setNotice(null);
    let started = false; let unknown = false;
    const current = (): boolean => owner.owns(ticket) && currentItem();
    void Promise.resolve().then(async () => {
      if (!current()) return;
      started = true;
      const result = retry ? await retryAssistanceWork({ item_id: item.item_id, expected_revision: item.revision }) : await assistanceItemWork(item.item_id);
      if (!current()) return;
      if (result.item_revision !== item.revision) { setStored(null); setNotice("事项版本已变化，请重新读取当前事项后再核对准备状态。"); return; }
      setStored({ owner: { address: owner.address, source: owner.source }, value: result });
      if (retry) setNotice("已收到本次重试的状态。请明确核对后续进展；不会自动采用草稿。");
    }).catch((problem: unknown) => {
      unknown = retry && started && !isDraftRefusal(problem);
      if (current()) { setStored(null); setError(unknown ? "重试结果尚未确认，请核对工作状态和事项列表；不会自动重发。" : asRefusal(problem).explanation); }
    }).finally(() => {
      if (slot !== null) assistanceWrites.finish(slot, unknown);
      if (owner.finish(ticket)) setReading(false);
    });
  };
  return <section className="panel"><h3>这项的本地准备</h3>
    <button type="button" disabled={reading || writes.busy} onClick={() => run(false)}>核对这项的准备状态</button>
    {error === null ? null : <p role="alert">{error}</p>}{notice === null ? null : <p role="status">{notice}</p>}
    {value === null ? null : <div data-testid="assistance-item-work">
      <p>{value.state === "ready" && !ready ? "准备状态需重新核对，不能将旧结果视为当前准备。" : WORK[value.state]}</p>
      {value.failure_code === null ? null : <p>工作错误代码：{value.failure_code}。</p>}
      {value.state === "running" ? <p>正在等待实际工作结束；核对或切页不会取消它。</p> : null}
      {value.state === "unknown" ? <p>只能核对当前状态，不重发结果未知的工作。</p> : null}
      {value.state === "source_unavailable" ? <p>请先核对事项来源；不会自动换用其他材料。</p> : null}
    </div>}
    {retryable ? <button type="button" disabled={reading || writes.busy || writes.unknown} onClick={() => run(true)}>重试本地准备</button> : null}
  </section>;
}
