/**
 * The four step-splitting modes. Every mode partitions the non-empty cells:
 * no cell is dropped, none is emitted twice, and empty cells never appear
 * (T-SPL-0). Cell order inside a step is always row-major.
 *
 * Tie-breaks follow contract gap G6: palette index ascending, and components
 * ordered by their top-most then left-most cell.
 */

import { cellAt, rowMajorCells, type CellRef, type Grid } from "./grid.ts";
import { BOARD_28 } from "./framing.ts";
import type { ColorIndex } from "./palette.ts";

export type SplitMode = "color-by-color" | "tile" | "outline-infill" | "row-by-row";

export type StepPart = "all" | "outline" | "inner-border" | "fill";

export interface Step {
  readonly mode: SplitMode;
  /** Palette index when the step is single-coloured, otherwise null. */
  readonly color: ColorIndex | null;
  /** Which slice of an Outline→Infill component this is; `all` elsewhere. */
  readonly part: StepPart;
  /** Tile origin / component index / row index, depending on the mode. */
  readonly group: number;
  readonly cells: readonly CellRef[];
}

export interface ColorByColorOptions {
  /** Default `ascending`: rare accent colours first. */
  readonly order?: "ascending" | "descending";
}

export function colorByColor(grid: Grid, options: ColorByColorOptions = {}): Step[] {
  const order = options.order ?? "ascending";
  const buckets = new Map<ColorIndex, CellRef[]>();
  for (const cell of rowMajorCells(grid)) {
    const bucket = buckets.get(cell.color);
    if (bucket === undefined) buckets.set(cell.color, [cell]);
    else bucket.push(cell);
  }

  const colors = [...buckets.keys()].sort((a, b) => {
    const countA = buckets.get(a)!.length;
    const countB = buckets.get(b)!.length;
    if (countA !== countB) return order === "ascending" ? countA - countB : countB - countA;
    // Equal counts always fall back to palette index ascending, both ways round.
    return a - b;
  });

  return colors.map((color, group) => ({
    mode: "color-by-color" as const,
    color,
    part: "all" as const,
    group,
    cells: buckets.get(color)!,
  }));
}

/** Fixed 28×28 tiles, left→right then top→bottom. Edge tiles keep their real size. */
export function tileSplit(grid: Grid, tileSize: number = BOARD_28): Step[] {
  if (!Number.isInteger(tileSize) || tileSize <= 0) {
    throw new RangeError(`tile 尺寸必须是正整数，收到 ${tileSize}`);
  }
  const steps: Step[] = [];
  let group = 0;
  for (let ty = 0; ty < grid.height; ty += tileSize) {
    for (let tx = 0; tx < grid.width; tx += tileSize) {
      const cells: CellRef[] = [];
      const yEnd = Math.min(ty + tileSize, grid.height);
      const xEnd = Math.min(tx + tileSize, grid.width);
      for (let y = ty; y < yEnd; y += 1) {
        for (let x = tx; x < xEnd; x += 1) {
          const color = cellAt(grid, x, y);
          if (color !== null) cells.push({ x, y, color });
        }
      }
      // A tile with nothing in it is skipped rather than emitted empty.
      if (cells.length === 0) continue;
      steps.push({ mode: "tile", color: null, part: "all", group, cells });
      group += 1;
    }
  }
  return steps;
}

/** Rows top→bottom; one step per non-empty row; empty rows are skipped. */
export function rowByRow(grid: Grid): Step[] {
  const steps: Step[] = [];
  for (let y = 0; y < grid.height; y += 1) {
    const cells: CellRef[] = [];
    for (let x = 0; x < grid.width; x += 1) {
      const color = cellAt(grid, x, y);
      if (color !== null) cells.push({ x, y, color });
    }
    if (cells.length === 0) continue;
    steps.push({ mode: "row-by-row", color: null, part: "all", group: y, cells });
  }
  return steps;
}

const NEIGHBOURS_4 = [
  [0, -1],
  [-1, 0],
  [1, 0],
  [0, 1],
] as const;

const NEIGHBOURS_8 = [
  [-1, -1],
  [0, -1],
  [1, -1],
  [-1, 0],
  [1, 0],
  [-1, 1],
  [0, 1],
  [1, 1],
] as const;

/**
 * Marks background cells reachable from outside the grid by a 4-connected
 * flood. Everything else that is background is a hole (G4).
 */
