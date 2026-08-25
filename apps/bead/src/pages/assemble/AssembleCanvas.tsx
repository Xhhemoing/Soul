import { memo, type CSSProperties } from "react";

import type { Grid } from "../../algo/grid.ts";
import type { CellState } from "./session.ts";

/**
 * D-ASM-6: a DOM grid, not `<canvas>`. At ≤56×56 that is ≤3136 nodes, which the
 * browser handles and which testing-library can actually assert on. When 512²
 * boards arrive (post-B07) the pixel renderer swaps in here and nowhere else.
 *
 * The palette prop is deliberately `{code, hex}`-shaped and this file does not
 * import the catalog: once WP-B03 feeds real conversions in, they arrive
 * through the same door with a `generic-5mm` palette and nothing here changes.
 */
export interface AssembleCanvasProps {
  readonly grid: Grid;
  readonly palette: readonly { readonly code: string; readonly hex: string }[];
  readonly cellStates: readonly (CellState | undefined)[];
  readonly label: string;
  /** D-ASM-10: 5mm cell pitch via CSS `mm`. */
  readonly physical: boolean;
  /** Derived from the backdrop luminance, never from a theme token (D-UI-6). */
  readonly outlineColor: string;
}

export const AssembleCanvas = memo(function AssembleCanvas({
  grid,
  palette,
  cellStates,
  label,
  physical,
  outlineColor,
}: AssembleCanvasProps) {
  const style = {
    "--assemble-columns": String(grid.width),
    "--assemble-outline": outlineColor,
  } as CSSProperties;

  return (
    <div
      className="assemble__grid"
      role="img"
      aria-label={label}
      data-testid="assemble-grid"
      {...(physical ? { "data-scale": "physical" as const } : {})}
      style={style}
    >
      {grid.cells.map((cell, index) => {
        const state = cellStates[index];
        if (cell === null || state === undefined) {
          return <span key={index} className="assemble__bead assemble__bead--empty" aria-hidden="true" />;
        }
        // The grid is authoritative but a fixture can still be edited by hand;
        // an index past the palette renders visibly rather than crashing.
        const hex = palette[cell]?.hex ?? "#808080";
        return (
          <span
            key={index}
            className="assemble__bead"
            data-cell-state={state}
            aria-hidden="true"
            style={{ background: hex }}
          />
        );
      })}
    </div>
  );
});
