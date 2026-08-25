/**
 * The first-run wizard. AC-02: when it finishes, collection is off, the cloud
 * is off, and there is no model endpoint. AC-03: when it finishes, there is a
 * profile, and everything in it is something the user said.
 *
 * PRODUCT_LOCK also says it must not open with a wall of permission prompts,
 * so it has none: the only thing the user checks on the first page is that
 * they read what the defaults are. Turning something on happens later, on the
 * page that owns it, with the consequence written next to the switch.
 *
 * ## The eleven questions
 *
 * They come second, after the defaults have been acknowledged, because the
 * first thing a user should learn about Soul is what it is not doing. They are
 * the only path to a profile for someone who imports nothing, which is what
 * AC-03 is about — and they are read from the core rather than written here.
 * `soul_import::questionnaire::QUESTIONS` is the canonical list and
 * `profile::questions()` pairs each entry with the option tokens the recorder
 * will accept and the words for them, so this file decides layout and nothing
 * else. A question nobody added here would be a question the wizard does not
 * draw; a question added here would not be recordable at all. Drawing one is
 * `../questions`, shared with 灵魂档案, which asks the same eleven again
 * afterwards.
 *
 * Skipping is a real answer. A blank leaves no row, and the axis it would have
 * moved stays 还看不出方向 rather than being filled in from the answers that
 * were given — so the two buttons at the bottom are "hand in what I answered"
 * and "hand in nothing", and neither of them guesses.
 *
 * The list of what is off is not written here either. It comes from the
 * snapshot the core returns, so a capability that quietly defaulted to on
 * would show up on this screen rather than be described as off by a hard-coded
 * sentence.
 */

import { useEffect, useState } from "react";

import {
  answerQuestionnaire,
  completeWizard,
  questionnaire,
  type ConfigSnapshot,
  type IntakeReceipt,
  type Question,
  type Refusal,
} from "../core";
import { Ask } from "../questions";
import { asRefusal } from "../refusal";

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
      // Two independent sentences on purpose: authorizing a directory is what
      // lets Soul read it, and it is not what lets Soul write. This version has
      // no write path at all, so a note that reads as "点头之后就能写" would be
      // false the moment the user authorizes anything.
      name: "已授权目录",
      state: `${snapshot.authorized_root_count} 个`,
      note: "没有你点头，Soul 不看任何目录。这一版即使授权了也只做只读扫描与计划预览，不会写任何文件。",
    },
  ];
}

export function Wizard({ snapshot, onComplete }: WizardProps): React.JSX.Element {
  const [acknowledged, setAcknowledged] = useState(false);
  /** False until the defaults have been read; the questions come after. */
  const [asking, setAsking] = useState(false);
  const [questions, setQuestions] = useState<readonly Question[]>([]);
  const [given, setGiven] = useState<Record<string, string>>({});
  const [receipt, setReceipt] = useState<IntakeReceipt | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    questionnaire().then(
      (value) => {
        if (live) setQuestions(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  const answered = questions.filter(
    (question) => (given[question.question_id] ?? "").trim() !== "",
  ).length;

  const finish = (): void => {
    setBusy(true);
    completeWizard({ acknowledged_defaults_are_off: acknowledged }).then(
      (completed) => onComplete(completed),
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  /**
   * Hand the answers in, then finish. In that order and not the other way
   * round: a questionnaire the core refused must leave the user on this page
   * with the refusal in front of them, rather than in a shell whose profile is
   * empty for a reason nobody read.
   */
  const recordThenFinish = (): void => {
    setBusy(true);
    setRefusal(null);
    const answers = questions.map((question) => ({
      question_id: question.question_id,
      given: given[question.question_id] ?? "",
    }));
    answerQuestionnaire(answers).then(
      (written) => {
        setReceipt(written);
        finish();
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  return (
    <main className="wizard" aria-labelledby="wizard-heading">
      <h1 id="wizard-heading">欢迎使用 Soul</h1>
      <p data-testid="wizard-locality">
        Soul 在这台电脑上给你建一个电子版的你：人格、记忆、心理工作模型和人脉图。
        它只处理你交给它的东西。处理过程留在本机；只有你以后自己填写的模型端点例外，发出去的内容会先占位。
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
                <td className="muted" data-testid={`wizard-note-${line.name}`}>
                  {line.note}
                </td>
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

      {asking ? null : (
        <button
          type="button"
          className="primary"
          disabled={!acknowledged}
          onClick={() => setAsking(true)}
        >
          下一步
        </button>
      )}

      {asking ? (
        <section className="panel" aria-labelledby="questions-heading">
          <h2 id="questions-heading">先认识一下你（{questions.length} 题）</h2>
          <p className="muted">
            这些答案是档案的第一份材料，每一条都会记成「你自己说的」，
            以后在档案页上点开就能看到某句话是从哪一题来的。
            没把握的题跳过就好：跳过的轴会留成「还看不出方向」，Soul 不会替你猜。
          </p>
          <ol className="questions" data-testid="wizard-questions">
            {questions.map((question) => (
              <Ask
                key={question.question_id}
                question={question}
                given={given[question.question_id] ?? ""}
                busy={busy}
                onGive={(value) =>
                  setGiven((previous) => ({ ...previous, [question.question_id]: value }))
                }
              />
            ))}
          </ol>
          <p className="muted" data-testid="wizard-answered">
            已答 {answered} 题，跳过 {questions.length - answered} 题。
          </p>
          <div className="switch-row">
            <button
              type="button"
              className="primary"
              disabled={!acknowledged || busy || answered === 0}
              onClick={recordThenFinish}
            >
              写进档案，开始使用
            </button>
            <button type="button" disabled={!acknowledged || busy} onClick={finish}>
              一题都不答，直接开始
            </button>
          </div>
          <p className="muted">
            直接开始也可以，档案会是空的；这些题在档案页的「再答几题」里随时可以再答。
          </p>
        </section>
      ) : null}

      {receipt === null ? null : (
        <p className="muted" data-testid="wizard-receipt">
          记下了 {receipt.answered} 条，都是「你自己说的」。
          {receipt.axes_known} 条轴有了方向，{receipt.axes_unknown} 条留成还看不出方向。
        </p>
      )}

      {refusal === null ? null : (
        <section className="panel refusal" role="alert" aria-labelledby="wizard-refused-heading">
          <h2 id="wizard-refused-heading">这一步没有过去</h2>
          <p data-testid="wizard-refusal-code">{refusal.reason_code}</p>
          <p>{refusal.explanation}</p>
        </section>
      )}
    </main>
  );
}
