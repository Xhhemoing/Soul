/**
 * `generic-5mm`: a documented generic 5mm fuse-bead palette. It is deliberately
 * NOT a transcription of any real brand's colour card — the ids and names are
 * this project's own, so nothing here depends on a vendor list.
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

function entry(id: string, displayName: string, r: number, g: number, b: number): PaletteEntry {
  return { id, displayName, rgb: { r, g, b } };
}

export const GENERIC_5MM: Palette = {
  id: "generic-5mm",
  displayName: "通用 5mm 熔豆板",
  entries: [
    entry("G01", "纯白", 255, 255, 255),
    entry("G02", "米白", 245, 238, 220),
    entry("G03", "浅灰", 200, 200, 200),
    entry("G04", "中灰", 150, 150, 150),
    entry("G05", "深灰", 90, 90, 90),
    entry("G06", "纯黑", 0, 0, 0),
    entry("G07", "粉红", 255, 170, 190),
    entry("G08", "亮红", 230, 40, 50),
    entry("G09", "深红", 150, 25, 35),
    entry("G10", "橙", 245, 130, 40),
    entry("G11", "浅橙", 250, 190, 120),
    entry("G12", "黄", 250, 220, 60),
    entry("G13", "浅黄", 250, 240, 170),
    entry("G14", "金", 215, 170, 60),
    entry("G15", "嫩绿", 170, 220, 120),
    entry("G16", "亮绿", 70, 175, 80),
    entry("G17", "深绿", 25, 105, 60),
    entry("G18", "墨绿", 15, 60, 45),
    entry("G19", "青", 60, 190, 190),
    entry("G20", "浅蓝", 150, 205, 235),
    entry("G21", "亮蓝", 40, 120, 200),
    entry("G22", "深蓝", 25, 55, 130),
    entry("G23", "藏青", 18, 30, 70),
    entry("G24", "淡紫", 200, 175, 225),
    entry("G25", "紫", 130, 70, 175),
    entry("G26", "深紫", 75, 35, 110),
    entry("G27", "品红", 215, 60, 150),
    entry("G28", "玫红", 240, 110, 160),
    entry("G29", "棕", 120, 80, 50),
    entry("G30", "浅棕", 185, 140, 100),
    entry("G31", "肤色", 250, 215, 185),
    entry("G32", "沙色", 225, 200, 160),
    entry("G33", "橄榄", 130, 140, 60),
    entry("G34", "军绿", 90, 105, 70),
    entry("G35", "天蓝", 110, 180, 230),
    entry("G36", "湖蓝", 30, 150, 175),
    entry("G37", "珊瑚", 250, 145, 130),
    entry("G38", "酒红", 110, 30, 60),
    entry("G39", "银灰", 175, 180, 185),
    entry("G40", "炭灰", 50, 52, 58),
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
