/**
 * 文件计划. Name a directory, read what tidying it would mean, and stop there.
 *
 * The absence is the feature, and it is worth saying where it comes from. This
 * screen cannot move a file because there is no command to move one with:
 * `core.ts` names every command the shell has, `soulcore`'s file-plan surface
 * has no `execute`, and `soul-fileplan` has no write API at all — its
 * `tests/no_write_api.rs` reads its own source back to prove it. Hiding a
 * button would be a promise; having nothing to bind one to is a fact.
 *
 * PRODUCT_LOCK puts the write half in v0.1.1 (AC-27). The sentence saying so
 * comes from the core rather than from this file, so it cannot be softened
 * here.
 */

import { useEffect, useRef, useState } from "react";

import {
  authorizeDirectory,
  filesView,
  FilePlanSourceChangedError,
  previewPlan,
  subscribeDraftSource,
  type FilesView,
  type PlanPreview,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

export function Files(): React.JSX.Element {
  const [view, setView] = useState<FilesView | null>(null);
  const [path, setPath] = useState("");
  const [plan, setPlan] = useState<PlanPreview | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false);
  const live = useRef(false);
  const active = useRef<symbol | null>(null);
  const inputRevision = useRef(0);
  const sourceRevision = useRef(0);
  const initialRead = useRef<symbol | null>(null);

  useEffect(() => {
    live.current = true;
    const read = Symbol("initial roots");
    initialRead.current = read;
    const unsubscribe = subscribeDraftSource(() => {
      sourceRevision.current += 1;
      setPlan(null);
    });
    filesView().then(
      (value) => {
        if (live.current && initialRead.current === read) setView(value);
      },
      (error: unknown) => {
        if (live.current && initialRead.current === read) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live.current = false;
      active.current = null;
      if (initialRead.current === read) initialRead.current = null;
      unsubscribe();
    };
  }, []);

  const authorize = (): void => {
    if (active.current !== null) return;
    const ticket = Symbol("authorize");
    active.current = ticket;
    initialRead.current = null;
    const revision = inputRevision.current;
    setBusy(true);
    setRefusal(null);
    setChanged(false);
    authorizeDirectory(path).then(
      (value) => {
        if (!settle(ticket)) return;
        setView(value);
        if (revision === inputRevision.current) setPath("");
      },
      (error: unknown) => {
        if (settle(ticket) && revision === inputRevision.current) setRefusal(asRefusal(error));
      },
    );
  };

  const scan = (root: string): void => {
    if (active.current !== null) return;
    const ticket = Symbol("scan");
    active.current = ticket;
    const revision = inputRevision.current;
    const source = sourceRevision.current;
    setBusy(true);
    setRefusal(null);
    setPlan(null);
    setChanged(false);
    Promise.resolve().then(() => {
      if (!live.current || active.current !== ticket) throw new Error("inactive scan");
      return previewPlan(root);
    }).then(
      (value) => {
        if (settle(ticket) && revision === inputRevision.current && source === sourceRevision.current) setPlan(value);
      },
      (error: unknown) => {
        if (settle(ticket) && revision === inputRevision.current) {
          if (error instanceof FilePlanSourceChangedError) setChanged(true);
          else setRefusal(asRefusal(error));
        }
      },
    );
  };

  function settle(ticket: symbol): boolean {
    if (!live.current || active.current !== ticket) return false;
    active.current = null;
    setBusy(false);
    return true;
  }

  return (
    <>
      <section className="panel" aria-labelledby="authorize-heading">
        <h2 id="authorize-heading">你允许 Soul 读哪个目录</h2>
        <p className="muted" data-testid="read-only-notice">
          {view?.read_only_notice ?? "这一版只做只读扫描与计划预览。"}
        </p>
        <label className="field" htmlFor="root-path">
          目录完整路径
        </label>
        <p className="muted" data-testid="paste-path-hint">
          从资源管理器「复制为路径」贴进来也可以，外面那对引号会去掉。
        </p>
        <div className="switch-row">
          <input
            id="root-path"
            className="text-input"
            type="text"
            value={path}
            onChange={(event) => {
              inputRevision.current += 1;
              setPath(event.target.value);
              setPlan(null);
              setRefusal(null);
              setChanged(false);
            }}
            placeholder="例如 C:\Users\你\Downloads"
          />
          <button
            type="button"
            className="primary"
            onClick={authorize}
            disabled={path.trim() === "" || busy}
          >
            授权这个目录
          </button>
        </div>
      </section>

      {refusal === null ? null : (
        <Refused title="这一次没有读成" refusal={refusal} testId="files-refusal-code" />
      )}
      {changed ? <p role="status">计划依据或授权已经变更，请重新扫描。</p> : null}

      <section className="panel" aria-labelledby="roots-heading">
        <h2 id="roots-heading">已授权的目录</h2>
        {view === null || view.roots.length === 0 ? (
          <p className="muted" data-testid="no-roots">
            还没有授权任何目录。没授权之前，这一页不会去扫你机器上的任何目录。
          </p>
        ) : (
          <ul className="facts">
            {view.roots.map((root) => (
              <li key={root.path}>
                <code>{root.path}</code>
                <button type="button" onClick={() => scan(root.path)} disabled={busy}>
                  看整理计划
                </button>
              </li>
            ))}
          </ul>
        )}
        {view === null || view.unavailable_roots.length === 0 ? null : (
          <ul className="facts" data-testid="unavailable-roots">
            {view.unavailable_roots.map((root) => (
              <li key={root.path}>
                <code>{root.path}</code>
                <span className="muted">{root.explanation}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      {plan === null ? null : <Plan plan={plan} />}
    </>
  );
}

interface PlanProps {
  readonly plan: PlanPreview;
}

/**
 * The plan, as a list of two halves: what a tidy-up would change, and what it
 * would leave where it is, each with the core's own reason.
 *
 * There is no control on this component. The counts and the hash are here
 * because they are what a person would need in order to approve something —
 * and the point of v0.1 is that there is nothing to approve yet.
 */
function Plan({ plan }: PlanProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="plan-heading">
      <h2 id="plan-heading">整理计划</h2>
      <ul className="facts">
        <li>
          目录：<code>{plan.root}</code>
        </li>
        <li>
          扫过 {plan.scanned_entries} 项，跳过 {plan.skipped_entries} 项
          {plan.truncated ? "（触到了扫描上限，没有看完）" : ""}。
        </li>
        <li data-testid="plan-hash">
          计划哈希：<code>{plan.plan_hash}</code>
        </li>
        <li data-testid="disk-unchanged">
          {plan.disk_unchanged ? "扫描前后目录逐项一致：这次读取没有改动任何东西。" : "目录在扫描期间变过了，这份计划已经不描述现在的目录。"}
        </li>
      </ul>

      <h3>会归类的文件（{plan.moves.length}）</h3>
      {plan.moves.length === 0 ? (
        <p className="muted">这个目录里没有可以按类型归位的文件。</p>
      ) : (
        <ul className="facts" data-testid="plan-moves">
          {plan.moves.map((move) => (
            <li key={move.from}>
              <code>{move.from}</code> → <code>{move.to}</code>
              <span className="muted">（{move.kind_label}）</span>
            </li>
          ))}
        </ul>
      )}

      <h3>原地不动的（{plan.left_alone.length}）</h3>
      {plan.left_alone.length === 0 ? (
        <p className="muted">没有被跳过的条目。</p>
      ) : (
        <ul className="facts" data-testid="plan-left-alone">
          {plan.left_alone.map((left) => (
            <li key={left.path}>
              <code>{left.path}</code>
              <span className="muted">{left.explanation}</span>
            </li>
          ))}
        </ul>
      )}

      <p className="badge" data-testid="plan-read-only">
        {plan.read_only_notice}
      </p>
    </section>
  );
}
