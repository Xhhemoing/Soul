/**
 * The overview. One table of what is on, and an honest list of what this build
 * does not do yet.
 *
 * The collection line is read from `collect_status` rather than from the
 * configuration snapshot. `ConfigSnapshot.collect_enabled` is the in-memory
 * `Config` field, and that field is false for the whole life of the process:
 * `config.json` has nowhere to put collection and nothing writes it at
 * runtime. Consent lives in the ledger the collector actually reads, so the
 * ledger is what this page asks — otherwise 概览 would go on saying 关 while
 * /collect had a thread running.
 */

import { useEffect, useState } from "react";

import { collectStatus, type CollectStatus, type ConfigSnapshot, type SessionStatus } from "../core";
import { ROUTES } from "../router";

export interface HomeProps {
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
    return () => {
      live = false;
    };
  }, []);

  return (
    <>
      <section className="panel" aria-labelledby="state-heading">
        <h2 id="state-heading">这台机器上的 Soul</h2>
        <p className="badge" data-testid="closed-state">
          {snapshot.fully_closed ? "全部能力默认关闭" : `已打开：${snapshot.open_capabilities.join("、")}`}
        </p>
        <ul className="facts">
          <li data-testid="collect-fact">
            前台应用使用时长采集：<strong>{collectReading(collect)}</strong>
            <span className="muted">
              （在 <a href="#/collect">采集</a> 一页开关，重启之后回到关）
            </span>
          </li>
          <li>
            云端深度分析：<strong>{snapshot.cloud.label}</strong>
          </li>
          <li>
            语言模型端点：<strong>{snapshot.llm_endpoint_configured ? "已填写" : "未填写"}</strong>
          </li>
          <li>
            已授权目录：<strong>{snapshot.authorized_root_count} 个</strong>
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
    </>
  );
}
