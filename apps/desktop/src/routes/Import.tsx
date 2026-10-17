/** Local immutable import preparations, exact confirmation, and durable recovery. */
import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import {
  confirmImport, discardImport, importOperation, listImportNamespaces, prepareImport, recentImportOperations,
  importRecoveryState,
  MAX_IMPORT_BYTES, IMPORT_OVER_BYTE_BUDGET_NOTICE,
  type ImportNamespace, type ImportOperationView, type ImportSource, type PreparedImport, type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";
import { DurableImport, durableImport } from "./durableImport";
import { LocalInsights } from "./LocalInsights";

export function Import({ lane = durableImport }: { readonly lane?: DurableImport } = {}): React.JSX.Element {
  const state = useSyncExternalStore(lane.subscribe, lane.snapshot);
  const [format, setFormat] = useState<ImportSource>("soul-import-v1");
  const [namespace, setNamespace] = useState("");
  const [forward, setForward] = useState(false);
  const [namespaces, setNamespaces] = useState<readonly ImportNamespace[]>([]);
  const [prepared, setPrepared] = useState<PreparedImport | null>(null);
  const [rows, setRows] = useState<readonly ImportOperationView[]>([]);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [lookup, setLookup] = useState(state.operationId ?? "");
  const [checking, setChecking] = useState(false);
  const [namespaceRefresh, setNamespaceRefresh] = useState(0);
  const [namespaceError, setNamespaceError] = useState<string | null>(null);
  const namespaceRevision = useRef(0);
  const owner = useRef<symbol | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const queryOwner = useRef<symbol | null>(null);

  useEffect(() => {
    const mount = Symbol("import route"); owner.current = mount;
    return () => { if (owner.current === mount) owner.current = null; queryOwner.current = null; };
  }, []);

  useEffect(() => {
    const mount = owner.current; const receipt = state.receipt;
    let active = true;
    const current = (): boolean => active && mount !== null && owner.current === mount
      && namespaceRevision.current === namespaceRefresh && lane.snapshot().receipt === receipt;
    void listImportNamespaces().then((value) => {
      if (current()) { setNamespaces(value); setNamespaceError(null); }
    }, () => {
      if (current()) setNamespaceError("来源列表暂时无法更新，请点击“核对最近记录”重试。");
    });
    return () => { active = false; };
  }, [lane, state.receipt, namespaceRefresh]);

  const reset = (): void => {
    setPrepared(null); setRefusal(null); if (input.current !== null) input.current.value = "";
  };
  const read = async (file?: File): Promise<void> => {
    if (file === undefined || state.busy || state.unknown) return;
    const ticket = owner.current; reset();
    try {
      await lane.run(async () => {
        if (file.size > MAX_IMPORT_BYTES) throw { reason_code: "ROUTINE", explanation: IMPORT_OVER_BYTE_BUDGET_NOTICE };
        const text = await file.text();
        if (ticket === null || ticket !== owner.current) return;
        const result = await prepareImport({ text, source: format, namespace_id: namespace || null, legacy_choice: forward ? "forward_only" : "reject" });
        lane.remember(result.operation.operation_id);
        if (owner.current === ticket) { setPrepared(result); setLookup(result.operation.operation_id); }
      }, null);
    } catch (error: unknown) { if (owner.current === ticket) setRefusal(asRefusal(error)); }
    finally { if (input.current !== null && owner.current === ticket) input.current.value = ""; }
  };
  const confirm = async (): Promise<void> => {
    if (prepared === null) return;
    const ticket = owner.current; const approved = prepared; setRefusal(null);
    try {
      await lane.run(async () => {
        lane.remember(approved.operation.operation_id);
        const result = await confirmImport(approved.operation.operation_id, approved.operation.preview_id);
        lane.observed(result);
      });
      if (ticket === owner.current) setPrepared(null);
    } catch (error: unknown) {
      if (ticket === owner.current) { setPrepared(null); setRefusal(asRefusal(error)); }
    }
  };
  const discard = async (): Promise<void> => {
    if (prepared === null) return;
    const id = prepared.operation.operation_id; const ticket = owner.current;
    try { await lane.run(() => discardImport(id)); if (owner.current === ticket) reset(); }
    catch (error: unknown) { if (owner.current === ticket) setRefusal(asRefusal(error)); }
  };
  const discardKnown = async (id: string): Promise<void> => {
    const mount = owner.current;
    try {
      const result = await lane.run(() => discardImport(id), id, true);
      lane.observed(result);
      if (mount !== owner.current) return;
      setRows((previous) => previous.map((row) => row.operation.operation_id === id ? result : row));
      setPrepared(null);
    } catch (error: unknown) { if (mount === owner.current) setRefusal(asRefusal(error)); }
  };
  const check = async (id?: string): Promise<void> => {
    if (state.busy || queryOwner.current !== null) return;
    const query = Symbol("receipt lookup"); const mount = owner.current;
    const generation = lane.generation();
    queryOwner.current = query; setChecking(true); setRefusal(null);
    try {
      const result = id === undefined ? await recentImportOperations() : [await importOperation(id)];
      if (queryOwner.current !== query || owner.current !== mount || generation !== lane.generation()) return;
      lane.checked(result, generation);
      setRows(result);
      if (id === undefined) {
        const recovery = await importRecoveryState();
        if (queryOwner.current !== query || owner.current !== mount || generation !== lane.generation()) return;
        lane.recovered(recovery, generation);
      }
      for (const row of result) if (row.operation.state === "committed") { lane.observed(row, generation); break; }
      if (queryOwner.current === query && owner.current === mount && generation === lane.generation()) {
        namespaceRevision.current += 1;
        setNamespaceRefresh(namespaceRevision.current);
      }
    } catch (error: unknown) { if (owner.current === mount) setRefusal(asRefusal(error)); }
    finally { if (queryOwner.current === query) { queryOwner.current = null; setChecking(false); } }
  };
  const blocked = state.busy || state.unknown || prepared !== null || checking;

  return <>
    <LocalInsights key={state.receipt?.operation.operation_id ?? "initial"} />
    <section className="panel">
      <h2>选择明确来源并准备导入</h2>
      <p>只读取你选择的 JSONL 或 Telegram Desktop result.json，不打开归档、不登录账号、不自动出网。
        原文只在本次准备中保留，重启不恢复原文或批准。确认只提交操作编号和预览编号。</p>
      <label className="field" htmlFor="import-format">导入格式</label>
      <select id="import-format" value={format} disabled={blocked} onChange={(event) => { reset(); setNamespace(""); setFormat(event.target.value as ImportSource); }}>
        <option value="soul-import-v1">soul-import-v1 JSONL</option><option value="telegram-desktop">Telegram Desktop result.json</option>
      </select>
      <label className="field" htmlFor="import-namespace">来源身份</label>
      <select id="import-namespace" value={namespace} disabled={blocked} onChange={(event) => { reset(); setNamespace(event.target.value); }}>
        <option value="">{format === "telegram-desktop" ? "按导出中的稳定本人账号匹配；首次建立来源" : "明确建立一个新 JSONL 来源"}</option>
        {namespaces.filter((row) => row.source === format).map((row) => <option key={row.namespace_id} value={row.namespace_id} disabled={row.state === "revoked"}>{row.namespace_id}（{row.state === "revoked" ? "已遗忘，禁止重导" : row.coverage === "forward_only" ? "本次及以后索引" : "自此来源建立起已索引"}）</option>)}
      </select>
      {namespaceError === null ? null : <p role="status">{namespaceError}</p>}
      <p>同一 JSONL 数据集请选择同一个来源编号。文件名、导出时间和正文不会用来猜测身份。新来源不会补回历史消息编号。</p>
      <label><input type="checkbox" checked={forward} disabled={blocked} onChange={(event) => { reset(); setForward(event.target.checked); }} />
        我明确接受：仅本次及以后建立消息索引；旧库没有编号，历史可能重叠。这不授权合并不同来源中的第三人。</label>
      <label className="field" htmlFor="import-file">选择导出文件</label>
      <input id="import-file" ref={input} type="file" accept={format === "telegram-desktop" ? ".json" : ".jsonl,.json,.txt"} disabled={blocked} onChange={(event) => { void read(event.target.files?.[0]); }} />
      {state.busy ? <p role="status">本次操作仍在进行；切页不会取消物理工作，请等待或稍后查询。</p> : null}
    </section>
    {prepared === null ? null : <section className="panel">
      <h2>确认这份不可变导入准备</h2>
      <p>{prepared.notice}</p><p>操作：<code>{prepared.operation.operation_id}</code>；来源：<code>{prepared.operation.namespace_id}</code></p>
      <p>文件包含 {prepared.preview.messages} 条消息；预计新增 {prepared.operation.preview.events_written} 条，跳过相同消息 {prepared.operation.preview.duplicates} 条。</p>
      {prepared.legacy_events > 0 ? <p>存在 {prepared.legacy_events} 条未索引历史；本次及以后建立索引，历史可能重叠。</p> : null}
      <button type="button" disabled={state.busy || state.unknown || !prepared.preview.owner_identified} onClick={() => { void confirm(); }}>确认导入这份准备</button>
      <button type="button" disabled={state.busy || state.unknown} onClick={() => { void discard(); }}>丢弃这份准备</button>
    </section>}
    {refusal === null ? null : <Refused title="这次导入需要核对" refusal={refusal} testId="import-refusal-code" />}
    <section className="panel">
      <h2>查询持久操作回执</h2>
      <p>未找到回执、Reserved 或页面重启都不证明操作没有发生。不会自动重发；Running 未确认时请完整退出并重新打开应用，再核对 Interrupted 或 Committed。</p>
      {state.unknown ? <p role="alert">上次结果未知，新的导入暂停。先核对最近记录，之后再由你选择一个新意图；不自动恢复旧批准。</p> : null}
      <label htmlFor="import-operation">操作编号</label><input id="import-operation" value={lookup} onChange={(event) => setLookup(event.target.value)} />
      <button type="button" disabled={state.busy || checking || lookup.trim() === ""} onClick={() => { void check(lookup.trim()); }}>查询这个编号</button>
      <button type="button" disabled={state.busy || checking} onClick={() => { void check(); }}>核对最近记录</button>
      {state.receipt === null ? null : <Operation value={state.receipt} />}
      {rows.filter((row) => row.operation.operation_id !== state.receipt?.operation.operation_id).map((row) => <div key={row.operation.operation_id}>
        <Operation value={row} />
        {row.prepared_here && row.operation.state === "reserved" ? <button type="button" disabled={state.busy} onClick={() => { void discardKnown(row.operation.operation_id); }}>丢弃操作 {row.operation.operation_id} 的旧准备</button> : null}
      </div>)}
    </section>
  </>;
}

function Operation({ value }: { readonly value: ImportOperationView }): React.JSX.Element {
  const { operation } = value; const receipt = operation.receipt;
  return <article><h3>操作 {operation.operation_id}</h3><p>状态：{operation.state}；审计：{operation.audit_state}</p>
    {operation.legacy_choice === "forward_only" ? <p>本次及以后建立索引；旧库历史可能重叠，这份回执不代表全历史去重。</p> : null}
    {receipt === null ? <p>没有成功提交回执，不能据此重放原操作。</p> : <p>已提交：新增事件 {receipt.events_written} 条、证据 {receipt.evidence_written} 条、重复跳过 {receipt.duplicates} 条；重算写回关系 {receipt.ties_rebuilt} 条（包括已有关系，不是新增关系数）。</p>}
    {value.namespace_revoked ? <p>来源已遗忘；上述数字仅为历史回执，不恢复事实或密钥，禁止重导。</p> : null}
    {operation.error_code === null ? null : <p>{operation.error_code}</p>}
    <a href="#/graph">查看图谱、证据与纠正</a>{" · "}<a href="#/audit">核对本机审计</a>
  </article>;
}
