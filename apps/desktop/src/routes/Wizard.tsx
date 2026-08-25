/**
 * The first-run wizard. AC-02: when it finishes, collection is off, the cloud
 * is off, and there is no model endpoint.
 *
 * PRODUCT_LOCK also says it must not open with a wall of permission prompts,
 * so it has none: the only thing the user checks is that they read what the
 * defaults are. Turning something on happens later, on the page that owns it,
 * with the consequence written next to the switch.
 *
 * The list of what is off is not written here. It comes from the snapshot the
 * core returns, so a capability that quietly defaulted to on would show up on
 * this screen rather than be described as off by a hard-coded sentence.
 */

import { useState } from "react";

import { completeWizard, type ConfigSnapshot } from "../core";

export interface WizardProps {
  readonly snapshot: ConfigSnapshot;
  readonly onComplete: (snapshot: ConfigSnapshot) => void;
}

interface DefaultLine {
  readonly name: string;
  readonly state: string;
  readonly note: string;
}

function defaultLines(snapshot: ConfigSnapshot): readonly DefaultLine[] {
  return [
    {
      name: "前台应用使用时长采集",
      state: snapshot.collect_enabled ? "开" : "关",
      note: "未同意之前一条事件都不会产生。窗口标题永远不采。",
    },
    {
      name: "云端深度分析",
      state: snapshot.cloud.label,
      note: "这一版没有云端出网的代码路径，开关只是让你看见它在哪里。",
    },
    {
      name: "语言模型端点",
      state: snapshot.llm_endpoint_configured ? "已填写" : "未填写",
      note: "要用生成能力，得你自己填一个兼容 OpenAI 的地址；不填也能用档案、人脉图与记忆。",
    },
    {
      name: "已授权目录",
      state: `${snapshot.authorized_root_count} 个`,
      note: "没有你点头，Soul 不看任何目录，也不会写任何文件。",
    },
  ];
}

export function Wizard({ snapshot, onComplete }: WizardProps): React.JSX.Element {
  const [acknowledged, setAcknowledged] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);

  const finish = async (): Promise<void> => {
    try {
      onComplete(await completeWizard({ acknowledged_defaults_are_off: acknowledged }));
    } catch (error) {
      setRefusal(String(error));
    }
  };

  return (
    <main className="wizard" aria-labelledby="wizard-heading">
      <h1 id="wizard-heading">欢迎使用 Soul</h1>
      <p>
        Soul 在这台电脑上给你建一个电子版的你：人格、记忆、心理工作模型和人脉图。
        它只处理你交给它的东西，处理过程留在本机。
      </p>

      <section className="panel" aria-labelledby="defaults-heading">
        <h2 id="defaults-heading">默认是什么样子</h2>
        <table className="defaults">
          <thead>
            <tr>
              <th scope="col">能力</th>
              <th scope="col">现在</th>
              <th scope="col">说明</th>
            </tr>
          </thead>
          <tbody>
            {defaultLines(snapshot).map((line) => (
              <tr key={line.name}>
                <th scope="row">{line.name}</th>
                <td data-testid={`wizard-state-${line.name}`}>{line.state}</td>
                <td className="muted">{line.note}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="muted">
          这一页不向你要任何权限。要开哪一项，去它自己的页面开，那里会写清楚开了会发生什么。
        </p>
      </section>

      <label className="acknowledge">
        <input
          type="checkbox"
          checked={acknowledged}
          onChange={(event) => setAcknowledged(event.target.checked)}
        />
        我读过上面这几行，知道现在什么都没有打开。
      </label>

      <button
        type="button"
        className="primary"
        disabled={!acknowledged}
        onClick={() => {
          void finish();
        }}
      >
        开始使用
      </button>

      {refusal === null ? null : (
        <p className="refusal" role="alert">
          {refusal}
        </p>
      )}
    </main>
  );
}