function exteriorBackground(grid: Grid): Uint8Array {
  const { width, height } = grid;
  const outside = new Uint8Array(width * height);
  if (width === 0 || height === 0) return outside;

  const queue: number[] = [];
  const push = (x: number, y: number) => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    const i = y * width + x;
    if (outside[i] === 1) return;
    if (cellAt(grid, x, y) !== null) return;
    outside[i] = 1;
    queue.push(i);
  };

  for (let x = 0; x < width; x += 1) {
    push(x, 0);
    push(x, height - 1);
  }
  for (let y = 0; y < height; y += 1) {
    push(0, y);
    push(width - 1, y);
  }

  while (queue.length > 0) {
    const i = queue.pop()!;
    const x = i % width;
    const y = (i - x) / width;
    for (const [dx, dy] of NEIGHBOURS_4) push(x + dx, y + dy);
  }
  return outside;
}

/**
 * Connected components on the non-empty mask, 4-adjacency (G4). Within a
 * component: outer outline first (touching the exterior, 8-adjacency), then the
 * inner border (touching a hole), then the fill. A cell is classified once and
 * outline wins.
 */
export function outlineInfill(grid: Grid): Step[] {
  const { width, height } = grid;
  const outside = exteriorBackground(grid);
  const componentOf = new Int32Array(width * height).fill(-1);
  const components: CellRef[][] = [];

  // Row-major seeding makes component order "top-most, then left-most" for free.
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const start = y * width + x;
      if (componentOf[start] !== -1 || cellAt(grid, x, y) === null) continue;
      const id = components.length;
      const members: CellRef[] = [];
      const stack = [start];
      componentOf[start] = id;
      while (stack.length > 0) {
        const i = stack.pop()!;
        const cx = i % width;
        const cy = (i - cx) / width;
        members.push({ x: cx, y: cy, color: cellAt(grid, cx, cy)! });
        for (const [dx, dy] of NEIGHBOURS_4) {
          const nx = cx + dx;
          const ny = cy + dy;
          if (nx < 0 || ny < 0 || nx >= width || ny >= height) continue;
          const ni = ny * width + nx;
          if (componentOf[ni] !== -1 || cellAt(grid, nx, ny) === null) continue;
          componentOf[ni] = id;
          stack.push(ni);
        }
      }
      members.sort((a, b) => (a.y !== b.y ? a.y - b.y : a.x - b.x));
      components.push(members);
    }
  }

  const steps: Step[] = [];
  components.forEach((members, group) => {
    const outline: CellRef[] = [];
    const innerBorder: CellRef[] = [];
    const fill: CellRef[] = [];

    for (const cell of members) {
      let touchesExterior = false;
      let touchesHole = false;
      for (const [dx, dy] of NEIGHBOURS_8) {
        const nx = cell.x + dx;
        const ny = cell.y + dy;
        if (nx < 0 || ny < 0 || nx >= width || ny >= height) {
          touchesExterior = true;
          break;
        }
        if (cellAt(grid, nx, ny) !== null) continue;
        if (outside[ny * width + nx] === 1) {
          touchesExterior = true;
          break;
        }
        touchesHole = true;
      }
      if (touchesExterior) outline.push(cell);
      else if (touchesHole) innerBorder.push(cell);
      else fill.push(cell);
    }

    const emit = (part: StepPart, cells: CellRef[]) => {
      if (cells.length === 0) return;
      steps.push({ mode: "outline-infill", color: null, part, group, cells });
    };
    emit("outline", outline);
    emit("inner-border", innerBorder);
    emit("fill", fill);
  });

  return steps;
}

export interface SplitOptions extends ColorByColorOptions {
  readonly tileSize?: number;
}

export function splitSteps(grid: Grid, mode: SplitMode, options: SplitOptions = {}): Step[] {
  switch (mode) {
    case "color-by-color":
      return colorByColor(grid, options.order === undefined ? {} : { order: options.order });
    case "tile":
      return tileSplit(grid, options.tileSize ?? BOARD_28);
    case "outline-infill":
      return outlineInfill(grid);
    case "row-by-row":
      return rowByRow(grid);
  }
}

export const SPLIT_MODES: readonly SplitMode[] = [
  "color-by-color",
  "tile",
  "outline-infill",
  "row-by-row",
];
