/**
 * The assemble session's whole brain, with no DOM in it. Everything here is
 * derived from `Step[]` on demand — D-ASM-1 lets exactly five scalars reach
 * storage, so progress percentages, BPM and milestones are recomputed rather
 * than remembered.
 */

import type { Grid } from "../../algo/grid.ts";
import { splitSteps, type SplitMode, type Step } from "../../algo/steps.ts";
import type { ProgressCursor } from "../../stores/types.ts";

export const DEFAULT_MODE: SplitMode = "color-by-color";

export const MODE_LABEL: Record<SplitMode, string> = {
  "color-by-color": "单色推进",
  tile: "分块推进",
  "outline-infill": "描边填充",
  "row-by-row": "逐行扫描",
};

export const PART_LABEL: Record<Step["part"], string> = {
  all: "整块",
  outline: "描边",
  "inner-edge": "内边界",
  fill: "填充",
};

/** D-ASM-9: crossed upward once each, never replayed on restore or on undo. */
export const MILESTONES: readonly number[] = [25, 50, 75, 100];

export interface SessionState {
  readonly mode: SplitMode;
  readonly stepIndex: number;
  readonly elapsedMs: number;
  /** D-ASM-7: a row window inside the current step. Session-only, never stored. */
  readonly rowLock: boolean;
  readonly rowSlice: number;
  /** D-ASM-10: 1:1 CSS-mm rendering. Session-only. */
  readonly physical: boolean;
  readonly announcedMilestones: readonly number[];
  readonly announcement: string;
}

export type SessionAction =
  | { readonly kind: "advance" }
  | { readonly kind: "undo" }
  | { readonly kind: "switchMode"; readonly mode: SplitMode }
  | { readonly kind: "toggleRowLock" }
  | { readonly kind: "togglePhysical" }
  | { readonly kind: "tick"; readonly deltaMs: number };

export interface SessionStats {
  readonly totalCells: number;
  readonly placedCells: number;
  /** 0…1, and 0 for an empty pattern rather than NaN. */
  readonly fraction: number;
  readonly percent: number;
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), high);
}

export function totalCells(steps: readonly Step[]): number {
  return steps.reduce((total, step) => total + step.cells.length, 0);
}

/** Distinct y values of a step, ascending — the row window walks this list. */
export function stepRows(step: Step | undefined): number[] {
  if (step === undefined) return [];
  const seen = new Set<number>();
  for (const cell of step.cells) seen.add(cell.y);
  return [...seen].sort((a, b) => a - b);
}

/** Cells of the step that sit above the row window, i.e. already placed. */
export function cellsBeforeSlice(step: Step | undefined, rowSlice: number): number {
  if (step === undefined || rowSlice <= 0) return 0;
  const rows = stepRows(step);
  const boundary = rows[Math.min(rowSlice, rows.length - 1)];
  if (boundary === undefined) return 0;
  return step.cells.reduce<number>((total, cell) => (cell.y < boundary ? total + 1 : total), 0);
}

export function sessionStats(
  steps: readonly Step[],
  stepIndex: number,
  cellsDoneInStep = 0,
): SessionStats {
  const total = totalCells(steps);
  const done = steps
    .slice(0, clamp(stepIndex, 0, steps.length))
    .reduce((sum, step) => sum + step.cells.length, 0);
  const placed = Math.min(done + Math.max(cellsDoneInStep, 0), total);
  const fraction = total === 0 ? 0 : placed / total;
  return { totalCells: total, placedCells: placed, fraction, percent: Math.round(fraction * 100) };
}

/**
 * D-ASM-9. Null rather than 0 while there is nothing to divide by: a rate
 * computed from the first half second is noise, and showing it as a number
 * invites the user to read it as a real pace.
 */
export function beadsPerMinute(placedCells: number, elapsedMs: number): number | null {
  if (placedCells <= 0 || elapsedMs < 1000) return null;
  return Math.round((placedCells * 60_000) / elapsedMs);
}

