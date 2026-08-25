/**
 * D-ED-20: the editor's pure half — mirroring, flood fill, global replace and
 * the undo stack, with no DOM and no storage in the room.
 *
 * These deliberately do NOT live in `algo/` (D-ED-4 of the pixel-editor IA):
 * `algo/` is the oracle-parity domain (BD14/BD18 — every function there is
 * mirrored by `crates/bead-core` and pinned by parity tests), and an editing
 * gesture has no counterpart on the Rust side. `pages/assemble/session.ts` set
 * the precedent for page-private pure logic.
 *
 * The whole editing model is a stack of patches. A patch says what each touched
 * cell was and what it became, which makes undo the same operation as redo with
 * the two fields swapped, and which keeps a 100-entry history proportional to
 * what was drawn rather than to 100 copies of a 3136-cell board.
 */

import { buildBom, type BomRow } from "../../algo/bom.ts";
import { createGrid, type Cell, type Grid } from "../../algo/grid.ts";
import { GENERIC_5MM, type ColorIndex } from "../../algo/palette.ts";
import { paletteSwatches } from "../../stores/patterns.ts";

/**
 * D-ED-6: every document the editor can open names `generic-5mm`, so the 48
 * swatches are a module constant rather than a prop threaded through the tree.
 */
export const EDITOR_SWATCHES = paletteSwatches(GENERIC_5MM);

/** D-ED-8: the active colour is a palette index or「空」, which is the eraser. */
export type ActiveColor = Cell;

/** G06 Black: legible on either theme, so the first stroke is always visible. */
export const DEFAULT_COLOR: ColorIndex = 5;

/** D-ED-14: session history is capped; the oldest entry falls off the bottom. */
export const MAX_HISTORY = 100;

export type Tool = "brush" | "bucket" | "eyedropper";

/** D-ED-9: four settings, and the mirrors only ever apply to a brush stroke. */
export type Symmetry = "none" | "x" | "y" | "quad";

export interface Point {
  readonly x: number;
  readonly y: number;
}

export interface CellChange {
  readonly index: number;
  readonly before: Cell;
  readonly after: Cell;
}

/** One history entry: a whole stroke, a whole fill, or a whole replace. */
export type Edit = readonly CellChange[];

function indexOf(grid: Grid, point: Point): number {
  return point.y * grid.width + point.x;
}

function inBounds(grid: Grid, point: Point): boolean {
  return point.x >= 0 && point.y >= 0 && point.x < grid.width && point.y < grid.height;
}

function cellAtIndex(grid: Grid, index: number): Cell {
  return grid.cells[index] ?? null;
}

/**
 * D-ED-9: 28 and 56 are both even, so the mirror axis falls between columns and
 * `width-1-x` never maps a cell onto itself. The dedupe is here anyway — an odd
 * board would fold the centre column onto itself, and a stroke that paints the
 * same cell twice is a stroke that reports the wrong「改写颗数」.
 */
export function symmetryPoints(symmetry: Symmetry, grid: Grid, point: Point): Point[] {
  const mirroredX = grid.width - 1 - point.x;
  const mirroredY = grid.height - 1 - point.y;
  const candidates: Point[] = [point];
  if (symmetry === "x" || symmetry === "quad") candidates.push({ x: mirroredX, y: point.y });
  if (symmetry === "y" || symmetry === "quad") candidates.push({ x: point.x, y: mirroredY });
  if (symmetry === "quad") candidates.push({ x: mirroredX, y: mirroredY });

  const seen = new Set<number>();
  return candidates.filter((candidate) => {
    if (!inBounds(grid, candidate)) return false;
    const index = indexOf(grid, candidate);
    if (seen.has(index)) return false;
    seen.add(index);
    return true;
  });
}

/**
 * A pointer move can jump several cells between two events, and a stroke with
 * holes in it is not the stroke the hand drew. Bresenham fills the gap, and the
 * whole segment reaches the reducer as one batch (R-ED-6: never one setState
 * per cell).
 */
