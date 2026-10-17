import { useCallback, useEffect, useRef, useState } from "react";
import { evidenceSources, getDraftSourceRevision, readEvidenceEvent, type DraftSourceRevision,
  type EvidenceEventSource, type EvidenceEventView, type EvidenceSourcesRequest, type EvidenceSourcesView } from "../core";
import { asRefusal } from "../refusal";

/** A click-bound local display intent, never an egress or drafting grant. */
export interface EvidenceSelection {
  readonly request: EvidenceSourcesRequest;
  readonly generation: number;
  readonly hash: string;
  readonly source: DraftSourceRevision;
  readonly fromHint: string;
  readonly toHint: string;
  readonly opener: HTMLButtonElement;
}
const sameId = (left: string, right: string): boolean => left.toLowerCase() === right.toLowerCase();
function sameEvidence(left: EvidenceSourcesRequest, right: EvidenceSourcesRequest): boolean {
  return sameId(left.contact_id, right.contact_id) && sameId(left.relationship_id, right.relationship_id) && sameId(left.evidence_id, right.evidence_id);
}
const AVAILABILITY = { available: "可以明确查看这一条", no_body: "这条没有正文", unavailable: "这条来源不可用", too_large: "这条超出本机查看容量" };

export function EvidenceSourceViewer({ selection, getSelection, onClose }: {
  readonly selection: EvidenceSelection;
  readonly getSelection: () => EvidenceSelection | null;
  readonly onClose: (selection: EvidenceSelection) => void;
}): React.JSX.Element {
  const [sources, setSources] = useState<EvidenceSourcesView | null>(null);
  const [body, setBody] = useState<EvidenceEventView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const life = useRef<symbol | null>(null); const eventGeneration = useRef(0);
  const sourcesRef = useRef<EvidenceSourcesView | null>(null);
  const reading = useRef(new Set<string>());
  const heading = useRef<HTMLHeadingElement>(null); const bodyHeading = useRef<HTMLHeadingElement>(null);
  const current = useCallback((): boolean => life.current !== null && getSelection() === selection
    && selection.hash === window.location.hash && selection.source === getDraftSourceRevision(), [getSelection, selection]);

  useEffect(() => {
    const mounted = Symbol(); life.current = mounted;
    const activeReads = reading.current;
    void Promise.resolve().then(() => {
      if (!current()) return null;
      return evidenceSources(selection.request);
    }).then((value) => {
      if (value === null || life.current !== mounted || !current()) return;
      if (!sameEvidence(value, selection.request) || new Set(value.events.map((row) => row.event_id.toLowerCase())).size !== value.events.length) {
        setError("来源条目与本次选择不一致，未显示。请关闭后重新核对。"); return;
      }
      sourcesRef.current = value; setSources(value);
    }, (error: unknown) => { if (life.current === mounted && current()) setError(asRefusal(error).explanation); });
    const frame = window.requestAnimationFrame(() => { if (life.current === mounted && current()) heading.current?.focus(); });
    return () => {
      if (life.current === mounted) life.current = null;
      eventGeneration.current += 1; sourcesRef.current = null; activeReads.clear();
      window.cancelAnimationFrame(frame);
    };
  }, [selection, current]);

  const show = (event: EvidenceEventSource): void => {
    const view = sources;
    if (!current() || view === null || sourcesRef.current !== view || event.availability !== "available" || event.namespace_id === null
      || reading.current.has(event.event_id) || !view.events.includes(event)) return;
    const mounted = life.current; const generation = ++eventGeneration.current;
    const owns = (): boolean => life.current === mounted && current() && sourcesRef.current === view && generation === eventGeneration.current;
    reading.current.add(event.event_id); setBody(null); setError(null); setPending(event.event_id);
    void Promise.resolve().then(() => {
      if (!owns()) return null;
      return readEvidenceEvent({ ...selection.request, event_id: event.event_id });
    }).then((value) => {
      if (value === null || !owns()) return;
      if (!sameEvidence(value, selection.request) || !sameId(value.event_id, event.event_id) || !sameId(value.namespace_id, event.namespace_id!)
        || value.source !== event.source || value.occurred_at !== event.occurred_at || value.direction !== event.direction || value.venue !== event.venue) {
        setError("返回的内容与这一次指定来源不一致，未显示。请关闭后重新核对。"); return;
      }
      setBody(value);
    }, (error: unknown) => { if (owns()) { setBody(null); setError(asRefusal(error).explanation); } }).finally(() => {
      reading.current.delete(event.event_id);
      if (owns()) setPending(null);
    });
  };

  useEffect(() => {
    if (body === null) return;
    const generation = eventGeneration.current;
    const frame = window.requestAnimationFrame(() => {
      if (current() && generation === eventGeneration.current) bodyHeading.current?.focus();
    });
    return () => window.cancelAnimationFrame(frame);
  }, [body, current]);

  return <section className="panel" aria-labelledby="evidence-sources-heading">
    <h2 id="evidence-sources-heading" tabIndex={-1} ref={heading}>本地单条来源查看</h2>
    <p>关系：{selection.fromHint} ↔ {selection.toHint}。联系人 {selection.request.contact_id}；关系 {selection.request.relationship_id}；证据 {selection.request.evidence_id}。</p>
    <p>这里核对当前来源，不认证旧统计或摘要仍然准确。先列元数据；只有明确点击某一条，才在本机显示那一条可能敏感的正文。</p>
    <button type="button" onClick={() => { if (current()) { eventGeneration.current += 1; sourcesRef.current = null; setBody(null); setError(null); onClose(selection); } }}>关闭并清除本次来源展示</button>
    {sources === null && error === null ? <p role="status">正在核对这一条证据的来源元数据…</p> : null}
    {error === null ? null : <p role="alert">{error}</p>}
    {sources === null ? null : <>
      <p>{sources.notice}</p>
      {sources.events.length === 0 ? <p>这条证据没有可查看的正文来源。不会改读其它证据。</p> : <ul aria-label="本条证据的来源事件">{sources.events.map((event) => <li key={event.event_id}>
        <p>事件 {event.event_id}；命名空间 {event.namespace_id ?? "不可用"}；来源 {event.source}；时间 {event.occurred_at}；方向 {event.direction}；场景 {event.venue}；正文字数 {event.body_chars ?? "未知"}。</p>
        <p>{event.notice} {AVAILABILITY[event.availability]}</p>
        <button type="button" disabled={event.availability !== "available" || event.namespace_id === null || pending === event.event_id}
          onClick={() => show(event)}>在本机显示这一条敏感内容：{event.event_id}</button>
      </li>)}</ul>}
    </>}
    {pending === null ? null : <p role="status">正在读取明确选择的事件 {pending}；没有预取其它正文。</p>}
    {body === null ? null : <section aria-labelledby="evidence-event-heading">
      <h3 id="evidence-event-heading" tabIndex={-1} ref={bodyHeading}>当前这一条来源内容</h3>
      <p>证据 {body.evidence_id}；事件 {body.event_id}；命名空间 {body.namespace_id}。</p>
      <p>{body.notice}</p><pre className="pre-wrap" data-testid="evidence-event-body">{body.body}</pre>
      <p>正文仅用于当前本地核对，没有加入端点请求、记忆或起草材料。关闭会从页面移除这次展示。</p>
    </section>}
  </section>;
}
