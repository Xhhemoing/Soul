/**
 * PixelArt vs Photo, plus the grid extraction the pixel-art path needs.
 *
 * Rule heuristic only — no external vision API. Contract gap G3 says the grid
 * detector has to be a named, testable function rather than something the
 * browser does implicitly, so `detectGrid` is public and fixture-locked.
 *
 * BD15: the heuristic's one numeric output is named `confidence`, and the
 * intermediate it comes from is `evidence`. No other word for it is allowed in
 * this tree — `constraints.test.ts` enforces that.
 */

import { isOpaque, type RgbaImage } from "./image.ts";

export type ImageKind = "PixelArt" | "Photo";

export interface Classification {
  readonly kind: ImageKind;
  /** Always within [0,1], never NaN — including on all-transparent input. */
  readonly confidence: number;
}

export interface GridGeometry {
  readonly cellWidth: number;
  readonly cellHeight: number;
  readonly offsetX: number;
  readonly offsetY: number;
}

/** Above this many distinct colours the colour cue contributes nothing. */
export const PIXEL_ART_COLOR_CEILING = 256;
/** Cell sizes larger than this are not treated as up-scaling. */
export const MAX_DETECTED_CELL = 64;
/** Degenerate input (no opaque pixels) is `PixelArt` at exactly this confidence. */
export const DEGENERATE_CONFIDENCE = 0.5;

const WEIGHT_FLAT_NEIGHBOURS = 0.5;
const WEIGHT_GRID = 0.3;
const WEIGHT_COLORS = 0.2;
/** Evidence at or above this is PixelArt; below it is Photo. */
export const PIXEL_ART_THRESHOLD = 0.5;

function gcd(a: number, b: number): number {
  let x = Math.abs(a);
  let y = Math.abs(b);
  while (y !== 0) {
    const t = x % y;
    x = y;
    y = t;
  }
  return x;
}

/** Two pixels are "the same" only if RGBA matches byte for byte. */
function samePixel(image: RgbaImage, ai: number, bi: number): boolean {
  const { data } = image;
  const a = ai * 4;
  const b = bi * 4;
  return (
    data[a] === data[b] &&
    data[a + 1] === data[b + 1] &&
    data[a + 2] === data[b + 2] &&
    data[a + 3] === data[b + 3]
  );
}

function edgeColumns(image: RgbaImage): number[] {
  const edges: number[] = [];
  for (let x = 1; x < image.width; x += 1) {
    for (let y = 0; y < image.height; y += 1) {
      if (!samePixel(image, y * image.width + x, y * image.width + x - 1)) {
        edges.push(x);
        break;
      }
    }
  }
  return edges;
}

function edgeRows(image: RgbaImage): number[] {
  const edges: number[] = [];
  for (let y = 1; y < image.height; y += 1) {
    for (let x = 0; x < image.width; x += 1) {
      if (!samePixel(image, y * image.width + x, (y - 1) * image.width + x)) {
        edges.push(y);
        break;
      }
    }
  }
  return edges;
}

/**
 * Period = gcd of the gaps between consecutive change positions; phase = the
 * first change position modulo that period. Fewer than two change positions
 * means there is no period to infer, so the whole axis is one cell.
 */
function periodAndPhase(edges: number[], span: number): { size: number; offset: number } {
  if (edges.length < 2) return { size: span, offset: 0 };
  let period = 0;
  for (let i = 1; i < edges.length; i += 1) period = gcd(period, edges[i]! - edges[i - 1]!);
  if (period <= 0 || period > MAX_DETECTED_CELL) return { size: span, offset: 0 };
  return { size: period, offset: edges[0]! % period };
}

export function detectGrid(image: RgbaImage): GridGeometry {
  const horizontal = periodAndPhase(edgeColumns(image), image.width);
  const vertical = periodAndPhase(edgeRows(image), image.height);
  return {
    cellWidth: horizontal.size,
    cellHeight: vertical.size,
    offsetX: horizontal.offset,
    offsetY: vertical.offset,
  };
}

/** Fraction of detected cells whose pixels are all identical. */
function gridEvidence(image: RgbaImage, geometry: GridGeometry): number {
  const { cellWidth, cellHeight, offsetX, offsetY } = geometry;
  if (cellWidth <= 1 && cellHeight <= 1) return 0;

  let cells = 0;
  let uniform = 0;
  for (let y0 = offsetY - cellHeight; y0 < image.height; y0 += cellHeight) {
    for (let x0 = offsetX - cellWidth; x0 < image.width; x0 += cellWidth) {
      const xs = Math.max(x0, 0);
      const ys = Math.max(y0, 0);
      const xe = Math.min(x0 + cellWidth, image.width);
      const ye = Math.min(y0 + cellHeight, image.height);
      if (xs >= xe || ys >= ye) continue;
      cells += 1;
      const anchor = ys * image.width + xs;
      let flat = true;
      for (let y = ys; y < ye && flat; y += 1) {
        for (let x = xs; x < xe; x += 1) {
          if (!samePixel(image, y * image.width + x, anchor)) {
            flat = false;
            break;
          }
        }
      }
      if (flat) uniform += 1;
    }
  }
  return cells === 0 ? 0 : uniform / cells;
}

export interface ClassificationFeatures {
  readonly opaquePixels: number;
  readonly uniqueColors: number;
  /** Share of neighbouring pixel pairs that are byte-identical. */
  readonly flatNeighbourRatio: number;
  readonly gridEvidence: number;
  readonly geometry: GridGeometry;
}

export function classificationFeatures(image: RgbaImage): ClassificationFeatures {
  const { width, height, data } = image;
  const unique = new Set<number>();
  let opaquePixels = 0;
  for (let i = 0; i < width * height; i += 1) {
    if (!isOpaque(data[i * 4 + 3]!)) continue;
    opaquePixels += 1;
    unique.add((data[i * 4]! << 16) | (data[i * 4 + 1]! << 8) | data[i * 4 + 2]!);
  }

  let pairs = 0;
  let flat = 0;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const i = y * width + x;
      if (x + 1 < width) {
        pairs += 1;
        if (samePixel(image, i, i + 1)) flat += 1;
      }
      if (y + 1 < height) {
        pairs += 1;
        if (samePixel(image, i, i + width)) flat += 1;
      }
    }
  }

  const geometry = detectGrid(image);
  return {
    opaquePixels,
    uniqueColors: unique.size,
    // A single pixel has no neighbours; treat it as vacuously flat.
    flatNeighbourRatio: pairs === 0 ? 1 : flat / pairs,
    gridEvidence: gridEvidence(image, geometry),
    geometry,
  };
}

export function classifyImage(image: RgbaImage): Classification {
  const features = classificationFeatures(image);
  if (features.opaquePixels === 0) {
    // G8-adjacent: an all-transparent image is not an error, and dividing by
    // the opaque count here would be the obvious NaN source.
    return { kind: "PixelArt", confidence: DEGENERATE_CONFIDENCE };
  }

  const colorEvidence = Math.max(
    0,
    (PIXEL_ART_COLOR_CEILING - features.uniqueColors) / PIXEL_ART_COLOR_CEILING,
  );
  const evidence =
    WEIGHT_FLAT_NEIGHBOURS * features.flatNeighbourRatio +
    WEIGHT_GRID * features.gridEvidence +
    WEIGHT_COLORS * colorEvidence;

  const kind: ImageKind = evidence >= PIXEL_ART_THRESHOLD ? "PixelArt" : "Photo";
  const confidence = kind === "PixelArt" ? evidence : 1 - evidence;
  return { kind, confidence: Math.min(1, Math.max(0, confidence)) };
}
