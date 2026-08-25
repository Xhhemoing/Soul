/**
 * D-GAL-5 / D-GAL-6: Fork 改色, as pure functions.
 *
 * A gallery pattern's palette is a per-pattern table of codes a person typed
 * in — `paletteForNamespace("gallery")` returns null and `toPatternDoc` refuses
 * the whole document, by design. So a fork does not carry the gallery namespace
 * across; it quantises the pattern onto `generic-5mm`, which is the one palette
 * a `PatternDoc` can name, and from there the editor, the assembler, the
 * inventory requirements and the WP-B07 archive all pick it up unchanged.
 *
 * The quantisation is per palette *entry*, not per cell: `pattern.palette` is
 * at most ten rows, so this is ten ΔE00 searches instead of 3136 of them, and —
 * the reason that matters — two cells that were the same gallery colour are
 * still the same colour afterwards. Ties go to the lowest palette index, the
 * G2/G6 ruling `nearestEntry` already implements.
 *
 * Nothing here belongs in `algo/`: that tree is the oracle-parity domain
 * (BD18/BD8) and a fork is a product operation. Importing downhill from a page
 * into `algo/` is the direction D-ED already settled on.
 */

import { createGrid, type Cell, type Grid } from "../../algo/grid.ts";
import { GENERIC_5MM, nearestIndex, preparePalette, type ColorIndex } from "../../algo/palette.ts";
import type { FixtureGrid } from "../../fixtures/grids.ts";
import { parseHexColor } from "../../stores/inventory.ts";

/** The one field of a gallery swatch a fork reads. */
export interface ForkSwatch {
  readonly hex: string;
}

export interface ForkResult {
  /** The same shape as the fixture grid, indexed into `generic-5mm`. */
  readonly grid: Grid;
  /** Gallery palette index → `generic-5mm` index. */
  readonly map: readonly ColorIndex[];
  /** Generic-5mm indices two or more gallery colours landed on, ascending. */
  readonly merged: readonly ColorIndex[];
}

/**
 * The fixture is build-time code, so a hex it cannot parse is a bug in this
 * repository rather than bad input — it fails here instead of quietly forking a
 * board full of white. `parseHexColor` is reused rather than copied: a second
 * `#RRGGBB` reader is a second thing that can disagree about `#abc`.
 */
export function forkColorOf(hex: string): ColorIndex {
  const rgb = parseHexColor(hex);
  if (rgb === null) throw new RangeError(`画廊色板里不是 #RRGGBB 的颜色：${hex}`);
  return nearestIndex(rgb, preparePalette(GENERIC_5MM));
}

/** D-GAL-6: one entry in, one `generic-5mm` index out, in palette order. */
export function quantizeGalleryPalette(palette: readonly ForkSwatch[]): ColorIndex[] {
  return palette.map((swatch) => forkColorOf(swatch.hex));
}

/**
 * Shape-preserving: width, height and every empty cell stay where they were.
 * An index the map does not cover is a throw — `decodeFixtureRows` already
 * refuses those, and silently emptying the cell would lose beads.
 */
export function forkGrid(grid: Grid, map: readonly ColorIndex[]): Grid {
  const cells: Cell[] = grid.cells.map((cell) => {
    if (cell === null) return null;
    const mapped = map[cell];
    if (mapped === undefined) throw new RangeError(`画廊色号下标 ${cell} 不在映射表内`);
    return mapped;
  });
  return createGrid(grid.width, grid.height, cells);
}

/** Which `generic-5mm` codes two or more gallery colours collapsed onto (R-GAL-1). */
export function mergedTargets(map: readonly ColorIndex[]): ColorIndex[] {
  const seen = new Map<ColorIndex, number>();
  for (const index of map) seen.set(index, (seen.get(index) ?? 0) + 1);
  return [...seen.entries()]
    .filter(([, count]) => count > 1)
    .map(([index]) => index)
    .sort((a, b) => a - b);
}

/** Everything the Fork button needs from a fixture grid, in one pure call. */
export function forkFixture(fixture: FixtureGrid): ForkResult {
  const map = quantizeGalleryPalette(fixture.palette);
  return { grid: forkGrid(fixture.grid, map), map, merged: mergedTargets(map) };
}

/** D-GAL-7: the source shows up in the title and nowhere else (D-GAL-8). */
export const FORK_TITLE_SUFFIX = "（Fork）";

/** D-UP-11 / D-ED-4 share this ceiling; the suffix counts against it. */
export const MAX_FORK_TITLE_LENGTH = 64;

export function forkTitle(patternTitle: string): string {
  const room = MAX_FORK_TITLE_LENGTH - FORK_TITLE_SUFFIX.length;
  return `${patternTitle.trim().slice(0, room)}${FORK_TITLE_SUFFIX}`;
}
