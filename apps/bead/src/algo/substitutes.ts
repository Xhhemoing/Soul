/**
 * Substitute colours: what in the user's inventory can stand in for a colour
 * they do not have.
 *
 * The threshold is strictly `< 3` (G8) — ΔE00 = 3 exactly is rejected. The core
 * function does not special-case "the wanted colour is itself in stock"; a ΔE 0
 * candidate simply sorts first and the caller decides what that means (T-SUB-3).
 */

import { ciede2000, rgbToLab, type Lab, type Rgb } from "./color.ts";

export const SUBSTITUTE_MAX_DELTA_E = 3;

export interface InventoryColor {
  readonly code: string;
  readonly displayName: string;
  readonly rgb: Rgb;
}

export interface SubstituteCandidate {
  /** Position in the inventory list as given — the tie-break key (G6). */
  readonly index: number;
  readonly code: string;
  readonly displayName: string;
  readonly deltaE: number;
}

export interface SubstituteOptions {
  readonly maxDeltaE?: number;
}

/** Lab-level API, so the CIEDE2000 fixtures can drive the threshold directly. */
export function substitutesForLab(
  wanted: Lab,
  inventory: readonly Lab[],
  options: SubstituteOptions = {},
): { index: number; deltaE: number }[] {
  const limit = options.maxDeltaE ?? SUBSTITUTE_MAX_DELTA_E;
  const hits: { index: number; deltaE: number }[] = [];
  inventory.forEach((lab, index) => {
    const deltaE = ciede2000(wanted, lab);
    if (deltaE < limit) hits.push({ index, deltaE });
  });
  return hits.sort((a, b) => (a.deltaE !== b.deltaE ? a.deltaE - b.deltaE : a.index - b.index));
}

export function findSubstitutes(
  wanted: Rgb,
  inventory: readonly InventoryColor[],
  options: SubstituteOptions = {},
): SubstituteCandidate[] {
  const labs = inventory.map((it) => rgbToLab(it.rgb));
  return substitutesForLab(rgbToLab(wanted), labs, options).map(({ index, deltaE }) => {
    const item = inventory[index]!;
    return { index, code: item.code, displayName: item.displayName, deltaE };
  });
}
