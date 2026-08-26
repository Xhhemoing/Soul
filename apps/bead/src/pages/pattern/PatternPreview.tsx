import { type CSSProperties } from "react";

import type { FixtureGrid } from "../../fixtures/grids.ts";

/**
 * D-GAL-12 (DEV-GAL-2): the detail page's real grid preview.
 *
 * It is this page's own component, not `<AssembleCanvas>` — for the third time,
 * after `<ConversionPreview>` (D-UP-9) and `<EditorCanvas>` (D-ED-7): the canvas
 * over in `pages/assemble/` speaks in `cellStates`, which is the vocabulary of
 * an assembly session, and looking at a gallery pattern is not one. What is
 * borrowed is the rendering technique and its CSS — a DOM grid of coloured
 * spans, ≤56×56 so ≤3136 nodes on a single instance (D-ASM-6). That ceiling is
 * also why the feed keeps its grey placeholder: N cards × 3136 is a different
 * arithmetic (R-GAL-6).
 *
 * The container is `aria-hidden` and no cell is focusable or clickable. The
 * colour list below it is the text alternative and always has been: it names
 * every code, so a preview that announced「56×56 网格」would add a number, not
 * a description.
 */
export function PatternPreview({ fixture }: { fixture: FixtureGrid }) {
  const { grid, palette } = fixture;
  const style = { "--assemble-columns": String(grid.width) } as CSSProperties;

  return (
    <div className="upload__grid" data-testid="pattern-preview" aria-hidden="true" style={style}>
      {grid.cells.map((cell, index) => (
        <span
          key={index}
          className={cell === null ? "assemble__bead assemble__bead--empty" : "assemble__bead"}
          {...(cell === null ? {} : { style: { background: palette[cell]?.hex ?? "#808080" } })}
        />
      ))}
    </div>
  );
}
