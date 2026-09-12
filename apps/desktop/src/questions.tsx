/**
 * How one of the eleven questions is drawn, for the two screens that ask them.
 *
 * The wizard asks them once and 灵魂档案 asks them again afterwards, and the
 * two have to be the same question in the same words: `soul-profile` pairs
 * each entry of `soul_import::questionnaire::QUESTIONS` with the option tokens
 * the recorder will accept and the words for them, so a second copy of the
 * layout here would be a second place for an option label to drift away from
 * the token it stands for. There is no Chinese in this file that names an
 * answer — `option.reading` is the core's, and `option.value` is what goes
 * back.
 */

import type { Question } from "./core";

export interface AskProps {
  readonly question: Question;
  readonly given: string;
  readonly busy: boolean;
  readonly onGive: (given: string) => void;
  /** Distinguishes the wizard's copy of a question from the profile's. */
  readonly idPrefix?: string;
}

/**
 * One question: three buttons, or a text box.
 *
 * A chosen option can be un-chosen by pressing it again, because "I answered
 * this and then thought better of it" has to be reachable without restarting
 * the wizard — and an answer withdrawn is a blank, which is a skip.
 */
export function Ask({
  question,
  given,
  busy,
  onGive,
  idPrefix = "question",
}: AskProps): React.JSX.Element {
  const label = `${idPrefix}-${question.question_id}`;
  return (
    <li className="question">
      <p id={label}>{question.prompt}</p>
      {question.prose ? (
        <textarea
          className="paste-box"
          aria-labelledby={label}
          rows={2}
          value={given}
          disabled={busy}
          onChange={(event) => onGive(event.target.value)}
        />
      ) : (
        <div className="switch-row" role="group" aria-labelledby={label}>
          {question.options.map((option) => (
            <button
              key={option.value}
              type="button"
              disabled={busy}
              aria-pressed={given === option.value}
              className={given === option.value ? "primary" : undefined}
              onClick={() => onGive(given === option.value ? "" : option.value)}
            >
              {option.reading}
            </button>
          ))}
        </div>
      )}
    </li>
  );
}
