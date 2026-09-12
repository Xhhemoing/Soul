/**
 * The overview. One table of what is on, and nothing said about what is not.
 *
 * Both halves of this page are read again on mount rather than taken on trust
 * from the shell, because the shell asks the core once at launch and every
 * value here can change after that.
 *
 * The collection line comes from `collect_status`, which knows three states
 * where the snapshot knows two: consented and collecting, consented with
 * nothing on this machine to watch, and off. The line says which, in the same
 * words `/collect` uses, so the two pages cannot disagree.
 *
 * The badge comes from `config_snapshot`. `ConfigSnapshot.collect_enabled` is
 * the in-memory `Config` field, and `Session::grant_collection` now writes it
 * when the ledger records a grant — the same thing `set_user_endpoint` does
 * with the address. It reaches no file: `StoredConfig` has two fields and
 * neither is this one, so a restart finds the capability closed because there
 * was nowhere to write it down. Until that grant wrote the field, 概览 could
 * say 全部能力默认关闭 with a collector running one page over.
 */

import { useEffect, useState } from "react";

import {
  collectStatus,
  configSnapshot,
  type CollectStatus,
  type ConfigSnapshot,
  type SessionStatus,
} from "../core";
import { ROUTES } from "../router";

export interface HomeProps {
  /** What the shell read at launch, shown until this page's own read lands. */
  readonly snapshot: ConfigSnapshot;
  readonly status: SessionStatus;
}

function collectReading(collect: CollectStatus | null): string {
  if (collect === null) return "读取中…";
  if (collect.collector_running) return "正在采集";
  // Same three phrases as `/collect`, so the overview line and that page
  // cannot disagree about what the ledger is doing.
  return collect.consent_granted ? "已经同意，但没有在采" : "没有在采集";
}

export function Home({ snapshot, status }: HomeProps): React.JSX.Element {
  const unfinished = ROUTES.filter((route) => route.ownedBy !== null);
  const [collect, setCollect] = useState<CollectStatus | null>(null);
  /**
   * The shell fetched a snapshot when it started and hands it down as a prop,
   * but only 设置 gives it a newer one: authorizing a directory on /files and
   * consenting on /collect both change what belongs on this page and neither
   * tells the shell. This route unmounts when you leave it, so reading again
   * on mount is enough to stop the counts and the badge being launch-time
   * facts.
   */
  const [fetched, setFetched] = useState<ConfigSnapshot | null>(null);
  const shown = fetched ?? snapshot;

  useEffect(() => {
    let live = true;
    collectStatus().then(
      (value) => {
        if (live) setCollect(value);
      },
      () => {
        // A machine whose store did not open still has an overview to show.
        // The line stays at 读取中… rather than claiming collection is off,
        // which is a claim only the ledger can make.
      },
    );
    configSnapshot().then(
      (value) => {
        if (live) setFetched(value);
      },
      () => {
        // The prop stays on screen. It came from the same command one launch
        // earlier, which is stale rather than invented.
      },
    );
    return () => {
      live = false;
    };
  }, []);

  return (
    <>
      <section className="panel" aria-labelledby="state-heading">
        <h2 id="state-heading">这台机器上的 Soul</h2>
        <p className="badge" data-testid="closed-state">
          {shown.fully_closed ? "全部能力默认关闭" : `已打开：${shown.open_capabilities.join("、")}`}
        </p>
        <ul className="facts">
          <li data-testid="collect-fact">
            前台应用使用时长采集：<strong>{collectReading(collect)}</strong>
            <span className="muted">
              （在 <a href="#/collect">采集</a> 一页开关，重启之后回到关）
            </span>
          </li>
          <li>
            云端深度分析：<strong>{shown.cloud.label}</strong>
          </li>
          <li>
            语言模型端点：<strong>{shown.llm_endpoint_configured ? "已填写" : "未填写"}</strong>
          </li>
          <li>
            已授权目录：<strong>{shown.authorized_root_count} 个</strong>
          </li>
        </ul>
      </section>

      <section className="panel" aria-labelledby="store-heading">
        <h2 id="store-heading">本机的加密库</h2>
        <p className="badge" data-testid="store-state">
          {status.store_opened ? "已打开" : "没有打开"}
        </p>
        <p className="muted" data-testid="store-notice">
          {status.store_notice}
        </p>
        {status.config_problem === null ? null : (
          <p className="muted" data-testid="config-problem">
            {status.config_problem}
          </p>
        )}
      </section>

      {/*
        Every route has a view now, so this list is empty and the section is
        not drawn. A heading over an empty list reads as 「什么都不缺」, which
        is a claim this page has no way to check; the honest version of an
        empty list is no list. The section stays for the next route that is
        added before its view exists — that one should say whose it is rather
        than looking like a finished feature.
      */}
      {unfinished.length === 0 ? null : (
        <section className="panel" aria-labelledby="unfinished-heading">
          <h2 id="unfinished-heading">这一版还没有的东西</h2>
          <ul className="facts">
            {unfinished.map((route) => (
              <li key={route.id}>
                <a href={`#${route.path}`}>{route.title}</a>
                <span className="muted">（{route.ownedBy}）</span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  );
}
