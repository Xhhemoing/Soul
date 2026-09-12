/**
 * 设置. The cloud switch, a short checkable list of what the shell does not do,
 * and the one address the user can give Soul.
 *
 * The three lines under 出网 are not decoration. They are the claims the build
 * makes about itself: no automatic updates, no remote page in the WebView, no
 * elevation. `apps/desktop/src-tauri` has a test for each of them.
 *
 * The endpoint form is the other half of the same subject, and until it existed
 * the address was unreachable from an installed Soul: `soul-policy` had the
 * guard, `soulcore` had `PolicySession::with_user_endpoint`, and this page could
 * only report 未填写 forever. It is the same gap 采集 had before `/collect`.
 *
 * What the page may show is fixed by the value it renders. `ConfigSnapshot`
 * carries a boolean and a sentence, never the address — so "the shell cannot
 * show you back what you typed after a reload" is a property of the type rather
 * than of this component's restraint. The address goes out through
 * `setUserEndpoint` and does not come back.
 *
 * Collection deliberately does not live here. It is a capability gated by a
 * consent ledger; the cloud switch is a notice about a capability this build
 * does not have, and the endpoint is an address the user owns. Putting all
 * three side by side would suggest they are the same kind of thing.
 */

import { useState } from "react";

import { CloudToggle } from "../components/CloudToggle";
import {
  clearUserEndpoint,
  setUserEndpoint,
  type ConfigSnapshot,
  type Refusal,
} from "../core";
import { asRefusal, Refused } from "../refusal";

export interface SettingsProps {
  readonly snapshot: ConfigSnapshot;
  /**
   * The snapshot the core answered with, handed back up.
   *
   * 概览 renders 语言模型端点 off the same value, and it is held by `App`. Without
   * this the line there would go on saying 未填写 until the window was reloaded,
   * which is the shell disagreeing with the core about what the user just did.
   */
  readonly onSnapshot: (snapshot: ConfigSnapshot) => void;
}

export function Settings({ snapshot, onSnapshot }: SettingsProps): React.JSX.Element {
  const [url, setUrl] = useState("");
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

  const ask = (change: () => Promise<ConfigSnapshot>): void => {
    setBusy(true);
    setRefusal(null);
    change().then(
      (value) => {
        onSnapshot(value);
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
      <CloudToggle notice={snapshot.cloud} />

      <section className="panel" aria-labelledby="endpoint-heading">
        <h2 id="endpoint-heading">你自己的语言模型端点</h2>
        <p className="badge" data-testid="endpoint-state">
          {snapshot.llm_endpoint_configured ? "已填写" : "未填写"}
        </p>
        <p className="muted" data-testid="endpoint-notice">
          {snapshot.llm_endpoint_notice}
        </p>
        <ul className="facts">
          <li>
            只取地址里的协议、主机和端口：生成的时候请求发到同一个来源下的{" "}
            <code>/v1/chat/completions</code>，别的来源一律拒绝。
          </li>
          <li>地址不写进任何文件，核心也只回答填没填，不会把它交回给这个界面。</li>
          <li>不填也能用：起草会走本机的确定性语气模板，什么都不出网。</li>
        </ul>
        <label className="field" htmlFor="endpoint-url">
          端点地址
        </label>
        <div className="switch-row">
          <input
            id="endpoint-url"
            className="text-input"
            type="text"
            value={url}
            onChange={(event) => setUrl(event.target.value)}
            placeholder="例如 http://127.0.0.1:11434/v1"
            disabled={busy}
          />
          <button
            type="button"
            className="primary"
            onClick={() => ask(() => setUserEndpoint(url.trim()))}
            disabled={busy || url.trim() === ""}
          >
            保存端点
          </button>
          <button
            type="button"
            onClick={() => {
              setUrl("");
              ask(clearUserEndpoint);
            }}
            disabled={busy || !snapshot.llm_endpoint_configured}
          >
            清除端点
          </button>
        </div>
      </section>

      {refusal === null ? null : (
        <Refused title="端点这一步没有做成" refusal={refusal} testId="endpoint-refusal-code" />
      )}

      <section className="panel" aria-labelledby="egress-heading">
        <h2 id="egress-heading">这个壳会不会自己上网</h2>
        <ul className="facts">
          <li>不自动更新：这一版没有打包更新器，也没有检查更新的入口。</li>
          <li>界面全部来自安装目录里的文件，不加载任何远程页面。</li>
          <li>以普通用户身份运行，日常使用不要求管理员权限。</li>
          <li data-testid="egress-endpoint-when">
            只有你自己填写的语言模型地址会被访问，而且只在你确认生成草稿、或在人脉图上看某个人的摘要时。现在
            <strong>{snapshot.llm_endpoint_configured ? "已填写" : "未填写"}</strong>。
          </li>
        </ul>
      </section>
    </>
  );
}
