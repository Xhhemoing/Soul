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

import { useEffect, useRef, useState } from "react";

import {
  createMemory,
  forgetMemory,
  memoryDetail,
  memoryList,
  previewForget,
  retryForgetCleanup,
  updateMemory,
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
  const [list, setList] = useState<MemoryList | null>(null);
  const [open, setOpen] = useState<MemoryDetail | null>(null);
  const [preview, setPreview] = useState<ForgetPreview | null>(null);
  const [receipt, setReceipt] = useState<ForgetReceipt | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);
  const [uncertainForget, setUncertainForget] = useState(false);
  const [cleanup, setCleanup] = useState<ForgetCleanup | null>(null);
  const requests = useRef(new MemoryRequests()).current;

  const reload = (): void => {
    const ticket = requests.nextList();
    void Promise.resolve().then(() => {
      if (!requests.ownsList(ticket)) throw new Error("inactive list request");
      return memoryList();
    }).then(
      (value) => { if (requests.ownsList(ticket)) setList(value); },
      (error: unknown) => {
        if (requests.ownsList(ticket)) setRefusal(asRefusal(error));
      },
    );
  };

  useEffect(() => {
    requests.activate();
    const ticket = requests.nextList();
    void Promise.resolve().then(() => {
      if (!requests.ownsList(ticket)) throw new Error("inactive list request");
      return memoryList();
    }).then(
      (value) => { if (requests.ownsList(ticket)) setList(value); },
      (error: unknown) => {
        if (requests.ownsList(ticket)) setRefusal(asRefusal(error));
      },
    );
    return () => requests.dispose();
  }, [requests]);

  function invokeOwned<T>(ticket: number, work: () => Promise<T>): Promise<T> {
    return Promise.resolve().then(() => {
      if (!requests.owns(ticket)) throw new Error("inactive memory request");
      return work();
    });
  }

  const finish = (ticket: number): void => {
    if (requests.finish(ticket)) setBusy(false);
  };

  /** Admit before IPC, not after constructing an already-running promise. */
  const act = (change: () => Promise<MemoryDetail>): void => {
    const ticket = requests.begin();
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setPreview(null);
    setReceipt(null);
    void invokeOwned(ticket, change).then(
      (value) => {
        if (!requests.owns(ticket)) return;
        setOpen(value);
        finish(ticket);
        reload();
      },
      (error: unknown) => {
        if (!requests.owns(ticket)) return;
        setRefusal(asRefusal(error));
        finish(ticket);
      },
    );
  };

  const askPrice = (memoryId: string): void => {
    const ticket = requests.begin();
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setPreview(null);
    setReceipt(null);
    void invokeOwned(ticket, () => previewForget(memoryId)).then(
      (value) => {
        if (!requests.owns(ticket)) return;
        requests.remember(ticket, value);
        setPreview(value);
        finish(ticket);
      },
      (error: unknown) => {
        if (!requests.owns(ticket)) return;
        setRefusal(asRefusal(error));
        finish(ticket);
      },
    );
  };

  const forget = (quoted: ForgetPreview): void => {
    const ticket = requests.begin(quoted);
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setCleanup(null);
    void invokeOwned(ticket, () => forgetMemory({
      preview_id: quoted.preview_id, memory_id: quoted.memory_id,
    })).then(
      (value) => {
        if (!requests.owns(ticket)) return;
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
        if (!requests.owns(ticket)) return;
        const problem = asRefusal(error);
        setRefusal(problem);
        // A lost IPC reply is not proof of rollback. Only an explicit
        // preview mismatch is known to leave the held preview unspent.
        if (problem.reason_code !== "PLAN_HASH_MISMATCH") {
          requests.remember(ticket, null);
          setPreview(null);
          setOpen(null);
          setUncertainForget(true);
        }
        finish(ticket);
        reload();
      },
    );
  };

  const retryCleanup = (): void => {
    const ticket = requests.begin();
    if (ticket === null) return;
    setBusy(true);
    setRefusal(null);
    setPreview(null);
    setCleanup(null);
    void invokeOwned(ticket, retryForgetCleanup).then(
      (value) => {
        if (!requests.owns(ticket)) return;
        setCleanup(value.cleanup);
        // Preserve the original counts, preview match, and audit uncertainty.
        // Cleanup cannot manufacture a missing destruction acknowledgement.
        setReceipt((previous) => previous === null
          ? null : { ...previous, cleanup: value.cleanup });
        finish(ticket);
      },
      (error: unknown) => {
        if (!requests.owns(ticket)) return;
        setRefusal(asRefusal(error));
        finish(ticket);
      },
    );
  };

  if (list === null) {
    return refusal === null ? (
      <section className="panel" aria-busy="true"><p>正在读本机的记忆…</p></section>
    ) : <Refused title="记忆没有读出来" refusal={refusal} testId="memory-refusal-code" />;
  }

  const recoverable = receipt !== null || uncertainForget
    || list.memories.some((row) => row.forget_state === "forgotten");

  return (
    <>
      <Write types={list.memory_types} busy={busy} onWrite={(fresh) => act(() => createMemory(fresh))} />
      {refusal === null ? null : (
        <Refused title="这次请求需要核对" refusal={refusal} testId="memory-refusal-code" />
      )}
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
                onOpen={() => act(() => memoryDetail(row.memory_id))}
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
        <Edit key={open.memory_id} detail={open} types={list.memory_types} busy={busy}
          onSave={(change) => act(() => updateMemory(open.memory_id, change))} />
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
  readonly onWrite: (fresh: { memory_type: string; title: string; summary: string }) => void;
}

function Write({ types, busy, onWrite }: WriteProps): React.JSX.Element {
  const [kind, setKind] = useState(types[0] ?? "episodic");
  const [title, setTitle] = useState("");
  const [summary, setSummary] = useState("");

  const submit = (): void => {
    onWrite({ memory_type: kind, title, summary });
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
        onChange={(event) => setKind(event.target.value)}
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
        onChange={(event) => setTitle(event.target.value)}
      />
      <label className="field" htmlFor="memory-summary">
        再多写几句
      </label>
      <textarea
        id="memory-summary"
        className="paste-box"
        rows={4}
        value={summary}
        onChange={(event) => setSummary(event.target.value)}
      />
      <div className="switch-row">
        <button
          type="button"
          className="primary"
          disabled={busy || title.trim() === "" || summary.trim() === ""}
          onClick={submit}
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
