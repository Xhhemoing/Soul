/**
 * Settings. In this work package that means the cloud switch plus a short,
 * checkable list of what the shell itself does not do.
 *
 * The three lines under 出网 are not decoration. They are the claims the build
 * makes about itself: no automatic updates, no remote page in the WebView, no
 * elevation. `apps/desktop/src-tauri` has a test for each of them.
 */

import { CloudToggle } from "../components/CloudToggle";
import type { ConfigSnapshot } from "../core";

export interface SettingsProps {
  readonly snapshot: ConfigSnapshot;
}

export function Settings({ snapshot }: SettingsProps): React.JSX.Element {
  return (
    <>
      <CloudToggle notice={snapshot.cloud} />

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
