/**
 * 起草. A box to paste into, a button that writes, and no way to send.
 *
 * The absence is the feature, so it is worth saying where it comes from. This
 * screen cannot send because there is no command to send with: `core.ts` names
 * every command the shell has, `src-tauri/tests/command_surface.rs` checks that
 * list against the Rust one, and `soulcore`'s drafting surface is searched for
 * the word by `crates/soulcore/tests/draft_commands.rs`. Hiding a button would
 * be a promise; having nothing to bind one to is a fact.
 *
 * Everything the user reads about sending comes from the core rather than from
 * this file. What is decided here is layout.
 */

import { useEffect, useState } from "react";

import {
  draftNotices,
  draftReply,
  type Draft as DraftValue,
  type DraftNotices,
  type DraftRefusalView,
} from "../core";

/** What the core said, in the shape the screen renders it. */
type Outcome =
  | { readonly kind: "none" }
  | { readonly kind: "working" }
  | { readonly kind: "draft"; readonly draft: DraftValue }
  | { readonly kind: "refused"; readonly refusal: DraftRefusalView };

/** A refusal that did not arrive as one — the core is not answering at all. */
function asRefusal(error: unknown): DraftRefusalView {
  const shaped = error as Partial<DraftRefusalView> | null;
  return typeof shaped?.reason_code === "string" && typeof shaped.explanation === "string"
    ? { reason_code: shaped.reason_code, explanation: shaped.explanation }
    : { reason_code: "unavailable", explanation: String(error) };
}

const DEGRADED: Record<string, string> = {
  reply_unreadable: "端点的回复读不出来，这一条是本机模板写的。",
  reply_empty: "端点没有给出内容，这一条是本机模板写的。",
  reply_clinical: "端点的回复里有这个产品不说的词，已经丢掉，这一条是本机模板写的。",
};

export function Draft(): React.JSX.Element {
  const [pasted, setPasted] = useState("");
  const [outcome, setOutcome] = useState<Outcome>({ kind: "none" });
  const [notices, setNotices] = useState<DraftNotices | null>(null);
  /** Keys [`Result`], so a second draft arrives in a box that was reset. */
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let live = true;
    draftNotices().then(
      (value) => {
        if (live) setNotices(value);
      },
      () => {
        // The screen still works without them; what it must not do is invent
        // a reassuring sentence of its own to fill the gap.
        if (live) setNotices(null);
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const write = (): void => {
    setOutcome({ kind: "working" });
    setAttempt((previous) => previous + 1);
    draftReply(pasted).then(
      (draft) => setOutcome({ kind: "draft", draft }),
      (error: unknown) => setOutcome({ kind: "refused", refusal: asRefusal(error) }),
    );
  };

  return (
    <>
      <section className="panel" aria-labelledby="paste-heading">
        <h2 id="paste-heading">你收到的消息</h2>
        <p className="muted">
          贴进来的内容一律当作别人的话。这一版没有接你自己的模型端点，草稿由本机的确定性语气模板写，
          全程没有任何内容离开这台机器。
        </p>
        <label className="field" htmlFor="pasted">
          原文
        </label>
        <textarea
          id="pasted"
          className="paste-box"
          rows={6}
          value={pasted}
          onChange={(event) => setPasted(event.target.value)}
          placeholder="把对方发给你的消息贴在这里"
        />
        <div className="switch-row">
          <button
            type="button"
            className="primary"
            onClick={write}
            disabled={pasted.trim() === "" || outcome.kind === "working"}
          >
            写一版草稿
          </button>
          {notices === null ? null : (
            <span className="muted" data-testid="not-sent-notice">
              {notices.not_sent}
            </span>
          )}
        </div>
      </section>

      {outcome.kind === "refused" ? (
        <section className="panel refusal" role="alert" aria-labelledby="refused-heading">
          <h2 id="refused-heading">这一次没有写成</h2>
          <p data-testid="refusal-code">{outcome.refusal.reason_code}</p>
          <p>{outcome.refusal.explanation}</p>
        </section>
      ) : null}

      {outcome.kind === "draft" ? <Result key={attempt} draft={outcome.draft} /> : null}
    </>
  );
}

interface ResultProps {
  readonly draft: DraftValue;
}

/**
 * The draft, editable, with the core's own sentences under it.
 *
 * Editable because it is the user's text now: they will change it before they
 * use it, and a read-only box would just mean copying it somewhere else first.
 * Nothing reads it back. The caller keys this component per attempt, so the
 * edited text is discarded when a new draft arrives rather than surviving into
 * one it does not belong to.
 */
function Result({ draft }: ResultProps): React.JSX.Element {
  const [text, setText] = useState(draft.text);
  const [copied, setCopied] = useState(false);

  const copy = (): void => {
    void navigator.clipboard?.writeText(text).then(
      () => setCopied(true),
      () => setCopied(false),
    );
  };

  return (
    <section className="panel" aria-labelledby="draft-heading">
      <h2 id="draft-heading">草稿</h2>
      <label className="field" htmlFor="draft-text">
        你可以直接改
      </label>
      <textarea
        id="draft-text"
        className="paste-box"
        rows={6}
        value={text}
        onChange={(event) => setText(event.target.value)}
      />
      <div className="switch-row">
        <button type="button" className="primary" onClick={copy}>
          复制
        </button>
        {copied ? <span className="muted">已复制到剪贴板。</span> : null}
      </div>
      <ul className="facts">
        <li data-testid="draft-not-sent">{draft.not_sent_notice}</li>
        <li>{draft.source_notice}</li>
        {draft.degraded === null ? null : <li>{DEGRADED[draft.degraded]}</li>}
        {draft.injection_signals.length === 0 ? null : (
          <li data-testid="injection-notice">
            你贴进来的内容里有 {draft.injection_signals.length} 处写成了命令的样子。Soul 只把它当材料读，
            不会照着做，也不会去打开里面的链接。
          </li>
        )}
      </ul>
    </section>
  );
}
