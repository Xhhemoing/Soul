/**
 * What the shell does when the core says no.
 *
 * Every route needs the same two things and they were being written out once
 * per route: turn whatever came back out of a rejected promise into a
 * [`Refusal`], and put it on screen without improving on it. The sentence is
 * always the core's own — never one this file composed, and never an operating
 * system error quoted back with a path in it — because a refusal a screen
 * wrote is a refusal no Rust test has read.
 */

import type { Refusal } from "./core";

/**
 * A refusal, or the shape of one for an error that did not arrive as a
 * refusal at all — the core is not answering, which the user still has to be
 * told about rather than left in front of a spinner.
 */
export function asRefusal(error: unknown): Refusal {
  const shaped = error as Partial<Refusal> | null;
  return typeof shaped?.reason_code === "string" && typeof shaped.explanation === "string"
    ? { reason_code: shaped.reason_code, explanation: shaped.explanation }
    : { reason_code: "unavailable", explanation: String(error) };
}

export interface RefusedProps {
  /** What did not happen, in the words of the page it did not happen on. */
  readonly title: string;
  readonly refusal: Refusal;
  /** Distinguishes one route's refusal from another's in a test. */
  readonly testId: string;
}

export function Refused({ title, refusal, testId }: RefusedProps): React.JSX.Element {
  return (
    <section className="panel refusal" role="alert" aria-labelledby={`${testId}-heading`}>
      <h2 id={`${testId}-heading`}>{title}</h2>
      <p data-testid={testId}>{refusal.reason_code}</p>
      <p>{refusal.explanation}</p>
    </section>
  );
}
