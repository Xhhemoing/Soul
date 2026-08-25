import { type CSSProperties } from "react";

import type { BomRow } from "../../algo/bom.ts";
import type { Grid } from "../../algo/grid.ts";
import { occupiedCount } from "../../algo/grid.ts";
import type { Palette } from "../../algo/palette.ts";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { rgbToHex } from "../../stores/patterns.ts";
import { GENERIC_5MM_PALETTE } from "../../stores/types.ts";

/**
 * D-UP-9: the conversion preview is this page's own component, not
 * `<AssembleCanvas>`. The canvas speaks in `cellStates` — done / current /
 * pending — which is the vocabulary of an assembly session, and there is no
 * session here. It borrows the rendering technique (a DOM grid of coloured
 * spans, ≤56×56 so ≤3136 nodes) and nothing else.
 *
 * Nothing on this panel is ever written: it is derived from the file in the
 * input and the parameters above it, and it disappears when either changes.
 */
export interface ConversionPreviewProps {
  readonly grid: Grid;
  readonly bom: readonly BomRow[];
  readonly palette: Palette;
  readonly kindLabel: string;
  readonly confidence: number;
  readonly ditherApplied: boolean;
}

export function ConversionPreview({
  grid,
  bom,
  palette,
  kindLabel,
  confidence,
  ditherApplied,
}: ConversionPreviewProps) {
  const style = { "--assemble-columns": String(grid.width) } as CSSProperties;
  const beads = occupiedCount(grid);

  return (
    <div className="upload__preview">
      <ul className="upload__badges">
        <li>路径：{kindLabel}</li>
        <li>置信度：{Math.round(confidence * 100)}%</li>
        <li>抖动：{ditherApplied ? "已应用" : "未应用"}</li>
        <li>
          网格：{grid.width}×{grid.height} · {beads} 颗
        </li>
      </ul>

      <div
        className="upload__grid"
        role="img"
        aria-label={`转换预览：${grid.width}×${grid.height} 网格，共 ${beads} 颗豆`}
        data-testid="conversion-grid"
        style={style}
      >
        {grid.cells.map((cell, index) => (
          <span
            key={index}
            className={cell === null ? "assemble__bead assemble__bead--empty" : "assemble__bead"}
            aria-hidden="true"
            {...(cell === null
              ? {}
              : { style: { background: rgbToHex(palette.entries[cell]!.rgb) } })}
          />
        ))}
      </div>

      <ul className="upload__bom" aria-label="用色清单">
        {bom.map((row) => (
          <li key={row.code}>
            <ColorSwatch
              paletteId={GENERIC_5MM_PALETTE}
              code={row.code}
              name={row.displayName}
              hex={rgbToHex(palette.entries[row.index]!.rgb)}
              beads={row.count}
            />
          </li>
        ))}
      </ul>
    </div>
  );
}
