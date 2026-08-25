import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { useNavigate } from "react-router";

import type { SplitMode } from "../../algo/steps.ts";
import type { BoardSource } from "../../stores/patterns.ts";
import { asProjectId } from "../../stores/ids.ts";
import { useStore } from "../../stores/store.tsx";
import type { ProgressCursor, Project } from "../../stores/types.ts";
import { AssembleCanvas } from "./AssembleCanvas.tsx";
import { ControlOrb } from "./ControlOrb.tsx";
import { SessionHUD } from "./SessionHUD.tsx";
import {
  DEFAULT_MODE,
  beadsPerMinute,
  cellsBeforeSlice,
  computeCellStates,
  createSessionState,
  currentRowOf,
  describeStep,
  formatDuration,
  rowLockActive,
  sessionReduce,
  sessionStats,
  stepsOf,
  type SessionAction,
  type SessionState,
} from "./session.ts";

const TICK_MS = 1000;

/** Arrow keys are global, so a control that owns them keeps them (D-ASM-13). */
function ownsArrowKeys(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);
}

export interface AssembleSessionProps {
  readonly project: Project;
  /** A gallery fixture or a converted `PatternDoc` — same shape either way (D-UP-14). */
  readonly board: BoardSource;
  readonly cursor: ProgressCursor | undefined;
  readonly outlineColor: string;
}

export function AssembleSession({ project, board, cursor, outlineColor }: AssembleSessionProps) {
  const { grid, palette } = board;
  const navigate = useNavigate();
  const { setProjectStatus, upsertProgress } = useStore();
  const projectId = asProjectId(project.id);

  const paletteCodes = useMemo(() => palette.map((swatch) => swatch.code), [palette]);
  const paletteLabels = useMemo(
    () => palette.map((swatch) => `${swatch.code} ${swatch.name}`),
    [palette],
  );

  const [state, dispatch] = useReducer(
    (current: SessionState, action: SessionAction) =>
      sessionReduce(current, action, { steps: stepsOf(grid, current.mode), paletteCodes }),
    undefined,
    () => createSessionState(cursor, stepsOf(grid, cursor?.mode ?? DEFAULT_MODE)),
  );

  const steps = stepsOf(grid, state.mode);
  const step = steps[state.stepIndex];
  const complete = state.stepIndex >= steps.length;
  const stats = sessionStats(steps, state.stepIndex, cellsBeforeSlice(step, state.rowSlice));
  const stepDescription = describeStep(step, paletteLabels);
  const label = complete
    ? `拼装网格：全部 ${steps.length} 步已完成`
    : `拼装网格：第 ${state.stepIndex + 1}/${steps.length} 步 · ${stepDescription}`;

  // Recomputed only when the cursor actually moves: a once-a-second tick must
  // leave this identity alone so the memoised canvas skips its 3136 nodes.
  const { mode, stepIndex, rowLock, rowSlice } = state;
  const cellStates = useMemo(() => {
    const modeSteps = stepsOf(grid, mode);
    const row = currentRowOf({ mode, rowLock, rowSlice }, modeSteps[stepIndex]);
    return computeCellStates(grid, modeSteps, stepIndex, row);
  }, [grid, mode, stepIndex, rowLock, rowSlice]);

  // The cursor that reaches storage is assembled from state only at the write
  // moments below — never on a tick (D-ASM-8).
  const latest = useRef(state);
  useEffect(() => {
    latest.current = state;
  }, [state]);

  const milestoneCount = state.announcedMilestones.length;
  useEffect(() => {
    upsertProgress({ projectId, mode, stepIndex, elapsedMs: latest.current.elapsedMs });
  }, [upsertProgress, projectId, mode, stepIndex, milestoneCount]);

  useEffect(() => {
    const flush = () => {
      const snapshot = latest.current;
      upsertProgress({
        projectId,
        mode: snapshot.mode,
        stepIndex: snapshot.stepIndex,
        elapsedMs: snapshot.elapsedMs,
      });
    };
    window.addEventListener("pagehide", flush);
    return () => {
      window.removeEventListener("pagehide", flush);
      flush();
    };
  }, [upsertProgress, projectId]);

  const [pageVisible, setPageVisible] = useState(() => !document.hidden);
  useEffect(() => {
    const onVisibilityChange = () => setPageVisible(!document.hidden);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => document.removeEventListener("visibilitychange", onVisibilityChange);
  }, []);

  useEffect(() => {
    if (!pageVisible || complete) return undefined;
    const timer = setInterval(() => dispatch({ kind: "tick", deltaMs: TICK_MS }), TICK_MS);
    return () => clearInterval(timer);
  }, [pageVisible, complete]);

  const advance = useCallback(() => dispatch({ kind: "advance" }), []);
  const undo = useCallback(() => dispatch({ kind: "undo" }), []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
      if (ownsArrowKeys(event.target)) return;
      event.preventDefault();
      dispatch(event.key === "ArrowRight" ? { kind: "advance" } : { kind: "undo" });
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const finish = () => {
    setProjectStatus(projectId, "done");
    void navigate("/workspace");
  };

  return (
    <div className="assemble__session">
      <SessionHUD
        elapsedMs={state.elapsedMs}
        beadsPerMinute={beadsPerMinute(stats.placedCells, state.elapsedMs)}
        stepNumber={Math.min(state.stepIndex + 1, steps.length)}
        stepTotal={steps.length}
        percent={stats.percent}
        stepDescription={complete ? "全部步骤已完成" : stepDescription}
        mode={state.mode}
        onModeChange={(next: SplitMode) => dispatch({ kind: "switchMode", mode: next })}
        physical={state.physical}
        onTogglePhysical={() => dispatch({ kind: "togglePhysical" })}
        announcement={state.announcement}
      />

      <AssembleCanvas
        grid={grid}
        palette={palette}
        cellStates={cellStates}
        label={label}
        physical={state.physical}
        outlineColor={outlineColor}
      />

      {complete && (
        <section className="assemble__complete" aria-label="完成">
          <h2>这张拼完了</h2>
          <p>
            总用时 {formatDuration(state.elapsedMs)} · 总颗数 {stats.totalCells}
          </p>
          <button type="button" className="button button--primary" onClick={finish}>
            标记完工
          </button>
        </section>
      )}

      <ControlOrb
        onNext={advance}
        onUndo={undo}
        onToggleRowLock={() => dispatch({ kind: "toggleRowLock" })}
        canAdvance={!complete}
        canUndo={state.stepIndex > 0 || (rowLockActive(state) && state.rowSlice > 0)}
        rowLock={state.rowLock}
        rowLockUnavailable={state.mode === "row-by-row"}
      />
    </div>
  );
}
