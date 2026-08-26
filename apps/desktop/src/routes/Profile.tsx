/**
 * 灵魂档案. The five axes, the voice, and the lock a correction turns on.
 *
 * Three things this screen deliberately does not do. It writes no sentence
 * about the user: every reading on it — the axis directions, the voice words,
 * the summary at the bottom — was built in `soul-profile` and went through
 * `soul-policy`'s non-clinical check before it crossed the IPC, so a phrase
 * added here would be one nothing had read. It shows no boundary and no value
 * the user typed: those are sealed in the event the recorder wrote, and
 * `ProfileScreen` has no field that could carry them, only the question they
 * answered and the two ids that point at them. And it keeps showing the
 * inferences a correction refused — AC-07 is only visible if the user can see
 * that the machine still disagrees and is not allowed to act on it.
 *
 * Correcting an axis is a write, so the whole screen comes back from the core
 * afterwards rather than being patched here: the lock, the band and the
 * summary all move, and a screen that updated one of them itself would be
 * guessing at the other two.
 *
 * ## 再答几题
 *
 * The wizard is a first run and nothing else: `wizard_completed` is written
 * once and the screen never comes back. So the eleven questions live here too,
 * because otherwise a user who pressed 一题都不答，直接开始 would have no way
 * left to state a boundary or a value at all — the three text boxes are the
 * only path into `boundaries` and `values`, and an axis this page can correct
 * is an axis the questionnaire could have answered outright. Same command,
 * same list, same option tokens; `../questions` draws them so the wizard and
 * this page cannot drift apart. Answering again replaces rather than appends,
 * which the core does, and a blank stays a skip.
 */

import { useEffect, useState } from "react";

import {
  answerQuestionnaire,
  correctAxis,
  profileScreen,
  questionnaire,
  setVoice,
  type AxisRow,
  type IntakeReceipt,
  type ProfileScreen,
  type Question,
  type Refusal,
  type StatedRow,
  type VoiceFieldRow,
} from "../core";
import { Ask } from "../questions";
import { asRefusal, Refused } from "../refusal";

/** How much was observed, not how much anything is worth. */
const BAND: Record<string, string> = {
  none: "还没有证据",
  weak: "证据较少",
  moderate: "证据中等",
  strong: "证据较多",
};

/** Whether an inference still has evidence under it. */
const STATE: Record<string, string> = {
  live: "依据还在",
  orphaned: "依据已被遗忘",
};

/** Which half of the profile a sealed answer landed in. */
const STATED: Record<string, string> = {
  boundary: "边界",
  value: "在乎的事",
};

function words(source: Record<string, string>, key: string): string {
  return source[key] ?? key;
}

export function Profile(): React.JSX.Element {
  const [screen, setScreen] = useState<ProfileScreen | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    profileScreen().then(
      (value) => {
        if (live) setScreen(value);
      },
      (error: unknown) => {
        if (live) setRefusal(asRefusal(error));
      },
    );
    return () => {
      live = false;
    };
  }, []);

  /** Both writes answer with the whole screen, so both are the same call. */
  const write = (change: Promise<ProfileScreen>): void => {
    setBusy(true);
    setRefusal(null);
    change.then(
      (value) => {
        setScreen(value);
        setBusy(false);
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setBusy(false);
      },
    );
  };

  if (screen === null) {
    return refusal === null ? (
      <section className="panel" aria-busy="true">
        <p>正在读这台机器上的档案…</p>
      </section>
    ) : (
      <Refused title="档案没有读出来" refusal={refusal} testId="profile-refusal-code" />
    );
  }

  const answered = screen.axes.filter((axis) => axis.position !== "unknown").length;

  return (
    <>
      <section className="panel" aria-labelledby="reading-heading">
        <h2 id="reading-heading">现在的档案</h2>
        <p className="reading" data-testid="profile-reading">
          {screen.reading}
        </p>
        <p className="muted" data-testid="profile-known">
          五条轴里有 {answered} 条有方向，其余 {screen.axes.length - answered}{" "}
          条还看不出方向。没答过的题不会被猜。
        </p>
        <p className="badge" data-testid="profile-notice">
          {screen.notice}
        </p>
      </section>

      {refusal === null ? null : (
        <Refused title="这一次没有改成" refusal={refusal} testId="profile-refusal-code" />
      )}

      <section className="panel" aria-labelledby="axes-heading">
        <h2 id="axes-heading">特质轴（{screen.axes.length}）</h2>
        <p className="muted" data-testid="axes-explanation">
          方向来自你答过的那些题，和你在这一页按下去的纠正：只有偏向，没有高低，也没有名次。
          你觉得哪一条不对就按下去改；改过之后这条轴就锁住了，以后机器再推出别的方向也不会覆盖你。
        </p>
        <ul className="facts" data-testid="axis-list">
          {screen.axes.map((axis) => (
            <Axis
              key={axis.axis_id}
              axis={axis}
              busy={busy}
              onCorrect={(position) => write(correctAxis(axis.axis_id, position))}
            />
          ))}
        </ul>
      </section>

      <section className="panel" aria-labelledby="voice-heading">
        <h2 id="voice-heading">语气</h2>
        <p className="muted" data-testid="voice-reading">
          {screen.voice.reading}
        </p>
        <ul className="facts" data-testid="voice-list">
          {screen.voice.fields.map((field) => (
            <Voice
              key={field.field}
              field={field}
              busy={busy}
              onChoose={(option) => write(setVoice(field.field, option))}
            />
          ))}
        </ul>
      </section>

      <section className="panel" aria-labelledby="stated-heading">
        <h2 id="stated-heading">你自己写过的（{screen.stated.length}）</h2>
        {screen.stated.length === 0 ? (
          <p className="muted" data-testid="no-stated">
            向导里那三道填空题你都跳过了。跳过就是跳过，这里不会替你写点什么；
            想说的时候，下面的「再答几题」里那三道题还在。
          </p>
        ) : (
          <ul className="facts" data-testid="stated-list">
            {screen.stated.map((row) => (
              <Stated key={row.evidence_id} row={row} />
            ))}
          </ul>
        )}
        <p className="muted">
          你写的原话密封在库里，这个页面不去打开它，只留着指回去的编号。
        </p>
      </section>

      <AskAgain busy={busy} onRecorded={() => write(profileScreen())} />
    </>
  );
}

