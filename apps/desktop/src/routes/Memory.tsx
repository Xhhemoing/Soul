/**
 * 自传记忆. Write one, edit one, and — the part worth being careful about —
 * forget one.
 *
 * Forgetting is allowed here, and it is the only destructive thing v0.1 does.
 * `docs/DECISIONS.md` D15 defines deleting as destroying the content key: the
 * prose stops being openable, the row stays behind as a tombstone, and no file
 * is written or removed anywhere on disk. That is why this screen has a forget
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

import { useEffect, useState } from "react";

import {
  createMemory,
  forgetMemory,
  memoryDetail,
  memoryList,
  previewForget,
  updateMemory,
  type ForgetPreview,
  type ForgetReceipt,
  type MemoryDetail,
  type MemoryList,
  type MemoryRow,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

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

  const reload = (): Promise<void> =>
    memoryList().then(
      (value) => setList(value),
      (error: unknown) => setRefusal(asRefusal(error)),
    );

  useEffect(() => {
    let live = true;
    memoryList().then(
      (value) => {
        if (live) setList(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  /** Anything that touches one memory: clear the forget in flight first. */
  const act = (change: Promise<MemoryDetail>): void => {
    setBusy(true);
    setRefusal(null);
    setPreview(null);
    setReceipt(null);
    change.then(
      (value) => {
        setOpen(value);
        setBusy(false);
        void reload();
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  /** Read the price. Nothing is destroyed by this call. */
  const askPrice = (memoryId: string): void => {
    setBusy(true);
    setRefusal(null);
    setReceipt(null);
    previewForget(memoryId).then(
      (value) => {
        setPreview(value);
        setBusy(false);
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  /**
   * Pay it, with the core's own answer echoed back unchanged.
   *
   * The preview stays on screen until the core says the forget ran. A refusal
   * leaves the held preview standing on the other side — `Session::forget_
   * memory` matches before it takes, so a confirmation that named the wrong
   * preview costs the click and not the price the user read — and a screen
   * that cleared the panel anyway would make one refused click the reason to
   * walk the irreversible screen again. Retries that get clicked through
   * rather than read are the thing this whole page is built to avoid.
   */
  const forget = (quoted: ForgetPreview): void => {
    setBusy(true);
    setRefusal(null);
    forgetMemory({ preview_id: quoted.preview_id, memory_id: quoted.memory_id }).then(
      (value) => {
        setPreview(null);
        setReceipt(value);
        setOpen(null);
        setBusy(false);
        void reload();
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  if (list === null) {
    return refusal === null ? (
      <section className="panel" aria-busy="true">
        <p>正在读本机的记忆…</p>
      </section>
    ) : (
      <Refused title="记忆没有读出来" refusal={refusal} testId="memory-refusal-code" />
    );
  }

  return (
    <>
      <Write types={list.memory_types} busy={busy} onWrite={(fresh) => act(createMemory(fresh))} />

      {refusal === null ? null : (
        <Refused title="这一次没有做成" refusal={refusal} testId="memory-refusal-code" />
      )}

      <section className="panel" aria-labelledby="memories-heading">
        <h2 id="memories-heading">这台机器上的记忆（{list.memories.length}）</h2>
        {list.memories.length === 0 ? (
          <p className="muted" data-testid="no-memories">
            还没有写过记忆。上面写一条，它会加密存在本机，正文只有你打开的时候才解开。
          </p>
        ) : (
          <ul className="facts" data-testid="memory-list">
            {list.memories.map((row) => (
              <Row
                key={row.memory_id}
                row={row}
                busy={busy}
                onOpen={() => act(memoryDetail(row.memory_id))}
                onAskPrice={() => askPrice(row.memory_id)}
              />
            ))}
          </ul>
        )}
        <p className="muted" data-testid="forget-notice">
          {list.forget_notice}
        </p>
      </section>

      {open === null ? null : (
        <Edit
          key={open.memory_id}
          detail={open}
          types={list.memory_types}
          busy={busy}
          onSave={(change) => act(updateMemory(open.memory_id, change))}
        />
      )}

      {preview === null ? null : (
        <Price
          preview={preview}
          busy={busy}
          onForget={() => forget(preview)}
          onKeep={() => setPreview(null)}
        />
      )}

      {receipt === null ? null : (
        <section className="panel" aria-labelledby="receipt-heading">
          <h2 id="receipt-heading">已经遗忘</h2>
          <ul className="facts" data-testid="forget-receipt">
            <li>
              销毁了 {receipt.content_keys_destroyed} 把内容密钥，
              {receipt.sealed_blobs_destroyed} 块密封正文从此打不开。
            </li>
            <li>{receipt.inferences_orphaned} 条推断失去了依据，会标出来而不是悄悄留着。</li>
            <li data-testid="receipt-matched">
              {receipt.matched_preview
                ? "销毁的东西和你看过的那份预览一致。"
                : "销毁的东西和预览对不上，请把这件事报告出来。"}
            </li>
          </ul>
        </section>
      )}
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
 * The panel stays up through a refusal. The core kept the preview it issued —
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
