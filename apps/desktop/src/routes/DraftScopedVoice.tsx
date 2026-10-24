import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import {
  confirmScopedRegister, discardScopedRegisterProposal, draftScopedReply, proposeScopedRegister,
  scopedRegisterRecipients, subscribeDraftSource, getDraftSourceRevision,
  scopedRegisterStatus,
  subscribeExternalSource, hasPendingSourceChanges,
  getExternalSourceRevision,
  DraftSourceChangedError,
  type Refusal, type ScopedDraftResult, type ScopedOccasion, type ScopedRegister,
  type ScopedRegisterPreference, type ScopedRegisterProposal, type ScopedRegisterRecipient,
  type ScopedRegisterStatus,
} from "../core";
import { asRefusal, Refused } from "../refusal";
import { getRouteAddress, subscribeRouteAddress } from "../routeAddress";
import { requestDraftDiscard } from "./draftDiscard";
import { draftLane, isDraftRefusal } from "./draftRequests";
import { draftMaterialSelection } from "./draftMaterialSelection";
import { memoryHash } from "./memoryTarget";

const REGISTER: Readonly<Record<ScopedRegister, string>> = { casual: "Casual · 随意", plain: "Plain · 平实", formal: "Formal · 正式" };
const STATUS: Readonly<Record<ScopedRegisterStatus["state"], string>> = {
  absent: "此范围还没有已确认偏好。", active: "此范围当前有已确认偏好。", inactive: "此范围偏好当前未启用。",
  forgotten: "此范围偏好已遗忘。", unavailable: "此范围当前不可用。",
};

