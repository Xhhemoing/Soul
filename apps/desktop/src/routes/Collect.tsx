/**
 * 采集. The one capability in v0.1 the user can switch on, and the page that
 * lets them.
 *
 * Until this route existed the consent ledger was unreachable from an installed
 * Soul: `soul-collect` had the gate, `soulcore::commands::collect` had the
 * pipeline, and the only way to open either was `soul-headless collect-probe`,
 * which is an instrument rather than a product surface. A Windows user had no
 * way to turn collection on, so "采集默认关" was true the way an unimplemented
 * feature is true.
 *
 * What the page may show is fixed by the type it renders. `CollectStatus` has
 * two booleans, a fixed source label, a count and two sentences — there is no
 * field on it that could hold an application name, a window title or a path, so
 * the promise PRODUCT_LOCK makes about this slice is kept by the shape of the
 * value rather than by this component's restraint.
 *
 * It deliberately does not live on 设置. That page is the cloud switch, which
 * is a notice about a capability this build does not have; collection is a
 * capability it does have, gated by a consent ledger rather than by a
 * configuration field, and putting the two side by side would suggest they are
 * the same kind of thing.
 */

import { useEffect, useState } from "react";

import {
  collectStatus,
  grantCollectConsent,
  revokeCollectConsent,
  type CollectStatus,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

/** What the source label means, for the two this build can report. */
const SOURCE: Record<string, string> = {
  "windows.foreground_process": "Windows 的前台进程（只取可执行文件名）",
  "fake.scripted_desktop": "测试用的脚本桌面",
  unsupported: "这台机器上没有前台来源",
};

function sourceReading(source: string): string {
  return SOURCE[source] ?? source;
}

export function Collect(): React.JSX.Element {
  const [status, setStatus] = useState<CollectStatus | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    collectStatus().then(
      (value) => {
        if (live) setStatus(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const ask = (change: () => Promise<CollectStatus>): void => {
    setBusy(true);
    setRefusal(null);
    change().then(
      (value) => {
        setStatus(value);
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
      <section className="panel" aria-labelledby="collect-heading">
        <h2 id="collect-heading">这一页会采集什么</h2>
        <p className="muted" data-testid="collect-duration-only">
          {status === null ? "正在读取采集状态…" : status.duration_only_notice}
        </p>
        <ul className="facts">
          <li>采的是：前台是哪个应用，以及它在前台待了多久。</li>
          <li>不采的是：窗口标题、文件内容、按键、剪贴板、鼠标位置。</li>
          <li>默认关闭。同意只在这一次运行里有效，退出 Soul 再打开还是关的。</li>
          <li>只有 Windows 上看得到前台。Linux 与开发机上可以给出同意，但没有窗口可看。</li>
        </ul>
      </section>

      {refusal === null ? null : (
        <Refused title="采集这一步没有做成" refusal={refusal} testId="collect-refusal-code" />
      )}

      {status === null ? null : <State status={status} busy={busy} onAsk={ask} />}
    </>
  );
}

interface StateProps {
  readonly status: CollectStatus;
  readonly busy: boolean;
  readonly onAsk: (change: () => Promise<CollectStatus>) => void;
}

/**
 * The current state, and the two buttons that change it.
 *
 * Both are always drawn and one of them is always disabled, so the page reads
 * the same way whichever state it is in. 开始采集 is disabled while a collector
 * is already running rather than hidden: a button that disappears makes the
 * user look for it.
 */
function State({ status, busy, onAsk }: StateProps): React.JSX.Element {
  return (
    <section className="panel" aria-labelledby="collect-state-heading">
      <h2 id="collect-state-heading">现在的状态</h2>
      <p className="badge" data-testid="collect-state">
        {status.collector_running
          ? "正在采集"
          : status.consent_granted
            ? "已经同意，但没有在采"
            : "没有在采集"}
      </p>
      <ul className="facts">
        <li data-testid="collect-consent">
          你的同意：<strong>{status.consent_granted ? "已给出" : "没有给出"}</strong>
        </li>
        <li data-testid="collect-running">
          采集线程：<strong>{status.collector_running ? "在跑" : "没有在跑"}</strong>
        </li>
        <li data-testid="collect-source">
          前台来源：{sourceReading(status.source)}（<code>{status.source}</code>）
        </li>
        <li data-testid="collect-count">
          {status.events_collected === null
            ? "库没有打开，所以数不出已经记了多少条。"
            : `库里已经有 ${status.events_collected} 条前台记录。这里只有条数，没有应用名。`}
        </li>
        <li data-testid="collect-restart">
          {status.survives_restart
            ? "这份同意会跨重启保留，请把这件事报告出来。"
            : "重启不会保留这份同意：下次打开 Soul，采集还是关的。"}
        </li>
      </ul>
      <div className="switch-row">
        <button
          type="button"
          className="primary"
          onClick={() => onAsk(grantCollectConsent)}
          disabled={busy || status.collector_running}
        >
          开始采集
        </button>
        <button
          type="button"
          onClick={() => onAsk(revokeCollectConsent)}
          disabled={busy || (!status.consent_granted && !status.collector_running)}
        >
          停止采集
        </button>
      </div>
      <p className="muted" data-testid="collect-notice">
        {status.notice}
      </p>
    </section>
  );
}
