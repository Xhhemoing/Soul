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

import { useEffect, useRef, useState, useSyncExternalStore } from "react";

import {
  discardDraft,
  draftNotices,
  draftReply,
  DraftSourceChangedError,
  generateDraft,
  getDraftSourceRevision,
  prepareDraft,
  draftReplyWithMemory,
  prepareDraftWithMemory,
  localDraftWithInsight,
  prepareDraftWithInsight,
  subscribeDraftSource,
  type Draft as DraftValue,
  type DraftNotices,
  type E1DraftPlan,
  type Refusal,
  type MemoryDraftPlan,
  type InsightDraftPlan,
  type AssistanceContextView,
} from "../core";
import { asRefusal, Refused } from "../refusal";
import {
  draftLane, DraftRequests, isDraftRefusal,
  type DraftQuote, type DraftRequestKind, type DraftTicket,
} from "./draftRequests";
import { DraftMemory, MemoryReferences, type MemoryTask } from "./DraftMemory";
import { requestDraftDiscard } from "./draftDiscard";
import { DraftInsights, InsightReferences, type InsightTask } from "./DraftInsights";
import { draftAssistanceSelection, draftInsightSelection, draftMaterialSelection, draftMemoryCandidates } from "./draftMaterialSelection";
import { DraftAssistance } from "./DraftAssistance";
import { getRouteAddress, subscribeRouteAddress, useRouteAddress } from "../routeAddress";
import { DraftScopedVoice } from "./DraftScopedVoice";

/** What the core said, in the shape the screen renders it. */
type Outcome =
  | { readonly kind: "none" }
  | { readonly kind: "working" }
  | { readonly kind: "unknown" }
  | { readonly kind: "source_changed" }
  | { readonly kind: "draft"; readonly draft: DraftValue }
  | { readonly kind: "refused"; readonly refusal: Refusal };

const DEGRADED: Record<string, string> = {
  reply_unreadable: "端点的回复读不出来，这一条是本机模板写的。",
  reply_empty: "端点没有给出内容，这一条是本机模板写的。",
  reply_clinical: "端点的回复里有这个产品不说的词，已经丢掉，这一条是本机模板写的。",
};

export function Draft(): React.JSX.Element {
  const address = useRouteAddress();
  const material = useSyncExternalStore(draftMaterialSelection.subscribe, draftMaterialSelection.generation);
  const lane = useSyncExternalStore(draftLane.subscribe, draftLane.snapshot);
  const [scope, setScope] = useState<{ readonly address: typeof address; readonly material: number } | null>(null);
  const scoped = scope !== null && scope.address === address && scope.material === material;
  const hasMaterials = draftInsightSelection.snapshot() !== null || draftAssistanceSelection.snapshot() !== null || draftMemoryCandidates.snapshot().length > 0;
  const enter = (): void => {
    if (hasMaterials || address !== getRouteAddress()) return;
    const slot = draftLane.begin("discard");
    if (slot === null) return;
    // Clear an ordinary preparation before mounting the isolated local scope.
    void discardDraft().then(() => {
      draftLane.finish(slot, false);
      if (address === getRouteAddress() && material === draftMaterialSelection.generation()) setScope({ address, material });
    }, () => { draftLane.finish(slot, true); });
  };
  return scoped ? <DraftScopedVoice onExit={() => setScope(null)} /> : <>
    <section className="panel"><button type="button" disabled={hasMaterials || lane.busy || lane.uncertain} onClick={enter}>按联系人和场合本地起草</button>
      <p className="muted">这是普通本地模板的可选范围。使用手选记忆、Insight 或 Assistance 时，先结束材料选择再进入。</p></section>
    <OrdinaryDraft />
  </>;
}