/** Milestones already reached at this fraction — what a restore starts from. */
export function reachedMilestones(percent: number): number[] {
  return MILESTONES.filter((milestone) => percent >= milestone);
}

export function formatDuration(elapsedMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1000));
  const seconds = totalSeconds % 60;
  const minutes = Math.floor(totalSeconds / 60) % 60;
  const hours = Math.floor(totalSeconds / 3600);
  const pad = (value: number) => value.toString().padStart(2, "0");
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${pad(minutes)}:${pad(seconds)}`;
}

const SPLIT_CACHE = new WeakMap<Grid, Map<SplitMode, readonly Step[]>>();

/**
 * `splitSteps` is a pure function of (grid, mode) and there are four modes, so
 * each split is computed at most once per board. Without this the reducer would
 * re-partition a 56×56 grid on every action — including every timer tick.
 */
export function stepsOf(grid: Grid, mode: SplitMode): readonly Step[] {
  let byMode = SPLIT_CACHE.get(grid);
  if (byMode === undefined) {
    byMode = new Map();
    SPLIT_CACHE.set(grid, byMode);
  }
  const cached = byMode.get(mode);
  if (cached !== undefined) return cached;
  const computed = splitSteps(grid, mode);
  byMode.set(mode, computed);
  return computed;
}

export interface SessionContext {
  readonly steps: readonly Step[];
  /** Palette codes, index-aligned with `Step.color`. Only used for wording. */
  readonly paletteCodes: readonly string[];
}

export function createSessionState(
  cursor: Pick<ProgressCursor, "mode" | "stepIndex" | "elapsedMs"> | undefined,
  steps: readonly Step[],
): SessionState {
  const stepIndex = clamp(Math.floor(cursor?.stepIndex ?? 0), 0, steps.length);
  return {
    mode: cursor?.mode ?? DEFAULT_MODE,
    stepIndex,
    elapsedMs: Math.max(0, cursor?.elapsedMs ?? 0),
    rowLock: false,
    rowSlice: 0,
    physical: false,
    // Restoring must not replay what the previous session already announced.
    announcedMilestones: reachedMilestones(sessionStats(steps, stepIndex).percent),
    announcement: "",
  };
}

/** The row window only means anything when a step holds more than one row. */
export function rowLockActive(state: Pick<SessionState, "rowLock" | "mode">): boolean {
  return state.rowLock && state.mode !== "row-by-row";
}

function announceFor(
  state: SessionState,
  context: SessionContext,
  completedStep: Step | undefined,
  nextPercent: number,
): Pick<SessionState, "announcedMilestones" | "announcement"> {
  const crossed = reachedMilestones(nextPercent).filter(
    (milestone) => !state.announcedMilestones.includes(milestone),
  );
  const parts: string[] = [];
  // D-ASM-9: only the single-colour mode has a colour to finish; every other
  // mode leaves `step.color` null and must stay quiet.
  if (completedStep !== undefined && completedStep.color !== null) {
    const code = context.paletteCodes[completedStep.color];
    if (code !== undefined) parts.push(`色号 ${code} 完成`);
  }
  const highest = crossed[crossed.length - 1];
  if (highest !== undefined) parts.push(`已完成 ${highest}%`);
  return {
    announcedMilestones:
      crossed.length === 0 ? state.announcedMilestones : [...state.announcedMilestones, ...crossed],
    announcement: parts.length === 0 ? state.announcement : parts.join(" · "),
  };
}

export function sessionReduce(
  state: SessionState,
  action: SessionAction,
  context: SessionContext,
): SessionState {
  const { steps } = context;
  switch (action.kind) {
    case "advance": {
      if (state.stepIndex >= steps.length) return state;
      const step = steps[state.stepIndex];
      if (rowLockActive(state)) {
        const rows = stepRows(step);
        if (state.rowSlice + 1 < rows.length) {
          const nextSlice = state.rowSlice + 1;
          const percent = sessionStats(
            steps,
            state.stepIndex,
            cellsBeforeSlice(step, nextSlice),
          ).percent;
          return { ...state, rowSlice: nextSlice, ...announceFor(state, context, undefined, percent) };
        }
      }
      const stepIndex = state.stepIndex + 1;
      const percent = sessionStats(steps, stepIndex).percent;
      return {
        ...state,
        stepIndex,
        rowSlice: 0,
        ...announceFor(state, context, step, percent),
      };
    }
    case "undo": {
      // Undo is always a whole step back once the row window is at its top:
      // the persisted granularity is the step (round2-data §3), so a finer
      // rewind would not survive a refresh anyway.
      if (rowLockActive(state) && state.rowSlice > 0) {
        return { ...state, rowSlice: state.rowSlice - 1, announcement: "" };
      }
      if (state.stepIndex === 0) return state;
      return { ...state, stepIndex: state.stepIndex - 1, rowSlice: 0, announcement: "" };
    }
    case "switchMode": {
      if (action.mode === state.mode) return state;
      // D-ASM-4: stepIndex has no meaning across modes, so it resets; the
      // elapsed time belongs to the project and carries over untouched.
      return {
        ...state,
        mode: action.mode,
        stepIndex: 0,
        rowSlice: 0,
        announcedMilestones: [],
        announcement: "",
      };
    }
    case "toggleRowLock":
      return { ...state, rowLock: !state.rowLock, rowSlice: 0 };
    case "togglePhysical":
      return { ...state, physical: !state.physical };
    case "tick":
      return { ...state, elapsedMs: state.elapsedMs + Math.max(0, action.deltaMs) };
  }
}

/** D-ASM-5. Empty cells get no state at all — they are not beads. */
export type CellState = "done" | "current" | "pending";

/** The y the row window is parked on, or null when the whole step is current. */
export function currentRowOf(
  state: Pick<SessionState, "rowLock" | "mode" | "rowSlice">,
  step: Step | undefined,
): number | null {
  if (!rowLockActive(state)) return null;
  const rows = stepRows(step);
  return rows[Math.min(state.rowSlice, rows.length - 1)] ?? null;
}

/**
 * One pass over the board per cursor move. Kept out of the component so the
 * result can be memoised and the canvas skipped entirely on a timer tick
 * (R-ASM-2: 3136 nodes must not reflow once a second).
 */
export function computeCellStates(
  grid: Grid,
  steps: readonly Step[],
  stepIndex: number,
  currentRow: number | null,
): (CellState | undefined)[] {
  const states = grid.cells.map<CellState | undefined>((cell) =>
    cell === null ? undefined : "pending",
  );
  const at = (x: number, y: number) => y * grid.width + x;
  for (const step of steps.slice(0, clamp(stepIndex, 0, steps.length))) {
    for (const cell of step.cells) states[at(cell.x, cell.y)] = "done";
  }
  const step = steps[stepIndex];
  if (step !== undefined) {
    for (const cell of step.cells) {
      const index = at(cell.x, cell.y);
      if (currentRow === null || cell.y === currentRow) states[index] = "current";
      else states[index] = cell.y < currentRow ? "done" : "pending";
    }
  }
  return states;
}

/** What the HUD and the canvas label say about where the user is. */
export function describeStep(step: Step | undefined, palette: readonly string[]): string {
  if (step === undefined) return "全部步骤已完成";
  switch (step.mode) {
    case "color-by-color": {
      const code = step.color === null ? undefined : palette[step.color];
      return code === undefined ? "整块" : `色号 ${code}`;
    }
    case "row-by-row":
      return `第 ${step.group + 1} 行`;
    case "tile":
      return `第 ${step.group + 1} 块`;
    case "outline-infill":
      return `第 ${step.group + 1} 块 · ${PART_LABEL[step.part]}`;
  }
}
