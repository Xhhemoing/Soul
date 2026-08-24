/**
 * 起草. A box to paste into, two buttons that write, and no way to send.
 *
 * The absence is the feature, so it is worth saying where it comes from. This
 * screen cannot send because there is no command to send with: `core.ts` names
 * every command the shell has, `src-tauri/tests/command_surface.rs` checks that
 * list against the Rust one, and `soulcore`'s drafting surface is searched for
 * the word by `crates/soulcore/tests/draft_commands.rs`. Hiding a button would
 * be a promise; having nothing to bind one to is a fact.
 *
 * ## The two paths, and the screen between them
 *
 * 写一版草稿 is the local one: `soulcore` builds no request body at all, so
 * there is nothing for a bug to leak. The other button prepares a request
 * against the endpoint the user configured and stops — what comes back is a
 * description of it, and until somebody presses 确认 nothing has left. The
 * approval is the core's own value echoed back unchanged, both halves of it;
 * `soulcore` refuses one that does not match the preparation it is holding,
 * which is what keeps "the plan you read is the plan that runs" true rather
 * than merely likely.
 *
 * Everything the user reads about sending comes from the core rather than from
 * this file. What is decided here is layout.
 */

import { useEffect, useState } from "react";

import {
  discardDraft,
  draftNotices,
  draftReply,
  generateDraft,
  prepareDraft,
  type Draft as DraftValue,
  type DraftNotices,
  type E1DraftPlan,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

/** What the core said, in the shape the screen renders it. */
type Outcome =
  | { readonly kind: "none" }
  | { readonly kind: "working" }
  | { readonly kind: "draft"; readonly draft: DraftValue }
  | { readonly kind: "refused"; readonly refusal: Refusal };

const DEGRADED: Record<string, string> = {
  reply_unreadable: "端点的回复读不出来，这一条是本机模板写的。",
  reply_empty: "端点没有给出内容，这一条是本机模板写的。",
  reply_clinical: "端点的回复里有这个产品不说的词，已经丢掉，这一条是本机模板写的。",
};

export function Draft(): React.JSX.Element {
  const [pasted, setPasted] = useState("");
  const [outcome, setOutcome] = useState<Outcome>({ kind: "none" });
  const [notices, setNotices] = useState<DraftNotices | null>(null);
  /** The request the user is being asked about, if there is one. */
  const [plan, setPlan] = useState<E1DraftPlan | null>(null);
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
    setPlan(null);
    setAttempt((previous) => previous + 1);
    draftReply(pasted).then(
      (draft) => setOutcome({ kind: "draft", draft }),
      (error: unknown) => setOutcome({ kind: "refused", refusal: asRefusal(error) }),
    );
  };

  /** Step one: ask the core what it would send, and show that. */
  const describe = (): void => {
    setOutcome({ kind: "working" });
    setPlan(null);
    prepareDraft(pasted).then(
      (prepared) => {
        setPlan(prepared);
        setOutcome({ kind: "none" });
      },
      (error: unknown) => setOutcome({ kind: "refused", refusal: asRefusal(error) }),
    );
  };

  /** Step two: hand back the value that was on screen, unchanged. */
  const approve = (approved: E1DraftPlan): void => {
    setOutcome({ kind: "working" });
    setPlan(null);
    setAttempt((previous) => previous + 1);
    generateDraft({
      preparation_id: approved.preparation_id,
      plan_hash: approved.plan_hash,
    }).then(
      (draft) => setOutcome({ kind: "draft", draft }),
      (error: unknown) => setOutcome({ kind: "refused", refusal: asRefusal(error) }),
    );
  };

  const abandon = (): void => {
    setPlan(null);
    setOutcome({ kind: "none" });
    void discardDraft();
  };

  return (
    <>
      <section className="panel" aria-labelledby="paste-heading">
        <h2 id="paste-heading">你收到的消息</h2>
        <p className="muted">
          贴进来的内容一律当作别人的话。写一版草稿走本机的确定性语气模板，不构造任何请求；
          用你自己配置的模型端点写会先把「这一次要发出去什么」摆给你看，你不点确认就不会发出去。
          两条路都不会替你把草稿发给任何人。
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
          <button
            type="button"
            onClick={describe}
            disabled={pasted.trim() === "" || outcome.kind === "working"}
          >
            用你自己的模型端点写
          </button>
          {notices === null ? null : (
            <span className="muted" data-testid="not-sent-notice">
              {notices.not_sent}
            </span>
          )}
        </div>
      </section>

      {plan === null ? null : (
        <Confirm plan={plan} onApprove={approve} onAbandon={abandon} />
      )}

      {outcome.kind === "refused" ? (
        <Refused title="这一次没有写成" refusal={outcome.refusal} testId="refusal-code" />
      ) : null}

      {outcome.kind === "draft" ? <Result key={attempt} draft={outcome.draft} /> : null}
    </>
  );
}

interface ConfirmProps {
  readonly plan: E1DraftPlan;
  readonly onApprove: (plan: E1DraftPlan) => void;
  readonly onAbandon: () => void;
}

/**
 * The screen between the two steps: what would go out, as counts.
 *
 * The third party's words are not here, and their absence is the design. The
 * plan the core hashes carries counts and a model name precisely so that
 * approving it cannot mean approving prose nobody re-read; showing the text
 * here would put back the thing the hash was kept clean of. What the user is
 * being asked is whether a request of this shape may run.
 *
 * The two identifiers are shown because they are what gets echoed back. A
 * mismatch is refused by `soulcore` and nothing leaves, so they are also the
 * one part of this panel a person could check against a refusal message.
 */
function Confirm({ plan, onApprove, onAbandon }: ConfirmProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="confirm-heading">
      <h2 id="confirm-heading">确认这一次要生成什么</h2>
      <p data-testid="e1-notice">{plan.notice}</p>
      <ul className="facts">
        <li data-testid="e1-model">
          模型：<code>{plan.model}</code>
        </li>
        <li data-testid="e1-counts">
          别人的话 {plan.third_party_turns} 段，其中已占位 {plan.placeheld_turns} 段。
        </li>
        <li data-testid="e1-exempted">
          {plan.carries_exempted_original
            ? "有一段是你二次确认过、按原文带上的。"
            : "没有任何一段按原文带上。"}
        </li>
        <li data-testid="e1-plan-hash">
          计划哈希：<code>{plan.plan_hash}</code>
        </li>
        <li data-testid="e1-preparation-id">
          这次准备的编号：<code>{plan.preparation_id}</code>
        </li>
      </ul>
      <div className="switch-row">
        <button type="button" className="primary" onClick={() => onApprove(plan)}>
          确认，开始生成
        </button>
        <button type="button" onClick={onAbandon}>
          不了，丢掉这次准备
        </button>
      </div>
      <p className="muted" data-testid="e1-not-sent">
        {plan.not_sent_notice}
      </p>
    </section>
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
