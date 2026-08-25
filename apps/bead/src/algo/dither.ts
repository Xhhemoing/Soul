/**
 * Quantisation, with Floyd–Steinberg as an opt-in.
 *
 * Contract gap G7 is pinned as:
 * - classic raster scan (left→right, top→bottom); no serpentine;
 * - error accumulates in sRGB *code value* space as f64 and is never clamped
 *   early — the clamp to [0,255] happens only just before the nearest-colour
 *   lookup;
 * - error that would land outside the image is dropped;
 * - empty cells (alpha < 128, G1) neither receive nor forward error.
 *
 * With dithering off this is a plain per-pixel nearest-colour map, which is
 * exactly what T-FS-1 asserts.
 */

import { clamp } from "./color.ts";
import { createGrid, type Cell, type Grid } from "./grid.ts";
import { isOpaque, type RgbaImage } from "./image.ts";
import { nearestEntry, preparePalette, type Palette } from "./palette.ts";
import { rgbToLab } from "./color.ts";

export interface QuantizeOptions {
  readonly palette: Palette;
  readonly dither: boolean;
}

export interface QuantizeResult {
  readonly grid: Grid;
  /**
   * Smallest `runnerUp − best` ΔE seen over all opaque pixels. `Infinity` when
   * nothing was quantised. The near-tie sentinel (T-PAR-3) reads this.
   */
  readonly minRunnerUpMargin: number;
}

const WEIGHT_RIGHT = 7 / 16;
const WEIGHT_DOWN_LEFT = 3 / 16;
const WEIGHT_DOWN = 5 / 16;
const WEIGHT_DOWN_RIGHT = 1 / 16;

export function quantize(image: RgbaImage, options: QuantizeOptions): QuantizeResult {
  const prepared = preparePalette(options.palette);
  const { width, height, data } = image;
  const cells: Cell[] = new Array<Cell>(width * height).fill(null);
  let minMargin = Infinity;

  // Unclamped f64 error carriers, one per channel, in sRGB code-value space.
  const carriers = options.dither ? width * height : 0;
  const errR = new Float64Array(carriers);
  const errG = new Float64Array(carriers);
  const errB = new Float64Array(carriers);

  const spread = (x: number, y: number, dr: number, dg: number, db: number, weight: number) => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    const target = y * width + x;
    // An empty cell is not a relay: it neither keeps nor forwards the error.
    if (!isOpaque(data[target * 4 + 3]!)) return;
    errR[target] = errR[target]! + dr * weight;
    errG[target] = errG[target]! + dg * weight;
    errB[target] = errB[target]! + db * weight;
  };

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const i = y * width + x;
      const o = i * 4;
      if (!isOpaque(data[o + 3]!)) continue;

      const rawR = data[o]! + (errR[i] ?? 0);
      const rawG = data[o + 1]! + (errG[i] ?? 0);
      const rawB = data[o + 2]! + (errB[i] ?? 0);

      const lookup = {
        r: clamp(rawR, 0, 255),
        g: clamp(rawG, 0, 255),
        b: clamp(rawB, 0, 255),
      };
      const match = nearestEntry(rgbToLab(lookup), prepared);
      cells[i] = match.index;
      const margin = match.runnerUpDeltaE - match.deltaE;
      if (margin < minMargin) minMargin = margin;

      if (!options.dither) continue;

      const chosen = prepared.palette.entries[match.index]!.rgb;
      // Error is measured against the *unclamped* accumulated value so a run of
      // saturated pixels cannot silently swallow it.
      const dr = rawR - chosen.r;
      const dg = rawG - chosen.g;
      const db = rawB - chosen.b;

      spread(x + 1, y, dr, dg, db, WEIGHT_RIGHT);
      spread(x - 1, y + 1, dr, dg, db, WEIGHT_DOWN_LEFT);
      spread(x, y + 1, dr, dg, db, WEIGHT_DOWN);
      spread(x + 1, y + 1, dr, dg, db, WEIGHT_DOWN_RIGHT);
    }
  }

  return { grid: createGrid(width, height, cells), minRunnerUpMargin: minMargin };
}
