/**
 * Image → bead pattern, end to end.
 *
 * Order of operations is part of the contract, because reordering framing and
 * quantisation changes the colour codes:
 *   1. classify (or take the caller's override);
 *   2. on the PixelArt path, undo an integer up-scale via `detectGrid` first,
 *      so framing does not average across cell borders;
 *   3. frame (board / aspect / manual);
 *   4. quantise, optionally with Floyd–Steinberg;
 *   5. BOM.
 *
 * Dithering is force-disabled on the PixelArt path: a pixel-art source already
 * has flat cells, and diffusing error across them destroys the very structure
 * the path exists to preserve. The result reports what was actually applied.
 */

import { buildBom, type BomRow } from "./bom.ts";
import {
  classifyImage,
  detectGrid,
  type Classification,
  type GridGeometry,
  type ImageKind,
} from "./classify.ts";
import { quantize } from "./dither.ts";
import { applyFraming, collapseLattice, type Framing } from "./framing.ts";
import type { Grid } from "./grid.ts";
import { AlgoError, type RgbaImage } from "./image.ts";
import { GENERIC_5MM, type Palette } from "./palette.ts";

export interface PipelineOptions {
  readonly framing: Framing;
  /** Requested; ignored on the PixelArt path. Defaults to off. */
  readonly dither?: boolean;
  readonly palette?: Palette;
  /** Force a path instead of running the heuristic. */
  readonly kind?: ImageKind;
}

export interface PipelineResult {
  readonly grid: Grid;
  readonly bom: readonly BomRow[];
  readonly palette: Palette;
  readonly classification: Classification;
  readonly kind: ImageKind;
  /** True only when the caller asked for dithering *and* the path allows it. */
  readonly ditherApplied: boolean;
  /** Present when the PixelArt path undid an up-scale. */
  readonly detectedGrid: GridGeometry | null;
  /** Smallest runner-up ΔE margin over all quantised pixels (T-PAR-3 sentinel). */
  readonly minRunnerUpMargin: number;
}

/** Undo an integer pixel-art up-scale, sampling each detected cell once. */
function collapseUpscale(image: RgbaImage, geometry: GridGeometry): RgbaImage {
  const { cellWidth, cellHeight } = geometry;
  if (cellWidth <= 1 && cellHeight <= 1) return image;
  if (cellWidth >= image.width && cellHeight >= image.height) return image;
  return collapseLattice(image, geometry);
}

export function imageToPattern(image: RgbaImage, options: PipelineOptions): PipelineResult {
  if (image.width <= 0 || image.height <= 0) {
    throw new AlgoError("InvalidDimensions", "源图尺寸为零");
  }
  const palette = options.palette ?? GENERIC_5MM;
  if (palette.entries.length === 0) {
    throw new AlgoError("EmptyPalette", `色板 ${palette.id} 没有条目`);
  }

  const classification = classifyImage(image);
  const kind = options.kind ?? classification.kind;

  let working = image;
  let detectedGrid: GridGeometry | null = null;
  if (kind === "PixelArt") {
    const geometry = detectGrid(image);
    const collapsed = collapseUpscale(image, geometry);
    if (collapsed !== image) {
      detectedGrid = geometry;
      working = collapsed;
    }
  }

  const framed = applyFraming(working, options.framing);
  const ditherApplied = kind === "Photo" && options.dither === true;
  const { grid, minRunnerUpMargin } = quantize(framed, { palette, dither: ditherApplied });

  return {
    grid,
    bom: buildBom(grid, palette),
    palette,
    classification,
    kind,
    ditherApplied,
    detectedGrid,
    minRunnerUpMargin,
  };
}
