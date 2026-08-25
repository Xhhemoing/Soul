/**
 * Bill of materials: colour code, display name, bead count.
 *
 * Sorted by count descending, then palette index ascending (G6). Empty cells
 * are not counted, so `Σ count` always equals the number of non-empty cells.
 */

import { entryAt, type ColorIndex, type Palette } from "./palette.ts";
import type { Grid } from "./grid.ts";

export interface BomRow {
  readonly index: ColorIndex;
  readonly code: string;
  readonly displayName: string;
  readonly count: number;
}

export function buildBom(grid: Grid, palette: Palette): BomRow[] {
  const counts = new Map<ColorIndex, number>();
  for (const cell of grid.cells) {
    if (cell === null) continue;
    counts.set(cell, (counts.get(cell) ?? 0) + 1);
  }

  return [...counts.entries()]
    .map(([index, count]) => {
      const entry = entryAt(palette, index);
      return { index, code: entry.id, displayName: entry.displayName, count };
    })
    .sort((a, b) => (a.count !== b.count ? b.count - a.count : a.index - b.index));
}

export function totalBeads(bom: readonly BomRow[]): number {
  return bom.reduce((sum, row) => sum + row.count, 0);
}
