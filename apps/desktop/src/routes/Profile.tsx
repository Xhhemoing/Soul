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
 */

import { useEffect, useState } from "react";

import {
  correctAxis,
  profileScreen,
  setVoice,
  type AxisRow,
  type ProfileScreen,
  type Refusal,
  type StatedRow,
  type VoiceFieldRow,
} from "../core";
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
        <p className="muted">
          方向是从你说过的话和导入的往来里推出来的工作假设：只有偏向，没有高低，也没有名次。
          你觉得哪一条不对就按下去改；改过之后这条轴就锁住了，后面再推出别的也不会覆盖你。
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
            向导里那三道填空题你都跳过了。跳过就是跳过，这里不会替你写点什么。
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
    </>
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
