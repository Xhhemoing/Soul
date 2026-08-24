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

import { useEffect, useState } from "react";

import {
  authorizeDirectory,
  filesView,
  previewPlan,
  type FilesView,
  type PlanPreview,
  type Refusal,
} from "../core";

/** A refusal that did not arrive as one — the core is not answering at all. */
function asRefusal(error: unknown): Refusal {
  const shaped = error as Partial<Refusal> | null;
  return typeof shaped?.reason_code === "string" && typeof shaped.explanation === "string"
    ? { reason_code: shaped.reason_code, explanation: shaped.explanation }
    : { reason_code: "unavailable", explanation: String(error) };
}

export function Files(): React.JSX.Element {
  const [view, setView] = useState<FilesView | null>(null);
  const [path, setPath] = useState("");
  const [plan, setPlan] = useState<PlanPreview | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    filesView().then(
      (value) => {
        if (live) setView(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const authorize = (): void => {
    setBusy(true);
    setRefusal(null);
    authorizeDirectory(path).then(
      (value) => {
        setView(value);
        setPath("");
        setBusy(false);
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  const scan = (root: string): void => {
    setBusy(true);
    setRefusal(null);
    setPlan(null);
    previewPlan(root).then(
      (value) => {
        setPlan(value);
        setBusy(false);
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

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
        <div className="switch-row">
          <input
            id="root-path"
            className="text-input"
            type="text"
            value={path}
            onChange={(event) => setPath(event.target.value)}
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
        <section className="panel refusal" role="alert" aria-labelledby="files-refused-heading">
          <h2 id="files-refused-heading">这一次没有读成</h2>
          <p data-testid="files-refusal-code">{refusal.reason_code}</p>
          <p>{refusal.explanation}</p>
        </section>
      )}

      <section className="panel" aria-labelledby="roots-heading">
        <h2 id="roots-heading">已授权的目录</h2>
        {view === null || view.roots.length === 0 ? (
          <p className="muted" data-testid="no-roots">
            还没有授权任何目录。授权之前，Soul 读不到你机器上的任何文件。
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