/** Only explicit local-template scope. Edited prose never crosses this boundary. */
export function DraftScopedVoice({ onExit }: { readonly onExit: () => void }): React.JSX.Element {
  const lane = useSyncExternalStore(draftLane.subscribe, draftLane.snapshot);
  const [recipients, setRecipients] = useState<readonly ScopedRegisterRecipient[]>([]);
  const [pasted, setPasted] = useState("");
  const [person, setPerson] = useState("");
  const [occasion, setOccasion] = useState<ScopedOccasion | "">("");
  const [register, setRegister] = useState<ScopedRegister | "">("");
  const [result, setResult] = useState<ScopedDraftResult | null>(null);
  const [text, setText] = useState("");
  const [proposal, setProposal] = useState<ScopedRegisterProposal | null>(null);
  const [saved, setSaved] = useState<ScopedRegisterPreference | null>(null);
  const [confirmationSourceChanged, setConfirmationSourceChanged] = useState(false);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [phase, setPhase] = useState<"idle" | "proposing" | "confirming" | "discarding" | "blocked">("idle");
  const [appliedCurrent, setAppliedCurrent] = useState(true);
  const [copyStatus, setCopyStatus] = useState<"idle" | "copied" | "failed">("idle");
  const copyAttempt = useRef<symbol | null>(null);
  const [scopeStatus, setScopeStatus] = useState<ScopedRegisterStatus | null>(null);
  const [readingStatus, setReadingStatus] = useState(false);
  const statusRequest = useRef<symbol | null>(null);
  const statusWasRequested = useRef(false);
  const externalRefresh = useRef(false);
  const [externalReadAttempt, setExternalReadAttempt] = useState(0);
  const live = useRef(false);
  const generation = useRef(0);
  const editRevision = useRef(0);
  const held = useRef<ScopedRegisterProposal | null>(null);
  const hasNativeState = useRef(false);
  const confirming = useRef(false);
  const confirmationSourceEvents = useRef(0);
  const discarding = useRef(false);
  const blocked = useRef(false);
  const address = useRef(getRouteAddress());
  const material = useRef(draftMaterialSelection.generation());
  const locked = phase === "confirming" || phase === "discarding" || phase === "blocked";
  const current = (version: number): boolean => live.current && generation.current === version
    && address.current === getRouteAddress() && material.current === draftMaterialSelection.generation();

  useEffect(() => {
    live.current = true;
    const version = generation.current;
    void scopedRegisterRecipients().then(
      (value) => { if (live.current && generation.current === version) setRecipients(value); },
      (error: unknown) => { if (live.current && generation.current === version) setRefusal(asRefusal(error)); },
    );
    let source = getDraftSourceRevision();
    let externalSource = getExternalSourceRevision();
    const unsubscribe = subscribeDraftSource((next) => {
      if (next === source) return;
      source = next;
      const external = externalSource !== getExternalSourceRevision();
      externalSource = getExternalSourceRevision();
      statusRequest.current = null; setScopeStatus(null); setReadingStatus(false);
      setAppliedCurrent(false);
      if (externalRefresh.current && !hasPendingSourceChanges()) setExternalReadAttempt((value) => value + 1);
      if (confirming.current) { confirmationSourceEvents.current += 1; return; }
      generation.current += 1;
      held.current = null;
      setProposal(null); setResult(null);
      if (external) setConfirmationSourceChanged(true);
      else setSaved(null);
      // The source mutation invalidates the Session's held task as well. The
      // page refuses old delivery immediately; it never claims cancellation.
    });
    const unsubscribeExternal = subscribeExternalSource(() => {
      externalRefresh.current = true; setRecipients([]);
      if (!hasPendingSourceChanges()) setExternalReadAttempt((value) => value + 1);
    });
    const leave = (): void => {
      statusRequest.current = null; setScopeStatus(null); setReadingStatus(false);
      generation.current += 1;
      held.current = null;
      if (hasNativeState.current) { hasNativeState.current = false; requestDraftDiscard(); }
      setProposal(null); setResult(null); setSaved(null);
    };
    const unsubscribeAddress = subscribeRouteAddress(leave);
    const unsubscribeMaterial = draftMaterialSelection.subscribe(leave);
    return () => {
      live.current = false;
      statusRequest.current = null;
      generation.current += 1;
      held.current = null;
      unsubscribe(); unsubscribeExternal(); unsubscribeAddress(); unsubscribeMaterial();
      if (hasNativeState.current) { hasNativeState.current = false; requestDraftDiscard(); }
    };
  }, []);

  useEffect(() => {
    if (!externalRefresh.current || hasPendingSourceChanges() || phase !== "idle" || lane.busy) return;
    externalRefresh.current = false;
    const version = generation.current;
    const source = getDraftSourceRevision();
    let active = true;
    const owns = (): boolean => active && live.current && generation.current === version && source === getDraftSourceRevision();
    void scopedRegisterRecipients().then((value) => { if (owns()) setRecipients(value); },
      (error: unknown) => { if (owns()) setRefusal(asRefusal(error)); });
    if (statusWasRequested.current && person !== "" && occasion !== "") {
      void scopedRegisterStatus(person, occasion).then((value) => { if (owns()) setScopeStatus(value); },
        (error: unknown) => { if (owns()) setRefusal(asRefusal(error)); });
    }
    return () => { active = false; };
  }, [externalReadAttempt, phase, lane.busy, person, occasion]);

  /** Synchronously surrender ownership; a displayed preview requires an ack
   * before a user edit can be applied. A pending preview is cleaned on arrival. */
  const change = (apply: () => void): void => {
    if (!live.current || confirming.current || discarding.current || blocked.current || address.current !== getRouteAddress()) return;
    generation.current += 1;
    const version = generation.current;
    const previous = held.current;
    held.current = null;
    setProposal(null); setSaved(null); setCopyStatus("idle");
    const applyChange = (): void => {
      // A proposal discard can delay this edit. Copies started while waiting
      // still belong to the text/scope before the edit is actually applied.
      copyAttempt.current = null;
      setCopyStatus("idle");
      apply();
    };
    if (previous === null) { applyChange(); return; }
    const slot = draftLane.begin("discard");
    if (slot === null) { blocked.current = true; setPhase("blocked"); return; }
    discarding.current = true;
    setPhase("discarding");
    void discardScopedRegisterProposal(previous.proposal_id).then(() => {
      draftLane.finish(slot, false);
      discarding.current = false;
      if (live.current) setPhase("idle");
      if (current(version)) applyChange();
    }, (error: unknown) => {
      draftLane.finish(slot, true);
      discarding.current = false;
      if (live.current) { blocked.current = true; setPhase("blocked"); setRefusal(asRefusal(error)); }
    });
  };
  const clearResult = (): void => {
    statusWasRequested.current = false;
    statusRequest.current = null; setScopeStatus(null); setReadingStatus(false);
    setResult(null); setRegister(""); setAppliedCurrent(false); editRevision.current = 0;
    if (hasNativeState.current) { hasNativeState.current = false; requestDraftDiscard(); }
  };

  const readStatus = (): void => {
    if (locked || lane.busy || lane.uncertain || statusRequest.current !== null || person === "" || occasion === "" || address.current !== getRouteAddress()) return;
    const request = Symbol("scoped status read");
    statusWasRequested.current = true;
    const version = generation.current;
    statusRequest.current = request; setReadingStatus(true); setScopeStatus(null); setRefusal(null);
    const owns = (): boolean => current(version) && statusRequest.current === request;
    void Promise.resolve().then(async () => {
      if (!owns()) return;
      const value = await scopedRegisterStatus(person, occasion);
      if (owns()) setScopeStatus(value);
    }).catch((error: unknown) => { if (owns()) setRefusal(asRefusal(error)); }).finally(() => {
      if (statusRequest.current === request) { statusRequest.current = null; if (live.current) setReadingStatus(false); }
    });
  };

  const write = (): void => {
    if (locked || person === "" || occasion === "" || pasted.trim() === "" || address.current !== getRouteAddress()) return;
    change(() => {
      const slot = draftLane.begin("local");
      if (slot === null) return;
      const version = generation.current;
      hasNativeState.current = true;
      setResult(null); setRefusal(null); setRegister("");
      void Promise.resolve().then(async () => {
        if (!current(version)) { draftLane.finish(slot, false); return; }
        try {
          const value = await draftScopedReply(pasted, person, occasion);
          if (current(version)) {
            setResult(value); setText(value.draft.text); setAppliedCurrent(true); editRevision.current = 0;
          }
          draftLane.finish(slot, false);
        } catch (error: unknown) {
          draftLane.finish(slot, !isDraftRefusal(error) && !(error instanceof DraftSourceChangedError));
          if (current(version)) setRefusal(asRefusal(error));
        }
      });
    });
  };

  const propose = (): void => {
    if (locked || result === null || register === "" || held.current !== null || !appliedCurrent) return;
    const slot = draftLane.begin("prepare");
    if (slot === null) return;
    const version = generation.current;
    setPhase("proposing"); setRefusal(null);
    void Promise.resolve().then(async () => {
      if (!current(version)) { draftLane.finish(slot, false); if (live.current) setPhase("idle"); return; }
      try {
        const value = await proposeScopedRegister(result.result_id, editRevision.current, register);
        if (current(version)) { held.current = value; setProposal(value); }
        else await discardScopedRegisterProposal(value.proposal_id);
        draftLane.finish(slot, false);
        if (live.current) setPhase("idle");
      } catch (error: unknown) {
        draftLane.finish(slot, !isDraftRefusal(error) && !(error instanceof DraftSourceChangedError));
        if (live.current) { setPhase("idle"); if (current(version)) setRefusal(asRefusal(error)); }
      }
    });
  };

  const confirm = (): void => {
    const preview = held.current;
    if (preview === null || locked || address.current !== getRouteAddress()) return;
    const slot = draftLane.begin("generate");
    if (slot === null) return;
    const version = generation.current;
    held.current = null; setProposal(null); confirming.current = true; confirmationSourceEvents.current = 0;
    setConfirmationSourceChanged(false); setPhase("confirming"); setRefusal(null);
    void Promise.resolve().then(async () => {
      if (!current(version)) { confirming.current = false; draftLane.finish(slot, false); return; }
      try {
        const receipt = await confirmScopedRegister(preview.proposal_id, preview.expected_revision);
        if (current(version)) {
          setSaved(receipt); setResult(null);
          // mutateDraftSource announces this confirmation's start and finish.
          // Extra notifications mean another source operation overlapped; the
          // receipt is still a committed write, not proof of present validity.
          setConfirmationSourceChanged(confirmationSourceEvents.current > 2);
        }
        draftLane.finish(slot, false);
      } catch (error: unknown) {
        draftLane.finish(slot, !isDraftRefusal(error));
        if (current(version)) setRefusal(asRefusal(error));
      } finally {
        confirming.current = false;
        if (live.current) setPhase("idle");
      }
    });
  };

  const copy = (): void => {
    const attempt = Symbol("scoped draft copy");
    const version = generation.current;
    copyAttempt.current = attempt;
    setCopyStatus("idle");
    const owns = (): boolean => current(version) && copyAttempt.current === attempt;
    void (async () => {
      try {
        await navigator.clipboard.writeText(text);
        if (owns()) setCopyStatus("copied");
      } catch {
        if (owns()) setCopyStatus("failed");
      }
    })();
  };

  return <>
    <section className="panel">
      <h2>按联系人和场合本地起草</h2>
      <p>只用本机确定性模板。请明确选择人物和场合；这里的范围偏好仅用于以后相同人物、相同场合的普通本地回复。</p>
      <p>保存的是我的表达选择。普通 Memory 编辑会停用对应语域偏好；要再次启用，需回到起草重新明确确认。</p>
      <label className="field" htmlFor="scoped-person">收件人物</label>
      <select id="scoped-person" value={person} disabled={locked} onChange={(event) => { const value = event.target.value; change(() => { setPerson(value); clearResult(); }); }}>
        <option value="">请选择 Active 人物</option>
        {recipients.map((recipient) => <option key={recipient.person_id} value={recipient.person_id}>{recipient.identifier_hint} · {recipient.person_id}</option>)}
      </select>
      <label className="field" htmlFor="scoped-occasion">场合</label>
      <select id="scoped-occasion" value={occasion} disabled={locked} onChange={(event) => { const value = event.target.value as ScopedOccasion | ""; change(() => { setOccasion(value); clearResult(); }); }}>
        <option value="">请选择场合</option><option value="personal">私人 personal</option><option value="work">工作 work</option><option value="study">学习 study</option>
      </select>
      <button type="button" disabled={locked || lane.busy || lane.uncertain || readingStatus || person === "" || occasion === ""} onClick={readStatus}>查看此范围当前偏好</button>
      {readingStatus ? <p role="status">正在读取所选范围的当前偏好…</p> : null}
      {scopeStatus === null ? null : <section data-testid="scoped-current-status">
        <h3>当前偏好（本次读取快照）</h3><p>{STATUS[scopeStatus.state]}</p><p>{scopeStatus.notice}</p>
        {scopeStatus.preference === null ? null : <p>语域：{REGISTER[scopeStatus.preference.register]}（{scopeStatus.preference.register}）；版本：{scopeStatus.preference.revision}。</p>}
        {scopeStatus.memory_id === null || scopeStatus.state === "forgotten" ? null : <p>Memory：{scopeStatus.memory_id}。<a href={memoryHash(scopeStatus.memory_id)}>打开这条偏好记忆</a></p>}
      </section>}
      <label className="field" htmlFor="scoped-pasted">原文</label>
      <textarea id="scoped-pasted" className="paste-box" rows={6} value={pasted} disabled={locked} onChange={(event) => { const value = event.target.value; change(() => { setPasted(value); clearResult(); }); }} />
      <div className="switch-row"><button type="button" className="primary" disabled={locked || lane.busy || lane.uncertain || person === "" || occasion === "" || pasted.trim() === ""} onClick={write}>写一版本地草稿</button>
        <button type="button" disabled={locked} onClick={onExit}>返回普通起草</button></div>
      {phase === "discarding" ? <p role="status">正在撤销这次偏好预览，收到回执后再应用修改。</p> : null}
      {phase === "confirming" ? <p role="status">正在确认保存，完成前暂时锁定编辑和范围。</p> : null}
      {phase === "blocked" || lane.uncertain ? <p role="alert">未能确认上次操作是否完成。请完全退出并重新打开应用后继续；这里不会自动重试。</p> : null}
    </section>
    {refusal === null ? null : <Refused title="本次范围起草操作未完成" refusal={refusal} testId="scoped-refusal" />}
    {result === null ? null : <section className="panel">
      <h2>草稿</h2><label className="field" htmlFor="scoped-draft-text">你可以直接改</label>
      <textarea id="scoped-draft-text" className="paste-box" rows={6} value={text} disabled={locked} onChange={(event) => { const value = event.target.value; change(() => { editRevision.current += 1; setText(value); }); }} />
      <button type="button" onClick={copy}>复制</button>
      {copyStatus === "copied" ? <span>已复制到剪贴板。</span> : null}
      {copyStatus === "failed" ? <span role="alert">复制未完成，请选中草稿文字手动复制，或再次尝试。</span> : null}
      <p>{result.draft.not_sent_notice}</p><p>{result.draft.source_notice}</p>
      {appliedCurrent && result.applied_preference !== null ? <p data-testid="scoped-applied">本次应用了已确认的本人范围偏好：{result.applied_preference.register} · preference {result.applied_preference.preference_id} · revision {result.applied_preference.revision} · Memory {result.applied_preference.memory_id}。<a href={memoryHash(result.applied_preference.memory_id)}>查看或遗忘这条偏好</a></p>
        : <p>本次没有展示已应用的范围偏好；使用来源以当前本地结果为准。</p>}
      <label className="field" htmlFor="scoped-register">我的语域选择</label>
      <select id="scoped-register" value={register} disabled={locked} onChange={(event) => { const value = event.target.value as ScopedRegister | ""; change(() => setRegister(value)); }}>
        <option value="">主动选择，不从编辑推断</option>{Object.entries(REGISTER).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
      </select>
      <button type="button" disabled={locked || lane.busy || lane.uncertain || register === "" || proposal !== null || !appliedCurrent} onClick={propose}>保存我的表达偏好</button>
    </section>}
    {proposal === null ? null : <section className="panel" aria-label="范围偏好确认">
      <h2>确认我的范围偏好</h2><p>{proposal.notice}</p>
      <p>人物：{proposal.person_id}；场合：{proposal.occasion}；语域：{REGISTER[proposal.register]}。</p>
      <button type="button" onClick={confirm}>确认保存此范围偏好</button>
      <button type="button" onClick={() => change(() => undefined)}>取消这次偏好保存</button>
    </section>}
    {saved === null ? null : <section className="panel" data-testid="scoped-saved"><h2>已保存我的范围偏好</h2>
      <p>本次确认已提交，语域为 {REGISTER[saved.register]}。下一次本地起草会复核当时有效的设置，并展示实际应用来源；后续更改、普通 Memory 编辑或遗忘可能使这条规则停用。</p>
      {confirmationSourceChanged ? <p role="status" data-testid="scoped-confirm-source-changed">保存回执返回前，资料又发生过变更。这张回执仅证明本次提交，请重新起草核对现行规则。</p> : null}
      <p>Memory：{saved.memory_id}；revision：{saved.revision}。<a href={memoryHash(saved.memory_id)}>查看或遗忘这条偏好</a></p>
    </section>}
  </>;
}
