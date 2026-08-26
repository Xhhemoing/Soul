/**
 * `Grid` is the hand-off type between the image half of the pipeline and the
 * step splitters / BOM. Cells hold a palette index or `null` for an empty cell
 * (G1). 0×0 grids are legal and produce zero steps and an empty BOM (G8).
 */

import type { ColorIndex } from "./palette.ts";

export type Cell = ColorIndex | null;

export interface Grid {
  readonly width: number;
  readonly height: number;
  /** Row-major, length `width * height`. */
  readonly cells: readonly Cell[];
}

export interface CellRef {
  readonly x: number;
  readonly y: number;
  readonly color: ColorIndex;
}

export function createGrid(width: number, height: number, cells?: readonly Cell[]): Grid {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 0 || height < 0) {
    throw new RangeError(`网格尺寸必须是非负整数，收到 ${width}×${height}`);
  }
  const size = width * height;
  if (cells !== undefined && cells.length !== size) {
    throw new RangeError(`网格单元数 ${cells.length} 与 ${width}×${height} 不符`);
  }
  return { width, height, cells: cells ?? new Array<Cell>(size).fill(null) };
}

export function cellAt(grid: Grid, x: number, y: number): Cell {
  if (x < 0 || y < 0 || x >= grid.width || y >= grid.height) return null;
  return grid.cells[y * grid.width + x] ?? null;
}

export function isEmptyGrid(grid: Grid): boolean {
  return grid.cells.every((cell) => cell === null);
}

export function occupiedCount(grid: Grid): number {
  let total = 0;
  for (const cell of grid.cells) if (cell !== null) total += 1;
  return total;
}

/** Non-empty cells in row-major order — the canonical order every splitter uses inside a step. */
export function rowMajorCells(grid: Grid): CellRef[] {
  const out: CellRef[] = [];
  for (let y = 0; y < grid.height; y += 1) {
    for (let x = 0; x < grid.width; x += 1) {
      const cell = grid.cells[y * grid.width + x];
      if (cell !== null && cell !== undefined) out.push({ x, y, color: cell });
    }
  }
  return out;
}