interface AskAgainProps {
  readonly busy: boolean;
  /** The questionnaire wrote something, so the screen above it is stale. */
  readonly onRecorded: () => void;
}

/**
 * The eleven questions again, after the wizard is behind us.
 *
 * The list and the option words come from `questionnaire()`, the same command
 * the wizard calls, so this is the wizard's questionnaire rather than a
 * shorter one somebody assembled for this page. Handing in a questionnaire
 * where every box is blank is refused by the core, and the button is grey
 * until one of them is not, so the refusal is a thing the user is kept out of
 * rather than shown after the fact.
 *
 * Nothing that was answered comes back into the boxes. The three prose answers
 * are sealed and `ProfileScreen` has nowhere to put them, and pre-filling a
 * choice question with a position that inference moved would be this page
 * putting words in the user's mouth. So the boxes start empty every time, and
 * what is on screen above says what the core currently holds.
 */
function AskAgain({ busy, onRecorded }: AskAgainProps): React.JSX.Element {
  const [questions, setQuestions] = useState<readonly Question[]>([]);
  /** False until the user asks for them; the profile is what this page is. */
  const [asking, setAsking] = useState(false);
  const [given, setGiven] = useState<Record<string, string>>({});
  const [receipt, setReceipt] = useState<IntakeReceipt | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [writing, setWriting] = useState(false);

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
  /** Counted rather than written down: which questions are text boxes is the
   *  core's list to change, and 边界 has no other way in. */
  const prose = questions.filter((question) => question.prose).length;
  const stopped = busy || writing;

  /**
   * Hand in what was typed, then ask for the profile again.
   *
   * Every question goes back, including the blank ones, exactly as the wizard
   * hands them in: a blank is a skip the core drops, not an instruction to
   * erase what an earlier run recorded.
   */
  const record = (): void => {
    setWriting(true);
    setRefusal(null);
    setReceipt(null);
    const answers = questions.map((question) => ({
      question_id: question.question_id,
      given: given[question.question_id] ?? "",
    }));
    answerQuestionnaire(answers).then(
      (written) => {
        setReceipt(written);
        setGiven({});
        setWriting(false);
        onRecorded();
      },
      (error: unknown) => {
        setRefusal(asRefusal(error));
        setWriting(false);
      },
    );
  };

  return (
    <section className="panel" aria-labelledby="ask-again-heading">
      <h2 id="ask-again-heading">再答几题</h2>
      <p className="muted">
        向导里那 {questions.length} 道题在这里随时可以再答，答过的也可以改口，以你最后说的为准；
        留空的还是留空，不会被猜。其中 {prose} 道填空题是边界和在乎的事唯一的入口，
        写进去之后原话一样密封，这一页只拿得到问题和编号。
      </p>

      {asking ? null : (
        <button
          type="button"
          disabled={stopped || questions.length === 0}
          onClick={() => setAsking(true)}
        >
          打开题目
        </button>
      )}

      {asking ? (
        <>
          <ol className="questions" data-testid="profile-questions">
            {questions.map((question) => (
              <Ask
                key={question.question_id}
                question={question}
                given={given[question.question_id] ?? ""}
                busy={stopped}
                idPrefix="profile-question"
                onGive={(value) =>
                  setGiven((previous) => ({ ...previous, [question.question_id]: value }))
                }
              />
            ))}
          </ol>
          <p className="muted" data-testid="profile-answered">
            这一次答了 {answered} 题，留空 {questions.length - answered} 题。
          </p>
          <div className="switch-row">
            <button
              type="button"
              className="primary"
              disabled={stopped || answered === 0}
              onClick={record}
            >
              写进档案
            </button>
            <button
              type="button"
              disabled={stopped}
              onClick={() => {
                setAsking(false);
                setGiven({});
              }}
            >
              先收起来
            </button>
          </div>
          {answered === 0 ? (
            <p className="muted" data-testid="profile-nothing-answered">
              一道都没答的时候没有东西可以写进去，所以上面那个按钮是灰的。
            </p>
          ) : null}
        </>
      ) : null}

      {receipt === null ? null : (
        <div data-testid="profile-receipt">
          <p className="muted">
            记下了 {receipt.answered} 条，都是「你自己说的」。
            {receipt.axes_known} 条轴有了方向，{receipt.axes_unknown} 条留成还看不出方向。
          </p>
          {receipt.ignored.length === 0 ? null : (
            <ul data-testid="profile-ignored">
              {receipt.ignored.map((row) => (
                <li key={`${row.question_id}:${row.reason}`}>
                  {row.question_id} 已跳过（{row.reason}）
                </li>
              ))}
            </ul>
          )}
        </div>
      )}

      {refusal === null ? null : (
        <Refused title="这几题没有过去" refusal={refusal} testId="profile-ask-refusal-code" />
      )}
    </section>
  );
}