function OrdinaryDraft(): React.JSX.Element {
  const [input, setInput] = useState({ text: "", revision: 0 });
  const [requests] = useState(() => new DraftRequests());
  const lane = useSyncExternalStore(draftLane.subscribe, draftLane.snapshot);
  const [outcome, setOutcome] = useState<Outcome>({ kind: "none" });
  const [notices, setNotices] = useState<DraftNotices | null>(null);
  /** The request the user is being asked about, if there is one. */
  const [quote, setQuote] = useState<DraftQuote | null>(null);
  /** Keys [`Result`], so a second draft arrives in a box that was reset. */
  const [attempt, setAttempt] = useState(0);
  const [memory, setMemory] = useState<MemoryTask | null>(null);
  const [memoryPlan, setMemoryPlan] = useState<MemoryDraftPlan | null>(null);
  const selection = useSyncExternalStore(draftInsightSelection.subscribe, draftInsightSelection.snapshot);
  const [insight, setInsight] = useState<InsightTask | null>(null);
  const [insightPlan, setInsightPlan] = useState<InsightDraftPlan | null>(null);
  const assistanceSelection = useSyncExternalStore(draftAssistanceSelection.subscribe, draftAssistanceSelection.snapshot);
  const [assistance, setAssistance] = useState<AssistanceContextView | null>(null);
  const pasteGeneration = useRef(0);
  const [pasteRevision, setPasteRevision] = useState(0);
  const materialGeneration = draftMaterialSelection.generation();
  const address = useRouteAddress();
  const [displayOwner, setDisplayOwner] = useState(() => ({ address, source: getDraftSourceRevision(), material: materialGeneration, paste: 0 }));
  const displayCurrent = displayOwner.address === address && address === getRouteAddress()
    && displayOwner.source === getDraftSourceRevision() && displayOwner.material === materialGeneration
    && displayOwner.paste === pasteRevision;
  const rememberDisplay = (): void => {
    setDisplayOwner({ address, source: getDraftSourceRevision(), material: draftMaterialSelection.generation(), paste: pasteGeneration.current });
  };

  useEffect(() => {
    let live = true;
    requests.activate();
    let source = getDraftSourceRevision();
    const unsubscribe = subscribeDraftSource((next) => {
      if (next === source) return;
      source = next;
      const revision = requests.invalidate();
      setInput((previous) => ({ ...previous, revision }));
      setQuote(null);
      setOutcome({ kind: "none" });
      setMemory(null);
      setMemoryPlan(null);
      setInsight(null); setInsightPlan(null);
      setAssistance(null);
    });
    const unsubscribeSelection = draftMaterialSelection.subscribe(() => {
      const revision = requests.invalidate();
      setInput((previous) => ({ ...previous, revision }));
      setQuote(null); setMemory(null); setMemoryPlan(null); setInsight(null); setInsightPlan(null); setOutcome({ kind: "none" });
      setAssistance(null);
    });
    const unsubscribeAddress = subscribeRouteAddress(() => {
      const revision = requests.invalidate(); setInput((previous) => ({ ...previous, revision }));
      setQuote(null); setMemory(null); setMemoryPlan(null); setInsight(null); setInsightPlan(null); setAssistance(null); setOutcome({ kind: "none" });
      if (draftAssistanceSelection.snapshot() !== null || draftInsightSelection.snapshot() !== null || draftMemoryCandidates.snapshot().length > 0) requestDraftDiscard();
    });
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
      unsubscribe();
      unsubscribeSelection();
      unsubscribeAddress();
      requests.dispose();
      if (draftInsightSelection.snapshot() !== null || draftAssistanceSelection.snapshot() !== null) requestDraftDiscard();
    };
  }, [requests]);

  const start = (kind: DraftRequestKind, expected?: DraftQuote): DraftTicket | null => {
    if (materialGeneration !== draftMaterialSelection.generation() || address !== getRouteAddress()) return null;
    if (!displayCurrent && (memory !== null || insight !== null || assistance !== null || quote !== null)) return null;
    if ((kind === "local" || kind === "prepare") && assistanceSelection !== null && (assistance === null
      || (assistance.kind === "memory" ? memory?.context.context_id == null : insight?.context_id == null))) return null;
    if ((kind === "local" || kind === "prepare") && ((selection !== null && (insight === null || insight.context_id === null)) || (memory !== null && memory.context.context_id === null))) return null;
    const ticket = requests.begin(kind, input.revision, expected);
    if (ticket === null) return null;
    rememberDisplay();
    setOutcome({ kind: "working" });
    setQuote(null);
    if (kind === "prepare" || kind === "original" || kind === "local") { setMemoryPlan(null); setInsightPlan(null); }
    setAttempt((previous) => previous + 1);
    return ticket;
  };

  const failed = (ticket: DraftTicket, error: unknown): void => {
    const known = isDraftRefusal(error);
    const changed = error instanceof DraftSourceChangedError;
    if (requests.finish(ticket, !known && !changed) && address === getRouteAddress() && materialGeneration === draftMaterialSelection.generation()) {
      setOutcome(changed ? { kind: "source_changed" }
        : known ? { kind: "refused", refusal: asRefusal(error) } : { kind: "unknown" });
      if (insight !== null) setInsight({ ...insight, context_id: null, include_counts: false, include_goal: false, goal_preview: null });
    }
  };

  // Admission happens before React renders disabled buttons. Ownership is
  // checked again at the microtask that actually starts IPC, so a synchronous
  // edit/unmount cannot accidentally start a stale approval or exemption.
  const run = (ticket: DraftTicket, work: () => Promise<void>): void => {
    void Promise.resolve().then(async () => {
      if (!requests.owns(ticket) || materialGeneration !== draftMaterialSelection.generation() || address !== getRouteAddress()) {
        requests.finish(ticket);
        return;
      }
      try {
        await work();
      } catch (error: unknown) {
        failed(ticket, error);
      }
    });
  };
  const finish = (ticket: DraftTicket): boolean => {
    const current = requests.finish(ticket) && materialGeneration === draftMaterialSelection.generation() && address === getRouteAddress();
    if (current) rememberDisplay();
    return current;
  };

  const write = (): void => {
    const ticket = start("local");
    if (ticket === null) return;
    run(ticket, async () => {
      if ((selection !== null || assistance?.kind === "insight") && insight?.context_id !== null && insight?.context_id !== undefined) {
        const result = await localDraftWithInsight(ticket.text, insight.context_id);
        if (finish(ticket)) {
          setOutcome({ kind: "draft", draft: result.draft });
          setInsight({ context_id: null, reference: result.reference, include_counts: false, include_goal: false, goal_preview: null });
        }
        return;
      }
      const result = memory?.context.context_id === undefined || memory.context.context_id === null
        ? { draft: await draftReply(ticket.text), context: null } : await draftReplyWithMemory(ticket.text, memory.context.context_id);
      if (finish(ticket)) {
        setOutcome({ kind: "draft", draft: result.draft });
        if (result.context !== null) setMemory({ context: { context_id: null, preview: result.context }, owner_ids: [] });
      }
    });
  };

  /**
   * Step one: ask the core what it would send, and show that.
   *
   * `includeOriginal` is the user's second confirmation, and it is passed per
   * call rather than kept in state: pressing 「这一条按原文带上」 re-prepares this
   * same paste with it set, and every later preparation starts from false
   * again because there is no variable here that survives one.
   */
  const describe = (expected?: DraftQuote): void => {
    const ticket = start(expected === undefined ? "prepare" : "original", expected);
    if (ticket === null) return;
    run(ticket, async () => {
      if ((selection !== null || assistance?.kind === "insight") && insight?.context_id !== null && insight?.context_id !== undefined) {
        const material = await prepareDraftWithInsight(ticket.text, insight.context_id, insight.include_counts, expected !== undefined, insight.include_goal);
        const next = requests.remember(ticket, material.prepared.plan);
        if (finish(ticket) && next !== null) {
          setQuote(next); setOutcome({ kind: "none" }); setInsightPlan(material);
          setInsight({ ...insight, reference: material.reference, include_counts: material.prepared.includes_counts,
            include_goal: material.prepared.includes_goal, goal_preview: material.goal_preview });
        }
        return;
      }
      const material = memory?.context.context_id === undefined || memory.context.context_id === null
        ? null : await prepareDraftWithMemory(ticket.text, memory.context.context_id, memory.owner_ids, expected !== undefined);
      const prepared = material === null ? await prepareDraft(ticket.text, expected !== undefined) : material.prepared.plan;
      const next = requests.remember(ticket, prepared);
      if (finish(ticket) && next !== null) {
        setQuote(next);
        setOutcome({ kind: "none" });
        setMemoryPlan(material);
        if (material !== null) setMemory({
          context: { context_id: memory?.context.context_id ?? null, preview: material.context },
          owner_ids: material.prepared.endpoint_owner_ids,
        });
      }
    });
  };

  /** Step two: hand back the value that was on screen, unchanged. */
  const approve = (approved: DraftQuote): void => {
    const ticket = start("generate", approved);
    if (ticket === null) return;
    run(ticket, async () => {
      const draft = await generateDraft({
        preparation_id: approved.plan.preparation_id,
        plan_hash: approved.plan.plan_hash,
      });
      if (finish(ticket)) {
        setOutcome({ kind: "draft", draft });
        if (memory !== null) setMemory({ ...memory, context: { ...memory.context, context_id: null }, owner_ids: [] });
        if (insight !== null) setInsight({ ...insight, context_id: null });
      }
    });
  };

  const abandon = (abandoned: DraftQuote): void => {
    const ticket = start("discard", abandoned);
    if (ticket === null) return;
    run(ticket, async () => {
      await discardDraft();
      if (finish(ticket)) {
        setOutcome({ kind: "none" });
        setMemory(null); setMemoryPlan(null);
        setInsight(null); setInsightPlan(null);
      }
    });
  };

  const edit = (text: string): void => {
    if (address !== getRouteAddress()) return;
    pasteGeneration.current += 1;
    setPasteRevision(pasteGeneration.current);
    if (quote !== null || memory !== null || insight !== null || draftLane.isPreparing()) requestDraftDiscard();
    setInput({ text, revision: requests.edit(text) });
    setQuote(null);
    setOutcome({ kind: "none" });
    setMemory(null); setMemoryPlan(null);
    setInsight(null); setInsightPlan(null);
    setAssistance(null);
  };

  const memoryEdit = (): void => {
    if (address !== getRouteAddress()) return;
    if (quote !== null || memory !== null || insight !== null || draftLane.isPreparing()) requestDraftDiscard();
    setMemory(null); setMemoryPlan(null); setQuote(null); setOutcome({ kind: "none" });
    setInsight(null); setInsightPlan(null);
    setAssistance(null);
    setInput((previous) => ({ ...previous, revision: requests.invalidate() }));
  };
  const unusableMemory = memory !== null && memory.context.context_id === null;
  const unusableInsight = selection !== null && (insight === null || insight.context_id === null);
  const unusableAssistance = assistanceSelection !== null && (assistance === null
    || (assistance.kind === "memory" ? memory?.context.context_id == null : insight?.context_id == null));

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
          value={input.text}
          onChange={(event) => edit(event.target.value)}
          placeholder="把对方发给你的消息贴在这里"
        />
        <div className="switch-row">
          <button
            type="button"
            className="primary"
            onClick={write}
            disabled={input.text.trim() === "" || lane.busy || unusableMemory || unusableInsight || unusableAssistance}
          >
            写一版草稿
          </button>
          <button
            type="button"
            onClick={() => describe()}
            disabled={input.text.trim() === "" || lane.busy || lane.uncertain || unusableMemory || unusableInsight || unusableAssistance}
          >
            用你自己的模型端点写
          </button>
          {notices === null ? null : (
            <span className="muted" data-testid="not-sent-notice">
              {notices.not_sent}
            </span>
          )}
        </div>
        {lane.busy ? (
          <p role="status">正在等待本次操作结束。修改原文会使旧结果失效，不会取消已经开始的操作。</p>
        ) : null}
        {lane.uncertain ? (
          <p role="alert" data-testid="draft-recovery">
            上一次端点操作的结果未能确认。为避免旧操作影响新的准备，端点起草已暂停，
            不会自动重试。请保留原文，完全退出并重新打开应用后再准备；本机模板仍可使用。
          </p>
        ) : null}
      </section>

      {assistanceSelection !== null ? <DraftAssistance selection={assistanceSelection} context={displayCurrent ? assistance : null} memory={displayCurrent ? memory : null} insight={displayCurrent ? insight : null}
        busy={lane.busy || lane.uncertain} hasQuote={quote !== null} getHasQuote={() => requests.hasQuote()} getPasteGeneration={() => pasteGeneration.current}
        onPrepared={(value) => {
          rememberDisplay();
          setAssistance(value); setMemory(value.kind === "memory" ? { context: value.memory, owner_ids: [] } : null);
          setInsight(value.kind === "insight" ? { ...value.insight, include_counts: false, include_goal: false, goal_preview: value.goal_preview } : null);
          setQuote(null); setMemoryPlan(null); setInsightPlan(null); setOutcome({ kind: "none" });
          setInput((previous) => ({ ...previous, revision: requests.invalidate() }));
        }} onMemory={setMemory} onInsight={setInsight} /> : selection === null ? <DraftMemory value={displayCurrent ? memory : null} busy={lane.busy || lane.uncertain} hasQuote={quote !== null} getHasQuote={() => requests.hasQuote()} getPasteGeneration={() => pasteGeneration.current} onEdit={memoryEdit} onChange={(value) => {
        rememberDisplay();
        setMemory(value); setQuote(null); setMemoryPlan(null); setOutcome({ kind: "none" });
        setInput((previous) => ({ ...previous, revision: requests.invalidate() }));
      }} /> : <DraftInsights key={selection.revision} selection={selection} value={displayCurrent ? insight : null} busy={lane.busy || lane.uncertain} canClear={!lane.busy} hasQuote={quote !== null} getHasQuote={() => requests.hasQuote()}
        getPasteGeneration={() => pasteGeneration.current} onClear={() => draftInsightSelection.clear()} onChange={(value) => {
          rememberDisplay();
          setInsight(value); setMemory(null); setMemoryPlan(null); setQuote(null); setInsightPlan(null); setOutcome({ kind: "none" });
          setInput((previous) => ({ ...previous, revision: requests.invalidate() }));
        }} />}
      {!displayCurrent || insightPlan === null ? null : <section className="panel"><h2>这份准备实际保留的统计材料</h2>
        <InsightReferences reference={insightPlan.reference} />
        {insightPlan.goal_preview === null ? null : <MemoryReferences preview={insightPlan.goal_preview} />}
        <p>{insightPlan.prepared.includes_goal ? "本次模型请求包含另行勾选的本人目标脱敏节选。目标出处只说明这份材料的来源，不证明整篇草稿的事实。" : "本次模型请求未纳入本人目标正文。"}</p>
        <p>完整请求 UTF-8 JSON：{insightPlan.prepared.encoded_bytes} 字节（含固定指令和围栏，不是 token 数）。</p>
        <p>{insightPlan.prepared.includes_counts ? "本次模型请求包含已勾选的匿名 A2 派生计数。" : "本次模型请求未获得统计计数，仅保留本地参考标记。"}</p>
        <details><summary>查看实际脱敏统计材料</summary><pre className="pre-wrap">{insightPlan.prepared.body_preview}</pre></details>
      </section>}
      {!displayCurrent || memoryPlan === null ? null : <section className="panel"><h2>这份准备实际保留的记忆与端点材料</h2>
        <MemoryReferences preview={memoryPlan.context} /><p>完整请求 UTF-8 JSON：{memoryPlan.prepared.encoded_bytes} 字节（含固定指令和围栏，不是 token 数）。</p>
        <p>明确提供的本人记录：{memoryPlan.prepared.endpoint_owner_ids.join("、") || "无，记忆正文保持本地"}</p>
        <details><summary>查看实际脱敏材料</summary><pre className="pre-wrap">{memoryPlan.prepared.body_preview}</pre></details>
      </section>}
      {!displayCurrent || quote === null ? null : (
        <Confirm
          plan={quote.plan}
          onApprove={() => approve(quote)}
          onCarryOriginal={() => describe(quote)}
          onAbandon={() => abandon(quote)}
        />
      )}

      {outcome.kind === "refused" ? (
        <Refused title="这一次未得到可用草稿" refusal={outcome.refusal} testId="refusal-code" />
      ) : null}

      {outcome.kind === "unknown" && !lane.uncertain ? (
        <p role="alert">未能确认这次操作的结果。原文仍然保留，请核对后再决定下一步。</p>
      ) : null}

      {outcome.kind === "source_changed" ? (
        <p role="status" data-testid="draft-source-changed">
          资料或端点正在更新，或已经发生变化。请等相关操作完成后重新准备；原文仍然保留。
        </p>
      ) : null}

      {displayCurrent && outcome.kind === "draft" ? <Result key={attempt} draft={outcome.draft} /> : null}
    </>
  );
}

