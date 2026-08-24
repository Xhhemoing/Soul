/**
 * The overview. One table of what is on, and an honest list of what this build
 * does not do yet.
 */

import type { ConfigSnapshot } from "../core";
import { ROUTES } from "../router";

export interface HomeProps {
  readonly snapshot: ConfigSnapshot;
}

export function Home({ snapshot }: HomeProps): React.JSX.Element {
  const unfinished = ROUTES.filter((route) => route.ownedBy !== null);

  return (
    <>
      <section className="panel" aria-labelledby="state-heading">
        <h2 id="state-heading">这台机器上的 Soul</h2>
        <p className="badge" data-testid="closed-state">
          {snapshot.fully_closed ? "全部能力默认关闭" : `已打开：${snapshot.open_capabilities.join("、")}`}
        </p>
        <ul className="facts">
          <li>
            前台应用使用时长采集：<strong>{snapshot.collect_enabled ? "开" : "关"}</strong>
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