interface AxisProps {
  readonly axis: AxisRow;
  readonly busy: boolean;
  readonly onCorrect: (position: string) => void;
}

function Axis({ axis, busy, onCorrect }: AxisProps): React.JSX.Element {
  /**
   * A position in this axis's own words. The core sends the same three
   * readings it offers as corrections, so an inference is described in the
   * vocabulary the axis was defined with rather than as `leans_high`.
   */
  const direction = (position: string): string =>
    axis.choices.find((choice) => choice.position === position)?.reading ?? position;

  return (
    <li data-testid={`axis-${axis.axis_id}`}>
      <span data-testid={`axis-reading-${axis.axis_id}`}>{axis.reading}</span>
      <span className="muted">
        {" "}
        {words(BAND, axis.evidence_band)}（{axis.evidence_count} 条）
      </span>
      {axis.locked_by_user ? (
        <span className="badge" data-testid={`axis-locked-${axis.axis_id}`}>
          你纠正过，推断不再改这条
        </span>
      ) : null}
      <div className="switch-row">
        {axis.choices.map((choice) => (
          <button
            key={choice.position}
            type="button"
            disabled={busy || choice.position === axis.position}
            onClick={() => onCorrect(choice.position)}
          >
            {choice.reading}
          </button>
        ))}
      </div>
      {axis.inferences.length === 0 ? null : (
        <ul className="facts" data-testid={`axis-inferences-${axis.axis_id}`}>
          {axis.inferences.map((inference) => (
            <li key={inference.inference_id} className="muted">
              机器推的是「{direction(inference.position)}」，{words(BAND, inference.band)}（
              {inference.evidence_count} 条），{words(STATE, inference.state)}
              {axis.locked_by_user ? "，因为你锁定过所以没有生效" : ""}
              {inference.falsifier === null ? "" : `。什么会推翻它：${inference.falsifier}`}
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

interface VoiceProps {
  readonly field: VoiceFieldRow;
  readonly busy: boolean;
  readonly onChoose: (option: string) => void;
}

function Voice({ field, busy, onChoose }: VoiceProps): React.JSX.Element {
  return (
    <li data-testid={`voice-${field.field}`}>
      <span>{field.label}</span>
      {field.locked_by_user ? (
        <span className="badge" data-testid={`voice-locked-${field.field}`}>
          你自己设过，推断不再改这项
        </span>
      ) : null}
      <div className="switch-row">
        {field.options.map((option) => (
          <button
            key={option.value}
            type="button"
            disabled={busy || option.value === field.value}
            onClick={() => onChoose(option.value)}
          >
            {option.reading}
          </button>
        ))}
      </div>
    </li>
  );
}

interface StatedProps {
  readonly row: StatedRow;
}

/**
 * One thing the user wrote, as a pointer.
 *
 * The question is shown and the answer is not, which is the whole shape of
 * this row: the words are sealed and `StatedRow` has nowhere to put them, so
 * what is on screen is which question was answered and the ids that would let
 * a forget find it.
 */
function Stated({ row }: StatedProps): React.JSX.Element {
  return (
    <li>
      <span className="muted">{words(STATED, row.field)}｜</span>
      <span>{row.prompt}</span>
      <span className="muted">
        {" "}
        你答过（原话密封，编号 <code>{row.evidence_id}</code>）
      </span>
    </li>
  );
}
