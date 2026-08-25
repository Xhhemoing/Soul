/**
 * `generic-5mm`: a documented generic 5mm fuse-bead palette. It is deliberately
 * NOT a transcription of any real brand's colour card — the ids and names are
 * this project's own, so nothing here depends on a vendor list.
 *
 * The entries are the oracle's (`crates/bead-core/src/palette.rs`), code for
 * code and hex for hex. BD18/BD8 make `bead-core` the algorithm oracle, and a
 * colour-code sequence can only be compared across the two implementations when
 * the same id means the same colour on both sides (AT-1).
 *
 * Nearest-colour lookup pins contract gaps G2/G6: minimise ΔE00, and on an
 * exact tie take the lowest palette index so Rust and TypeScript cannot drift
 * apart at a near tie.
 */

import { ciede2000, rgbToLab, type Lab, type Rgb } from "./color.ts";

export interface PaletteEntry {
  /** Colour code shown next to every swatch. Colour is never the only signal. */
  readonly id: string;
  readonly displayName: string;
  readonly rgb: Rgb;
}

export interface Palette {
  readonly id: string;
  readonly displayName: string;
  readonly entries: readonly PaletteEntry[];
}

function entry(id: string, displayName: string, hex: string): PaletteEntry {
  const value = Number.parseInt(hex.slice(1), 16);
  return {
    id,
    displayName,
    rgb: { r: (value >> 16) & 255, g: (value >> 8) & 255, b: value & 255 },
  };
}

export const GENERIC_5MM: Palette = {
  id: "generic-5mm",
  displayName: "通用 5mm 熔豆板",
  entries: [
    entry("G01", "White", "#FFFFFF"),
    entry("G02", "Cream", "#F5EFE0"),
    entry("G03", "Light Grey", "#D3D3D3"),
    entry("G04", "Grey", "#9E9E9E"),
    entry("G05", "Dark Grey", "#5C5C5C"),
    entry("G06", "Black", "#000000"),
    entry("G07", "Silver", "#B7BFC6"),
    entry("G08", "Slate", "#6B7A85"),
    entry("G09", "Charcoal", "#2B2F33"),
    entry("G10", "Pale Pink", "#FFD9E2"),
    entry("G11", "Pink", "#FF9EC4"),
    entry("G12", "Rose", "#F0559A"),
    entry("G13", "Magenta", "#E0218A"),
    entry("G14", "Light Red", "#FF6F61"),
    entry("G15", "Red", "#E4032E"),
    entry("G16", "Dark Red", "#9B1B24"),
    entry("G17", "Salmon", "#FFA48A"),
    entry("G18", "Orange", "#F5821F"),
    entry("G19", "Dark Orange", "#D2601A"),
    entry("G20", "Peach", "#FFCBA4"),
    entry("G21", "Light Yellow", "#FFF3A1"),
    entry("G22", "Yellow", "#FFD400"),
    entry("G23", "Gold", "#E0A526"),
    entry("G24", "Lime", "#C6DE41"),
    entry("G25", "Light Green", "#8CC63F"),
    entry("G26", "Green", "#2E9E45"),
    entry("G27", "Dark Green", "#14602D"),
    entry("G28", "Mint", "#A8E6CF"),
    entry("G29", "Teal", "#009B9F"),
    entry("G30", "Dark Teal", "#00666B"),
    entry("G31", "Sky Blue", "#8FD3F4"),
    entry("G32", "Light Blue", "#4FA3E3"),
    entry("G33", "Blue", "#0B61A4"),
    entry("G34", "Dark Blue", "#123A6B"),
    entry("G35", "Navy", "#0B1E3C"),
    entry("G36", "Periwinkle", "#9FA8DA"),
    entry("G37", "Violet", "#7A5CC4"),
    entry("G38", "Purple", "#59259E"),
    entry("G39", "Dark Purple", "#3B1660"),
    entry("G40", "Lavender", "#D6C7EA"),
    entry("G41", "Beige", "#E3C79A"),
    entry("G42", "Tan", "#C9A06A"),
    entry("G43", "Light Brown", "#A9713F"),
    entry("G44", "Brown", "#7B4B25"),
    entry("G45", "Dark Brown", "#4A2B14"),
    entry("G46", "Skin Light", "#FFE0C4"),
    entry("G47", "Skin Medium", "#E8B48A"),
    entry("G48", "Skin Deep", "#A96A46"),
  ],
};

/** Index into `Palette.entries`. The grid stores these, never raw RGB. */
export type ColorIndex = number;

export interface PreparedPalette {
  readonly palette: Palette;
  readonly labs: readonly Lab[];
}

const PREPARED = new WeakMap<Palette, PreparedPalette>();

export function preparePalette(palette: Palette): PreparedPalette {
  const cached = PREPARED.get(palette);
  if (cached !== undefined) return cached;
  const prepared: PreparedPalette = {
    palette,
    labs: palette.entries.map((it) => rgbToLab(it.rgb)),
  };
  PREPARED.set(palette, prepared);
  return prepared;
}

export interface NearestMatch {
  readonly index: ColorIndex;
  readonly deltaE: number;
  /** ΔE of the runner-up, `Infinity` for a one-entry palette. Feeds the near-tie sentinel (T-PAR-3). */
  readonly runnerUpDeltaE: number;
}

export function nearestEntry(lab: Lab, prepared: PreparedPalette): NearestMatch {
  const { labs } = prepared;
  if (labs.length === 0) {
    throw new Error("EmptyPalette: 色板没有任何条目");
  }
  let index = 0;
  let best = Infinity;
  let runnerUp = Infinity;
  for (let i = 0; i < labs.length; i += 1) {
    const deltaE = ciede2000(lab, labs[i]!);
    // Strict `<` keeps the lowest index on an exact tie (G2/G6).
    if (deltaE < best) {
      runnerUp = best;
      best = deltaE;
      index = i;
    } else if (deltaE < runnerUp) {
      runnerUp = deltaE;
    }
  }
  return { index, deltaE: best, runnerUpDeltaE: runnerUp };
}

export function nearestIndex(rgb: Rgb, prepared: PreparedPalette): ColorIndex {
  return nearestEntry(rgbToLab(rgb), prepared).index;
}

export function entryAt(palette: Palette, index: ColorIndex): PaletteEntry {
  const found = palette.entries[index];
  if (found === undefined) {
    throw new RangeError(`色板 ${palette.id} 没有索引 ${index}`);
  }
  return found;
}
