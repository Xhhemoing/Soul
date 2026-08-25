import { memo, useRef, type CSSProperties, type PointerEvent } from "react";

import type { Grid } from "../../algo/grid.ts";
import type { BoardSwatch } from "../../stores/patterns.ts";
import { linePoints, type Point } from "./editor.ts";

/**
 * D-ED-7: the editor draws its own DOM grid rather than reusing
 * `<AssembleCanvas>`. The canvas over in `pages/assemble/` speaks in
 * `cellStates` — done / current / pending — which is the vocabulary of an
 * assembly session, and it is `role="img"` with every bead `aria-hidden`
 * because a session is something you look at. Borrowing the technique (≤3136
 * coloured spans, D-ASM-6) and leaving the semantics behind is the same call
 * `<ConversionPreview>` made, and it keeps the AssembleCanvas API frozen.
 *
 * Cells are plain non-focusable spans and the pointer handlers sit on the
 * container: 3136 listeners would be 3136 listeners, and 3136 tab stops would
 * be worse than the keyboard gap R-ED-1 already records.
 */
export interface EditorCanvasProps {
  readonly grid: Grid;
  readonly palette: readonly BoardSwatch[];
  readonly label: string;
  readonly onBegin: (point: Point) => void;
  /** A whole segment at once — never one dispatch per cell (R-ED-6). */
  readonly onExtend: (points: readonly Point[]) => void;
  readonly onEnd: () => void;
}

function pointOf(target: EventTarget | null): Point | null {
  if (!(target instanceof HTMLElement)) return null;
  const { x, y } = target.dataset;
  if (x === undefined || y === undefined) return null;
  return { x: Number(x), y: Number(y) };
}

export const EditorCanvas = memo(function EditorCanvas({
  grid,
  palette,
  label,
  onBegin,
  onExtend,
  onEnd,
}: EditorCanvasProps) {
  const last = useRef<Point | null>(null);
  const style = { "--assemble-columns": String(grid.width) } as CSSProperties;

  function begin(event: PointerEvent<HTMLDivElement>): void {
    const point = pointOf(event.target);
    if (point === null) return;
    event.preventDefault();
    // Capture keeps the stroke alive when the pointer leaves the board mid-drag.
    if (typeof event.currentTarget.setPointerCapture === "function") {
      event.currentTarget.setPointerCapture(event.pointerId);
    }
    last.current = point;
    onBegin(point);
  }

  function extend(event: PointerEvent<HTMLDivElement>): void {
    const from = last.current;
    if (from === null) return;
    const point = pointOf(event.target);
    if (point === null || (point.x === from.x && point.y === from.y)) return;
    last.current = point;
    // A fast drag skips cells between two pointer events; the segment between
    // them is filled in here so the stroke has no holes.
    onExtend(linePoints(from, point));
  }

  function end(): void {
    if (last.current === null) return;
    last.current = null;
    onEnd();
  }

  return (
    <div
      className="editor__grid"
      role="img"
      aria-label={label}
      data-testid="editor-grid"
      style={style}
      onPointerDown={begin}
      onPointerMove={extend}
      onPointerUp={end}
      onPointerCancel={end}
    >
      {grid.cells.map((cell, index) => (
        <span
          key={index}
          className={cell === null ? "editor__bead editor__bead--empty" : "editor__bead"}
          data-x={index % grid.width}
          data-y={Math.floor(index / grid.width)}
          aria-hidden="true"
          {...(cell === null ? {} : { style: { background: palette[cell]?.hex ?? "#808080" } })}
        />
      ))}
    </div>
  );
});