export function linePoints(from: Point, to: Point): Point[] {
  const dx = Math.abs(to.x - from.x);
  const dy = Math.abs(to.y - from.y);
  const stepX = from.x < to.x ? 1 : -1;
  const stepY = from.y < to.y ? 1 : -1;
  let error = dx - dy;
  let { x, y } = from;
  const points: Point[] = [{ x, y }];
  while (x !== to.x || y !== to.y) {
    const doubled = error * 2;
    if (doubled > -dy) {
      error -= dy;
      x += stepX;
    }
    if (doubled < dx) {
      error += dx;
      y += stepY;
    }
    points.push({ x, y });
  }
  return points;
}

/** Changes that paint `value` over `points`, skipping cells already holding it. */
export function paintChanges(
  grid: Grid,
  points: readonly Point[],
  value: ActiveColor,
): CellChange[] {
  const changes: CellChange[] = [];
  const seen = new Set<number>();
  for (const point of points) {
    if (!inBounds(grid, point)) continue;
    const index = indexOf(grid, point);
    if (seen.has(index)) continue;
    seen.add(index);
    const before = cellAtIndex(grid, index);
    if (before === value) continue;
    changes.push({ index, before, after: value });
  }
  return changes;
}

/**
 * D-ED-10: 4-connected, the same adjacency outline-infill uses (G6). The seed's
 * current value is the match value — including「空」, because pouring colour
 * into an empty region is the blank board's main use — and a seed that already
 * holds the active colour is a no-op that leaves no history entry behind.
 */
export function floodChanges(grid: Grid, seed: Point, value: ActiveColor): CellChange[] {
  if (!inBounds(grid, seed)) return [];
  const target = cellAtIndex(grid, indexOf(grid, seed));
  if (target === value) return [];

  const changes: CellChange[] = [];
  const visited = new Set<number>([indexOf(grid, seed)]);
  const queue: Point[] = [seed];
  while (queue.length > 0) {
    const point = queue.pop();
    if (point === undefined) break;
    const index = indexOf(grid, point);
    changes.push({ index, before: target, after: value });
    const neighbours: Point[] = [
      { x: point.x - 1, y: point.y },
      { x: point.x + 1, y: point.y },
      { x: point.x, y: point.y - 1 },
      { x: point.x, y: point.y + 1 },
    ];
    for (const neighbour of neighbours) {
      if (!inBounds(grid, neighbour)) continue;
      const neighbourIndex = indexOf(grid, neighbour);
      if (visited.has(neighbourIndex)) continue;
      if (cellAtIndex(grid, neighbourIndex) !== target) continue;
      visited.add(neighbourIndex);
      queue.push(neighbour);
    }
  }
  return changes;
}

/** D-ED-12: whole-document replace — v0 has no selection to scope it to. */
export function replaceChanges(grid: Grid, from: ColorIndex, to: ActiveColor): CellChange[] {
  if (from === to) return [];
  const changes: CellChange[] = [];
  grid.cells.forEach((cell, index) => {
    if (cell !== from) return;
    changes.push({ index, before: cell, after: to });
  });
  return changes;
}

export function applyChanges(grid: Grid, changes: Edit): Grid {
  if (changes.length === 0) return grid;
  const cells = [...grid.cells];
  for (const change of changes) cells[change.index] = change.after;
  return createGrid(grid.width, grid.height, cells);
}

export function revertChanges(grid: Grid, changes: Edit): Grid {
  if (changes.length === 0) return grid;
  const cells = [...grid.cells];
  for (const change of changes) cells[change.index] = change.before;
  return createGrid(grid.width, grid.height, cells);
}

/** D-ED-13: the live BOM, recomputed on every edit and never stored. */
export function editorBom(grid: Grid): BomRow[] {
  return buildBom(grid, GENERIC_5MM);
}

export interface EditorState {
  readonly grid: Grid;
  readonly tool: Tool;
  readonly activeColor: ActiveColor;
  readonly symmetry: Symmetry;
  readonly past: readonly Edit[];
  readonly future: readonly Edit[];
  /** Non-null between pointerdown and pointerup: the stroke being accumulated. */
  readonly stroke: Edit | null;
  /** Bumped on every grid change so autosave has exactly one thing to watch. */
  readonly revision: number;
}

export type EditorAction =
  | { kind: "selectTool"; tool: Tool }
  | { kind: "selectColor"; color: ActiveColor }
  | { kind: "selectSymmetry"; symmetry: Symmetry }
  | { kind: "strokeBegin"; point: Point }
  | { kind: "strokeTo"; points: readonly Point[] }
  | { kind: "strokeEnd" }
  | { kind: "fill"; point: Point }
  | { kind: "pick"; point: Point }
  | { kind: "replace"; from: ColorIndex; to: ActiveColor }
  | { kind: "undo" }
  | { kind: "redo" };