interface ConfirmProps {
  readonly plan: E1DraftPlan;
  readonly onApprove: (plan: E1DraftPlan) => void;
  /** The second confirmation: prepare this same paste again, unplaceheld. */
  readonly onCarryOriginal: () => void;
  readonly onAbandon: () => void;
}

/**
 * The screen between the two steps: counts and the complete redacted
 * material held by the core, which the user can inspect before approving.
 *
 * The two identifiers are shown because they are what gets echoed back. A
 * mismatch is refused by `soulcore` and nothing leaves, so they are also the
 * one part of this panel a person could check against a refusal message.
 *
 * ## Where the second confirmation lives
 *
 * 「这一条按原文带上」 is the exemption PRODUCT_LOCK describes, and this panel is
 * the only place it can be reached from: the user has already read a plan
 * saying the other person's words are placeheld, and pressing it prepares the
 * same paste again with that one turn unplaceheld. It is a second preparation
 * and not a send — 确认，开始生成 is still the only button that spends one — so
 * the plan on screen after it is a plan the user still has to approve.
 *
 * It disappears once the plan already carries the original, because a plan
 * cannot be exempted twice: there is one third-party turn in a paste, and the
 * counts above say what happened to it.
 */
function Confirm({
  plan,
  onApprove,
  onCarryOriginal,
  onAbandon,
}: ConfirmProps): React.JSX.Element {
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
      <h3>这次将提供给模型的脱敏材料</h3>
      <p>请完整核对以下内容；如果仍有不想提供的信息，可以取消这次准备。</p>
      <pre className="reading" style={{ overflowWrap: "anywhere" }} data-testid="e1-body-preview">{plan.body_preview}</pre>
      <div className="switch-row">
        <button type="button" className="primary" onClick={() => onApprove(plan)}>
          确认，开始生成
        </button>
        {plan.carries_exempted_original ? null : (
          <button type="button" data-testid="e1-carry-original" onClick={onCarryOriginal}>
            这一条按原文带上
          </button>
        )}
        <button type="button" onClick={onAbandon}>
          不了，丢掉这次准备
        </button>
      </div>
      {plan.carries_exempted_original ? null : (
        <p className="muted" data-testid="e1-carry-original-notice">
          按原文带上只对这一次准备有效，按下之后会重新准备一次给你看，仍然要你按确认才会生成；
          下一次准备又回到占位。姓名和账号在任何一种情况下都还是占位的。
        </p>
      )}
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
  const [copyStatus, setCopyStatus] = useState<"idle" | "copied" | "failed">("idle");
  const copyAttempt = useRef<symbol | null>(null);

  useEffect(() => () => { copyAttempt.current = null; }, []);

  const copy = (): void => {
    const attempt = Symbol("draft copy");
    copyAttempt.current = attempt;
    setCopyStatus("idle");
    void (async () => {
      try {
        await navigator.clipboard.writeText(text);
        if (copyAttempt.current === attempt) setCopyStatus("copied");
      } catch {
        if (copyAttempt.current === attempt) setCopyStatus("failed");
      }
    })();
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
        autoFocus
        rows={6}
        value={text}
        onChange={(event) => {
          copyAttempt.current = null;
          setCopyStatus("idle");
          setText(event.target.value);
        }}
      />
      <div className="switch-row">
        <button type="button" className="primary" onClick={copy}>
          复制
        </button>
        {copyStatus === "copied" ? <span className="muted">已复制到剪贴板。</span> : null}
        {copyStatus === "failed" ? <span role="alert">复制未完成，请选中草稿文字手动复制，或再次尝试。</span> : null}
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
