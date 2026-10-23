/**
 * 自传记忆. Write one, edit one, and — the part worth being careful about —
 * forget one.
 *
 * Forgetting is allowed here, and it is the only destructive thing v0.1 does.
 * `docs/DECISIONS.md` D15 defines deleting as destroying the content key: the
 * live prose stops being openable and the row remains a tombstone. Soul writes
 * its own encrypted database and log; cleanup and audit are separate results. That is why this screen has a forget
 * and the file-plan screen still has no execute — they are not the same kind
 * of act, and AC-27 is about the other one.
 *
 * Because it is irreversible and the numbers behind it are a live query, the
 * two halves are separate commands and the second one refuses to run on
 * anything but the first one's answer. `previewForget` destroys nothing and
 * says so in a field a component cannot branch past; the confirmation echoes
 * the `preview_id` the core issued, and `soulcore` refuses one that names a
 * preview nobody read. So a second click cannot destroy something other than
 * what was on the screen in front of the user.
 */

import { useEffect, useRef, useState, useSyncExternalStore } from "react";

import {
  createMemory,
  forgetMemory,
  memoryDetail,
  memoryList,
  previewForget,
  retryForgetCleanup,
  updateMemory,
  getDraftSourceRevision,
  subscribeDraftSource,
  DraftSourceChangedError,
  subscribeExternalSource,
  hasPendingSourceChanges,
  type ForgetPreview,
  type ForgetReceipt,
  type MemoryDetail,
  type MemoryList,
  type MemoryRow,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";
import { cleanupStatus, type ForgetCleanup } from "../forgetOutcome";
import { ForgetResult } from "../components/ForgetResult";
import { MemoryRequests } from "./memoryRequests";
import { draftMemoryCandidates } from "./draftMemorySelection";
import { getRouteAddress, useRouteAddress, type RouteAddress } from "../routeAddress";
import { memoryTarget } from "./memoryTarget";

/** `memory.schema.json`'s kinds. The core sends which ones exist. */
const KIND: Record<string, string> = {
  episodic: "一件事",
  semantic: "一条事实",
  procedural: "一个做法",
  preference: "一个偏好",
  commitment: "一个承诺",
};

/** Where a memory is on the way to being forgotten. */
const FORGET_STATE: Record<string, string> = {
  active: "在",
  pending_forget: "已排进遗忘",
  forgotten: "已遗忘，只剩墓碑",
};

function words(source: Record<string, string>, key: string): string {
  return source[key] ?? key;
}

export function Memory(): React.JSX.Element {
  const address = useRouteAddress();
  const target = memoryTarget(address.hash);
  const source = useSyncExternalStore(subscribeDraftSource, getDraftSourceRevision);
  const [viewAddress, setViewAddress] = useState(address);
  const [navigationRetry, setNavigationRetry] = useState(0);
  const [list, setList] = useState<MemoryList | null>(null);
  const [storedOpen, storeOpen] = useState<{ readonly value: MemoryDetail; readonly address: RouteAddress; readonly source: symbol } | null>(null);
  const open = storedOpen !== null && storedOpen.address === address && address === getRouteAddress()
    && storedOpen.source === source ? storedOpen.value : null;
  const ownsOpen = (): boolean => storedOpen !== null && storedOpen.address === getRouteAddress()
    && storedOpen.source === getDraftSourceRevision();
  const setOpen = (value: MemoryDetail | null): void => {
    storeOpen(value === null ? null : { value, address, source: getDraftSourceRevision() });
  };
  const [preview, setPreview] = useState<ForgetPreview | null>(null);
  const [receipt, setReceipt] = useState<ForgetReceipt | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [writeNotice, setWriteNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [uncertainForget, setUncertainForget] = useState(false);
  const [cleanup, setCleanup] = useState<ForgetCleanup | null>(null);
  const [requests] = useState(() => new MemoryRequests());
  const externalRefresh = useRef(false);
  const owns = (ticket: number): boolean => requests.owns(ticket) && address === getRouteAddress();

  const reload = (): void => {
    const source = getDraftSourceRevision();
    const ticket = requests.nextList();
    void Promise.resolve().then(() => {
      if (!requests.ownsList(ticket) || address !== getRouteAddress()) throw new Error("inactive list request");
      return memoryList();
    }).then(
      (value) => { if (requests.ownsList(ticket) && address === getRouteAddress() && source === getDraftSourceRevision()) setList(value); },
      (error: unknown) => {
        if (requests.ownsList(ticket) && address === getRouteAddress() && source === getDraftSourceRevision()) setRefusal(asRefusal(error));
      },
    );
  };

  useEffect(() => {
    requests.activate();
    return () => requests.dispose();
  }, [requests]);

  useEffect(() => {
    let source = getDraftSourceRevision();
    const unsubscribe = subscribeDraftSource((next) => {
      if (source === next) return;
      source = next;
      requests.invalidatePreview(); setPreview(null);
    });
    const unsubscribeExternal = subscribeExternalSource(() => {
      externalRefresh.current = true;
      storeOpen(null); setList(null);
      if (!hasPendingSourceChanges()) setNavigationRetry((value) => value + 1);
    });
    return () => { unsubscribe(); unsubscribeExternal(); };
  }, [requests]);

  useEffect(() => {
    if (target === "invalid") return;
    if (hasPendingSourceChanges() && externalRefresh.current) return;
    // A same-page navigation never releases an in-flight write. Its physical
    // settlement wakes this effect to read the latest address, not the old one.
    const ticket = requests.begin();
    if (ticket === null) return;
    const refreshOnly = externalRefresh.current;
    externalRefresh.current = false;
    const initialSource = getDraftSourceRevision();
    const current = (): boolean => requests.owns(ticket) && address === getRouteAddress();
    void Promise.resolve().then(async () => {
      if (!current()) return;
      setViewAddress(address); setList(null); storeOpen(null); setPreview(null);
      if (!refreshOnly) { setReceipt(null); setWriteNotice(null); setCleanup(null); setUncertainForget(false); }
      setRefusal(null); setBusy(true);
      if (initialSource !== getDraftSourceRevision()) throw new DraftSourceChangedError();
      const value = await memoryList();
      if (!current()) return;
      if (initialSource !== getDraftSourceRevision()) throw new DraftSourceChangedError();
      setList(value);
      if (target !== null) {
        const detail = await memoryDetail(target);
        if (!current()) return;
        if (initialSource !== getDraftSourceRevision()) throw new DraftSourceChangedError();
        if (detail.memory_id.toLowerCase() !== target) {
          throw { reason_code: "MEMORY_TARGET_MISMATCH", explanation: "返回的记忆与所选编号不符，请重新选择。" };
        }
        storeOpen({ value: detail, address, source: initialSource });
      }
    }).catch((error: unknown) => { if (current()) setRefusal(asRefusal(error)); }).finally(() => {
      if (requests.finish(ticket)) {
        setBusy(false);
        if (address !== getRouteAddress() || (externalRefresh.current && !hasPendingSourceChanges())) setNavigationRetry((value) => value + 1);
      }
    });
  }, [address, navigationRetry, requests, target]);

  function invokeOwned<T>(ticket: number, work: () => Promise<T>, expectedSource?: symbol): Promise<T> {
    return Promise.resolve().then(() => {
      if (!owns(ticket)) throw new Error("inactive memory request");
      if (expectedSource !== undefined && expectedSource !== getDraftSourceRevision()) throw new DraftSourceChangedError();
      return work();
    });
  }

  const finish = (ticket: number): void => {
    if (requests.finish(ticket)) {
      setBusy(false);
      if (address !== getRouteAddress() || (externalRefresh.current && !hasPendingSourceChanges())) setNavigationRetry((value) => value + 1);
    }
  };

  /** Admit before IPC, not after constructing an already-running promise. */
  const act = (change: () => Promise<MemoryDetail>, expectedSource?: symbol, readOnly = false): Promise<boolean> => {
    if (address !== getRouteAddress()) return Promise.resolve(false);
    if (expectedSource !== undefined && expectedSource !== getDraftSourceRevision()) return Promise.resolve(false);
    const ticket = requests.begin();
    if (ticket === null) return Promise.resolve(false);
    setBusy(true);
    setRefusal(null);
    setWriteNotice(null);
    setPreview(null);
    setReceipt(null);
    let observedSource = getDraftSourceRevision();
    let sourceChanges = 0;
    const unsubscribe = subscribeDraftSource((next) => {
      if (next !== observedSource) { observedSource = next; sourceChanges += 1; }
    });
    return invokeOwned(ticket, change, expectedSource).then(
      (value) => {
        if (!owns(ticket)) { finish(ticket); return false; }
        if (readOnly && expectedSource !== getDraftSourceRevision()) {
          setRefusal(asRefusal(new DraftSourceChangedError()));
          finish(ticket);
          return false;
        }
        // A successful mutation has its own start/finish notifications. Any
        // extra source operation means this is a receipt, not current prose.
        if (!readOnly && sourceChanges > 2) {
          setOpen(null);
          setWriteNotice("这次写入已收到核心回执；期间资料又有变更。请重新打开记忆，核对当前内容。");
        } else setOpen(value);
        finish(ticket);
        reload();
        return true;
      },
      (error: unknown) => {
        if (!owns(ticket)) { finish(ticket); return false; }
        setRefusal(asRefusal(error));
        finish(ticket);
        return false;
      },
    ).finally(unsubscribe);
  };

  const askPrice = (memoryId: string): void => {
    if (address !== getRouteAddress()) return;
    const ticket = requests.begin();
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setPreview(null);
    setReceipt(null);
    void invokeOwned(ticket, () => previewForget(memoryId)).then(
      (value) => {
        if (!owns(ticket)) { finish(ticket); return; }
        requests.remember(ticket, value);
        setPreview(value);
        finish(ticket);
      },
      (error: unknown) => {
        if (!owns(ticket)) { finish(ticket); return; }
        setRefusal(asRefusal(error));
        finish(ticket);
      },
    );
  };

  const forget = (quoted: ForgetPreview): void => {
    if (address !== getRouteAddress()) return;
    const ticket = requests.begin(quoted);
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setCleanup(null);
    let observedSource = getDraftSourceRevision();
    let sourceChanges = 0;
    const unsubscribe = subscribeDraftSource((next) => {
      if (next !== observedSource) { observedSource = next; sourceChanges += 1; }
    });
    void invokeOwned(ticket, () => forgetMemory({
      preview_id: quoted.preview_id, memory_id: quoted.memory_id,
    })).then(
      (value) => {
        if (!owns(ticket)) { finish(ticket); return; }
        requests.remember(ticket, null);
        setPreview(null);
        const matches = value.memory_id === quoted.memory_id;
        setReceipt(matches ? value : null);
        setOpen(null);
        setUncertainForget(!matches || value.logical_committed !== true);
        finish(ticket);
        reload();
      },
      (error: unknown) => {
        if (!owns(ticket)) { finish(ticket); return; }
        const problem = asRefusal(error);
        setRefusal(problem);
        // A lost IPC reply is not proof of rollback. Only an explicit
        // preview mismatch is known to leave the held preview unspent.
        if (problem.reason_code === "PLAN_HASH_MISMATCH" && sourceChanges === 2) {
          // Only this attempt's local start/finally occurred. The core says
          // the quoted preparation is unspent; restore that exact quote.
          requests.remember(ticket, quoted);
          setPreview(quoted);
        } else if (problem.reason_code !== "PLAN_HASH_MISMATCH") {
          requests.remember(ticket, null);
          setPreview(null);
          setOpen(null);
          setUncertainForget(true);
        }
        finish(ticket);
        reload();
      },
    ).finally(unsubscribe);
  };

  const retryCleanup = (): void => {
    if (address !== getRouteAddress()) return;
    const ticket = requests.begin();
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setPreview(null);
    setCleanup(null);
    void invokeOwned(ticket, retryForgetCleanup).then(
      (value) => {
        if (!owns(ticket)) { finish(ticket); return; }
        setCleanup(value.cleanup);
        // Preserve the original counts, preview match, and audit uncertainty.
        // Cleanup cannot manufacture a missing destruction acknowledgement.
        setReceipt((previous) => previous === null
          ? null : { ...previous, cleanup: value.cleanup });
        finish(ticket);
      },
      (error: unknown) => {
        if (!owns(ticket)) { finish(ticket); return; }
        setRefusal(asRefusal(error));
        finish(ticket);
      },
    );
  };

  if (target === "invalid") return <section className="panel" role="alert"><p>记忆地址无效，请从记忆列表重新选择。</p><a href="#/memory">返回记忆列表</a></section>;

  const rereadTarget = target === null ? null : <button type="button" disabled={busy} onClick={() => {
    if (address === getRouteAddress()) setNavigationRetry((value) => value + 1);
  }}>重新读取此地址</button>;

  if (viewAddress !== address || address !== getRouteAddress() || list === null) {
    return refusal === null ? (
      <section className="panel" aria-busy="true"><p>正在读本机的记忆…</p></section>
    ) : <><Refused title="记忆没有读出来" refusal={refusal} testId="memory-refusal-code" />{rereadTarget}</>;
  }

  const recoverable = receipt !== null || uncertainForget
    || list.memories.some((row) => row.forget_state === "forgotten");

  return (
    <>
      <Write types={list.memory_types} busy={busy} onWrite={(fresh) => act(() => createMemory(fresh))} />
      {refusal === null ? null : (
        <Refused title="这次请求需要核对" refusal={refusal} testId="memory-refusal-code" />
      )}
      {writeNotice === null ? null : <p role="status" data-testid="memory-write-source-changed">{writeNotice}</p>}
      {open === null && !busy ? rereadTarget : null}
      {uncertainForget ? (
        <section className="panel" role="status" data-testid="forget-uncertain">
          <h2>尚未确认上一次遗忘的结果</h2>
          <p>没有收到完整回执，不等于没有执行。不要重复确认销毁；先核对列表和审计。
            日志清理即使成功，也不能替代这次遗忘或审计的确认。</p>
          <button type="button" disabled={busy} onClick={reload}>刷新列表核对</button>
        </section>
      ) : null}

      <section className="panel" aria-labelledby="memories-heading">
        <h2 id="memories-heading">这台机器上的记忆（{list.memories.length}）</h2>
        {list.memories.length === 0 ? (
          <p className="muted" data-testid="no-memories">
            还没有写过记忆。上面写一条，它会加密存在本机，正文只有你打开的时候才解开。
          </p>
        ) : (
          <ul className="facts" data-testid="memory-list">
            {list.memories.map((row) => (
              <Row key={row.memory_id} row={row} busy={busy}
                onOpen={() => {
                  setOpen(null);
                  const initialSource = getDraftSourceRevision();
                  return act(async () => {
                    const detail = await memoryDetail(row.memory_id);
                    if (initialSource !== getDraftSourceRevision()) throw new DraftSourceChangedError();
                    if (detail.memory_id.toLowerCase() !== row.memory_id.toLowerCase()) throw {
                      reason_code: "MEMORY_TARGET_MISMATCH", explanation: "返回的记忆与所选编号不符，请重新选择。",
                    };
                    return detail;
                  }, initialSource, true);
                }}
                onAskPrice={() => askPrice(row.memory_id)} />
            ))}
          </ul>
        )}
        <p className="muted" data-testid="forget-stage-notice">
          下面“正文从此打不开”的说明以日志清理也获得确认为前提。
          墓碑只表明逻辑状态，不代表旧日志已经清理，也不代表审计写入已确认。
        </p>
        <p className="muted" data-testid="forget-notice">{list.forget_notice}</p>
      </section>

      {open === null ? null : (
        <section className="panel"><button type="button" disabled={busy || list.memories.find((row) => row.memory_id === open.memory_id)?.forget_state !== "active"} onClick={() => { if (ownsOpen()) draftMemoryCandidates.add(open.memory_id); }}>用于本次起草</button>{" "}<a href="#/draft">前往起草并预览本次用途和出处</a></section>
      )}
      {open === null ? null : (
        <Edit key={open.memory_id} detail={open} types={list.memory_types} busy={busy}
          onSave={(change) => ownsOpen() ? act(() => updateMemory(open.memory_id, change), storedOpen?.source) : Promise.resolve(false)} />
      )}
      {preview === null ? null : (
        <Price preview={preview} busy={busy} onForget={() => forget(preview)}
          onKeep={() => { if (requests.abandon()) setPreview(null); }} />
      )}
      {receipt === null ? null : <ForgetResult receipt={receipt} />}
      {recoverable ? (
        <section className="panel" aria-labelledby="cleanup-heading">
          <h2 id="cleanup-heading">日志清理与恢复</h2>
          <p>这个操作只重试 Soul 自己的数据库日志清理，不会再次销毁密钥，
            不会补写审计，也不承诺磁盘块的物理擦除。重启后仍可使用。</p>
          <button type="button" disabled={busy} onClick={retryCleanup}>仅重试日志清理</button>
          {cleanup === null ? null : (
            <p role="status" data-testid="cleanup-result">
              {cleanupStatus(cleanup) === "complete" ? "本次日志清理已确认。"
                : cleanupStatus(cleanup) === "pending" ? "日志清理仍待完成，旧日志可能仍有包裹密钥。"
                  : "本次清理结果尚未确认。"}
              这不改变上一次遗忘回执或审计的确认状态。
            </p>
          )}
        </section>
      ) : null}
    </>
  );
}

interface WriteProps {
  readonly types: readonly string[];
  readonly busy: boolean;
  readonly onWrite: (fresh: { memory_type: string; title: string; summary: string }) => Promise<boolean>;
}

function Write({ types, busy, onWrite }: WriteProps): React.JSX.Element {
  const [kind, setKind] = useState(types[0] ?? "episodic");
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");
  const revision = useRef(0);

  // A completed save belongs to the submitted draft, not later edits or a
  // form that has since unmounted. No copy of its prose is retained here.
  useEffect(() => () => { revision.current += 1; }, []);

  const submit = async (): Promise<void> => {
    const submittedRevision = revision.current;
    const saved = await onWrite({ memory_type: kind, title, summary });
    if (!saved || revision.current !== submittedRevision) return;
    revision.current += 1;
    setTitle("");
    setSummary("");
  };

  return (
    <section className="panel" aria-labelledby="write-heading">
      <h2 id="write-heading">记一条</h2>
      <label className="field" htmlFor="memory-kind">
        这是什么
      </label>
      <select
        id="memory-kind"
        className="text-input"
        value={kind}
        onChange={(event) => { revision.current += 1; setKind(event.target.value); }}
      >
        {types.map((type) => (
          <option key={type} value={type}>
            {words(KIND, type)}
          </option>
        ))}
      </select>
      <label className="field" htmlFor="memory-title">
        一句话说清是什么
      </label>
      <input
        id="memory-title"
        className="text-input"
        type="text"
        value={title}
        onChange={(event) => { revision.current += 1; setTitle(event.target.value); }}
      />
      <label className="field" htmlFor="memory-summary">
        再多写几句
      </label>
      <textarea
        id="memory-summary"
        className="paste-box"
        rows={4}
        value={summary}
        onChange={(event) => { revision.current += 1; setSummary(event.target.value); }}
      />
      <div className="switch-row">
        <button
          type="button"
          className="primary"
          disabled={busy || title.trim() === "" || summary.trim() === ""}
          onClick={() => void submit()}
        >
          存下来
        </button>
      </div>
    </section>
  );
}

interface RowProps {
  readonly row: MemoryRow;
  readonly busy: boolean;
  readonly onOpen: () => void;
  readonly onAskPrice: () => void;
}

/**
 * One memory in the list, with its prose still sealed.
 *
 * The two character counts are what `MemoryDigest` carries: enough to tell
 * which entry is the long one, and not enough to say anything about what it
 * says. Opening it is a separate command, so a list view cannot leak a body
 * by rendering too much of what it was handed.
 */
function Row({ row, busy, onOpen, onAskPrice }: RowProps): React.JSX.Element {
  const gone = row.forget_state === "forgotten";
  return (
    <li data-testid={`memory-${row.memory_id}`}>
      <span>{words(KIND, row.memory_type)}</span>
      <span className="muted">
        {" "}
        标题 {row.title_chars} 字，正文 {row.summary_chars} 字，
        {words(FORGET_STATE, row.forget_state)}
        {row.third_party_content_present ? "，里面有别人的内容" : ""}
      </span>
      <div className="switch-row">
        <button type="button" disabled={busy || gone} onClick={onOpen}>
          打开
        </button>
        <button type="button" disabled={busy || gone} onClick={onAskPrice}>
          看遗忘会影响什么
        </button>
      </div>
    </li>
  );
}

interface EditProps {
  readonly detail: MemoryDetail;
  readonly types: readonly string[];
  readonly busy: boolean;
  readonly onSave: (change: { memory_type: string; title: string; summary: string }) => void;
}

function Edit({ detail, types, busy, onSave }: EditProps): React.JSX.Element {
  const [kind, setKind] = useState(detail.memory_type);
  const [title, setTitle] = useState(detail.title);
  const [summary, setSummary] = useState(detail.summary);

  return (
    <section className="panel" aria-labelledby="edit-heading">
      <h2 id="edit-heading">打开的这一条</h2>
      <p className="muted">如果这条记忆用于联系人／场合语域偏好，普通编辑会停用该偏好；需回到起草重新明确确认。</p>
      <label className="field" htmlFor="edit-kind">
        这是什么
      </label>
      <select
        id="edit-kind"
        className="text-input"
        value={kind}
        onChange={(event) => setKind(event.target.value)}
      >
        {types.map((type) => (
          <option key={type} value={type}>
            {words(KIND, type)}
          </option>
        ))}
      </select>
      <label className="field" htmlFor="edit-title">
        一句话说清是什么
      </label>
      <input
        id="edit-title"
        className="text-input"
        type="text"
        value={title}
        onChange={(event) => setTitle(event.target.value)}
      />
      <label className="field" htmlFor="edit-summary">
        再多写几句
      </label>
      <textarea
        id="edit-summary"
        className="paste-box"
        rows={4}
        value={summary}
        onChange={(event) => setSummary(event.target.value)}
      />
      <div className="switch-row">
        <button
          type="button"
          className="primary"
          disabled={busy}
          onClick={() => onSave({ memory_type: kind, title, summary })}
        >
          改好了
        </button>
      </div>
      <p className="muted" data-testid="edit-key">
        改动会用同一把内容密钥重新密封，所以这条记忆始终是一个遗忘单位（
        <code>{detail.content_key_id}</code>）。
      </p>
    </section>
  );
}

interface PriceProps {
  readonly preview: ForgetPreview;
  readonly busy: boolean;
  readonly onForget: () => void;
  readonly onKeep: () => void;
}

/**
 * What forgetting would cost, and the one button that would pay it.
 *
 * `destroys_anything` is typed as the literal `false`, so this panel arriving
 * is not the act. What makes the button safe is not that it is a second click
 * but that it sends back the `preview_id` printed above it: the core refuses a
 * forget that names a preview it is not holding, so approving a stale set of
 * numbers destroys nothing.
 *
 * The panel stays up only for an explicit preview mismatch. The core kept the preview it issued —
 * it matches before it takes — so these are still the numbers it is holding,
 * and the button beneath them is still the one that pays for them.
 */
function Price({ preview, busy, onForget, onKeep }: PriceProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="price-heading">
      <h2 id="price-heading">遗忘这一条会影响什么</h2>
      <ul className="facts" data-testid="forget-preview">
        <li>要销毁 {preview.content_key_count} 把内容密钥。</li>
        <li>
          牵动 {preview.memories_affected} 条记忆、{preview.contacts_affected} 个人。
        </li>
        <li>
          {preview.sealed_blobs_destroyed} 块密封正文会打不开，{preview.inferences_orphaned}{" "}
          条推断会失去依据。
        </li>
        <li>{preview.audit_entries_retained} 条审计记录会留着：链子不记内容，也不该挡住遗忘。</li>
        <li>将撤销导入来源：{preview.import_namespaces_revoked ?? "未提供"}；将失效本地洞察：{preview.local_insights_invalidated ?? "未提供"}。来源撤销后禁止继续重导。</li>
        <li>将失效辅助事项：{preview.assistance_items_invalidated ?? "未提供"}。旧事项准备不能继续使用。</li>
        <li data-testid="forget-preview-id">
          这份预览的编号：<code>{preview.preview_id}</code>
        </li>
        <li data-testid="preview-destroys-nothing">
          {preview.destroys_anything
            ? "这份预览声称自己销毁了东西，请把这件事报告出来。"
            : "看这一页没有销毁任何东西。"}
        </li>
      </ul>
      <p className="muted">{preview.notice}</p>
      <div className="switch-row">
        <button type="button" className="primary" disabled={busy} onClick={onForget}>
          就按上面这些，遗忘它
        </button>
        <button type="button" disabled={busy} onClick={onKeep}>
          先留着
        </button>
      </div>
    </section>
  );
}