export function createEditorState(grid: Grid): EditorState {
  return {
    grid,
    tool: "brush",
    activeColor: DEFAULT_COLOR,
    symmetry: "none",
    past: [],
    future: [],
    stroke: null,
    revision: 0,
  };
}

/** A new operation makes the redo stack unreachable history, so it is dropped. */
function commit(state: EditorState, changes: Edit): EditorState {
  if (changes.length === 0) return state;
  const past = state.past.length >= MAX_HISTORY ? state.past.slice(1) : state.past;
  return {
    ...state,
    grid: applyChanges(state.grid, changes),
    past: [...past, changes],
    future: [],
    revision: state.revision + 1,
  };
}

/**
 * Two entries for the same cell inside one stroke collapse into one: the value
 * it had when the stroke started, and the value it has now. Undo has to land on
 * the board as it was before the pointer went down, not somewhere in the middle
 * of the drag.
 */
function mergeStroke(stroke: Edit, changes: Edit): CellChange[] {
  const byIndex = new Map<number, CellChange>();
  for (const change of [...stroke, ...changes]) {
    const existing = byIndex.get(change.index);
    byIndex.set(
      change.index,
      existing === undefined
        ? change
        : { index: change.index, before: existing.before, after: change.after },
    );
  }
  return [...byIndex.values()];
}

function paintStroke(state: EditorState, points: readonly Point[]): EditorState {
  const expanded = points.flatMap((point) => symmetryPoints(state.symmetry, state.grid, point));
  const changes = paintChanges(state.grid, expanded, state.activeColor);
  const stroke = mergeStroke(state.stroke ?? [], changes);
  if (changes.length === 0) return state.stroke === null ? { ...state, stroke } : state;
  return {
    ...state,
    grid: applyChanges(state.grid, changes),
    stroke,
    revision: state.revision + 1,
  };
}

export function editorReduce(state: EditorState, action: EditorAction): EditorState {
  switch (action.kind) {
    case "selectTool":
      return { ...state, tool: action.tool };
    case "selectColor":
      return { ...state, activeColor: action.color };
    case "selectSymmetry":
      return { ...state, symmetry: action.symmetry };

    // D-ED-8: a stroke is one history entry, however many cells it crossed, so
    // the pointer-down..up span accumulates into `stroke` and only lands on the
    // undo stack when the pointer is released.
    case "strokeBegin":
      return paintStroke({ ...state, stroke: [] }, [action.point]);
    case "strokeTo":
      return state.stroke === null ? state : paintStroke(state, action.points);
    case "strokeEnd": {
      if (state.stroke === null) return state;
      const changes = state.stroke.filter((change) => change.before !== change.after);
      if (changes.length === 0) return { ...state, stroke: null };
      const past = state.past.length >= MAX_HISTORY ? state.past.slice(1) : state.past;
      return { ...state, stroke: null, past: [...past, changes], future: [] };
    }

    case "fill":
      return commit(state, floodChanges(state.grid, action.point, state.activeColor));

    // D-ED-11: one shot. Picking「空」 is picking the eraser, which is the same
    // consistency that makes the eraser a colour rather than a fourth tool.
    case "pick": {
      if (!inBounds(state.grid, action.point)) return state;
      const picked = cellAtIndex(state.grid, indexOf(state.grid, action.point));
      return { ...state, activeColor: picked, tool: "brush" };
    }

    case "replace":
      return commit(state, replaceChanges(state.grid, action.from, action.to));

    case "undo": {
      const last = state.past[state.past.length - 1];
      if (last === undefined) return state;
      return {
        ...state,
        grid: revertChanges(state.grid, last),
        past: state.past.slice(0, -1),
        future: [...state.future, last],
        revision: state.revision + 1,
      };
    }
    case "redo": {
      const next = state.future[state.future.length - 1];
      if (next === undefined) return state;
      return {
        ...state,
        grid: applyChanges(state.grid, next),
        past: [...state.past, next],
        future: state.future.slice(0, -1),
        revision: state.revision + 1,
      };
    }
  }
}
