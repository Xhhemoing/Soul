/**
 * Settings. In this work package that means the cloud switch, a short and
 * checkable list of what the shell itself does not do, where the database key
 * lives, and the one control in v0.1 that writes anything: authorising a
 * directory for read-only scanning.
 *
 * The three lines under 出网 are not decoration. They are the claims the build
 * makes about itself: no automatic updates, no remote page in the WebView, no
 * elevation. `apps/desktop/src-tauri` has a test for each of them.
 *
 * Nothing on this page decides anything. The sentence about key material is
 * the core's constant, rendered as it arrives; the path the user types is sent
 * over as typed, and every check on it — exists, is a directory, resolves to
 * somewhere not already on the list — happens in Rust, where a test can see it.
 */

import { useEffect, useState } from "react";

import { CloudToggle } from "../components/CloudToggle";
import { authorizeRoot, authorizedRoots, refusalText, type ConfigSnapshot } from "../core";

export interface SettingsProps {
  readonly snapshot: ConfigSnapshot;
  /**
   * The core answered with a new snapshot. The shell keeps exactly one, so
   * this hands it up rather than keeping a second copy here — a settings page
   * that remembered its own count would let the overview go on saying zero
   * after the user authorised something.
   */
  readonly onSnapshot: (snapshot: ConfigSnapshot) => void;
}

export function Settings({ snapshot, onSnapshot }: SettingsProps): React.JSX.Element {
  const [roots, setRoots] = useState<readonly string[]>([]);
  const [draft, setDraft] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    authorizedRoots().then(
      (value) => {
        if (live) setRoots(value);
      },
      (error: unknown) => {
        if (live) setRefusal(refusalText(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const authorize = async (): Promise<void> => {
    try {
      onSnapshot(await authorizeRoot(draft));
      setRoots(await authorizedRoots());
      setRefusal(null);
      setDraft("");
    } catch (error) {
      setRefusal(refusalText(error));
    }
  };

  return (
    <>
      <CloudToggle notice={snapshot.cloud} />

      <section className="panel" aria-labelledby="roots-heading">
        <h2 id="roots-heading">授权目录</h2>
        <p className="badge" data-testid="authorized-root-count">
          已授权 {snapshot.authorized_root_count} 个目录
        </p>
        <p className="muted">
          Soul 只看你在这里点头的目录，而且只读不写：这一版没有任何写文件的代码路径。
        </p>
        <div className="field-row">
          <label htmlFor="root-path">目录的完整路径</label>
          <input
            id="root-path"
            type="text"
            value={draft}
            placeholder="C:\Users\你\Documents"
            onChange={(event) => setDraft(event.target.value)}
          />
          <button
            type="button"
            className="primary"
            onClick={() => {
              void authorize();
            }}
          >
            授权这个目录
          </button>
        </div>
        {refusal === null ? null : (
          <p className="refusal" role="alert">
            {refusal}
          </p>
        )}
        <ul className="facts" data-testid="authorized-roots">
          {roots.length === 0 ? (
            <li className="muted">还没有授权任何目录。</li>
          ) : (
            roots.map((root) => <li key={root}>{root}</li>)
          )}
        </ul>
        <p className="muted" data-testid="roots-are-session-only">
          本次会话有效，尚无持久化：关掉 Soul 之后这份清单就没了，下次启动要重新授权。
          把它记到哪个文件里是 WP13 的事，这一版不假装已经做了。
        </p>
      </section>

      <section className="panel" aria-labelledby="keys-heading">
        <h2 id="keys-heading">数据库密钥</h2>
        <p data-testid="key-protection">{snapshot.key_protection}</p>
      </section>

      <section className="panel" aria-labelledby="egress-heading">
        <h2 id="egress-heading">这个壳会不会自己上网</h2>
        <ul className="facts">
          <li>不自动更新：这一版没有打包更新器，也没有检查更新的入口。</li>
          <li>界面全部来自安装目录里的文件，不加载任何远程页面。</li>
          <li>以普通用户身份运行，日常使用不要求管理员权限。</li>
          <li>
            只有你自己填写的语言模型地址会被访问，而且只在你按下生成的时候。现在
            <strong>{snapshot.llm_endpoint_configured ? "已填写" : "未填写"}</strong>。
          </li>
        </ul>
      </section>
    </>
  );
}
