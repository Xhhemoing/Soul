/**
 * Placeholder only. The colour maths (sRGB→Lab, CIEDE2000, palette mapping,
 * Floyd–Steinberg, board slicing, step-order generation) belongs to WP-B01
 * (`crates/bead-core`, the oracle) and WP-B03 (the browser port that must
 * reproduce the oracle's colour-code sequence on the same fixture).
 *
 * WP-B02 is the app shell. Implementing any of it here would fork the
 * algorithm before the oracle exists, so this module deliberately throws.
 */

export const NOT_IMPLEMENTED = "NOT_IMPLEMENTED" as const;

export type NotImplemented = typeof NOT_IMPLEMENTED;

/** Owning work packages, surfaced so callers can point users at the right one. */
export const ALGO_OWNERS = {
  oracle: "WP-B01",
  browserPipeline: "WP-B03",
} as const;

export class NotImplementedError extends Error {
  readonly code: NotImplemented = NOT_IMPLEMENTED;

  constructor(what: string) {
    super(`${NOT_IMPLEMENTED}: ${what} 归 ${ALGO_OWNERS.oracle} / ${ALGO_OWNERS.browserPipeline}`);
    this.name = "NotImplementedError";
  }
}

export function notImplemented(what: string): never {
  throw new NotImplementedError(what);
}
