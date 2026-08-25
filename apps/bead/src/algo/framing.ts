/**
 * Board framing and resampling.
 *
 * Contract gap G5: resampling is a hand-written box (area-average) filter that
 * averages in **linear light**, with alpha-weighted colour so a transparent
 * neighbour cannot darken the edge. The browser's `drawImage` smoothing is
 * never used, because its filter is unspecified and would fork the colour codes
 * away from the Rust oracle.
 */

import { clamp, linearToSrgb, roundHalfUp, srgbToLinear } from "./color.ts";
import {
  AlgoError,
  createImage,
  isOpaque,
  OPAQUE_ALPHA_THRESHOLD,
  type RgbaImage,
} from "./image.ts";

export const BOARD_28 = 28;
export const BOARD_56 = 56;

export interface CropRect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export type Framing =
  /** Fixed board: resample the whole image onto 28×28 or 56×56. */
  | { readonly mode: "board"; readonly size: 28 | 56 }
  /** Keep the aspect ratio, longest side becomes `maxSide` (default 28). */
  | { readonly mode: "aspect"; readonly maxSide?: number }
  /** Manual viewport: crop in source pixels, then multiply by `scale`. */
  | { readonly mode: "manual"; readonly scale: number; readonly crop: CropRect }
  /** The oracle's `FitMode::FixedBoards`: a bead count, centre-cropped to fit. */
  | {
      readonly mode: "fixed-boards";
      readonly board: BoardSpec;
      readonly cols: number;
      readonly rows: number;
      readonly sampling: Sampling;
    };

export const DEFAULT_MAX_SIDE = BOARD_28;

/** Target size for aspect-fit, half-up rounded, each axis clamped to ≥ 1. */
export function aspectTarget(
  width: number,
  height: number,
  maxSide: number,
): { width: number; height: number } {
  if (width <= 0 || height <= 0) {
    throw new AlgoError("InvalidDimensions", `${width}×${height} 无法按比例适配`);
  }
  if (!Number.isFinite(maxSide) || maxSide < 1) {
    throw new AlgoError("InvalidScale", `最长边 ${maxSide} 必须 ≥ 1`);
  }
  const ratio = maxSide / Math.max(width, height);
  return {
    width: Math.max(1, roundHalfUp(width * ratio)),
    height: Math.max(1, roundHalfUp(height * ratio)),
  };
}

/**
 * Copies `rect` out of `image`. Out-of-image parts of the rect become
 * transparent (G8 / T-SCL-3); a rect entirely outside is a typed error.
 */
export function cropImage(image: RgbaImage, rect: CropRect): RgbaImage {
  if (
    !Number.isInteger(rect.x) ||
    !Number.isInteger(rect.y) ||
    !Number.isInteger(rect.width) ||
    !Number.isInteger(rect.height)
  ) {
    throw new AlgoError("InvalidCrop", "裁剪矩形必须是整数像素");
  }
  if (rect.width <= 0 || rect.height <= 0) {
    throw new AlgoError("InvalidCrop", `裁剪面积为零：${rect.width}×${rect.height}`);
  }
  const overlapW = Math.min(rect.x + rect.width, image.width) - Math.max(rect.x, 0);
  const overlapH = Math.min(rect.y + rect.height, image.height) - Math.max(rect.y, 0);
  if (overlapW <= 0 || overlapH <= 0) {
    throw new AlgoError("CropOutOfBounds", "裁剪矩形完全落在图像之外");
  }

  const out = createImage(rect.width, rect.height);
  for (let y = 0; y < rect.height; y += 1) {
    const sy = rect.y + y;
    if (sy < 0 || sy >= image.height) continue;
    for (let x = 0; x < rect.width; x += 1) {
      const sx = rect.x + x;
      if (sx < 0 || sx >= image.width) continue;
      const src = (sy * image.width + sx) * 4;
      const dst = (y * rect.width + x) * 4;
      out.data[dst] = image.data[src]!;
      out.data[dst + 1] = image.data[src + 1]!;
      out.data[dst + 2] = image.data[src + 2]!;
      out.data[dst + 3] = image.data[src + 3]!;
    }
  }
  return out;
}

/**
 * Area-average box resample. Source pixel `sx` contributes to target pixel `tx`
 * in proportion to the overlap of `[sx, sx+1)` with `[tx·sw/tw, (tx+1)·sw/tw)`.
 * Colour is averaged in linear light weighted by alpha; alpha is averaged
 * linearly on its own.
 */
export function resampleBox(image: RgbaImage, targetWidth: number, targetHeight: number): RgbaImage {
  if (!Number.isInteger(targetWidth) || !Number.isInteger(targetHeight)) {
    throw new AlgoError("InvalidDimensions", "目标尺寸必须是整数");
  }
  if (targetWidth <= 0 || targetHeight <= 0) {
    throw new AlgoError("InvalidDimensions", `目标尺寸 ${targetWidth}×${targetHeight} 必须为正`);
  }
  if (image.width <= 0 || image.height <= 0) {
    throw new AlgoError("InvalidDimensions", "源图尺寸为零");
  }

  const out = createImage(targetWidth, targetHeight);
  const scaleX = image.width / targetWidth;
  const scaleY = image.height / targetHeight;

  for (let ty = 0; ty < targetHeight; ty += 1) {
    const y0 = ty * scaleY;
    const y1 = (ty + 1) * scaleY;
    const sy0 = Math.floor(y0);
    const sy1 = Math.min(Math.ceil(y1), image.height);
    for (let tx = 0; tx < targetWidth; tx += 1) {
      const x0 = tx * scaleX;
      const x1 = (tx + 1) * scaleX;
      const sx0 = Math.floor(x0);
      const sx1 = Math.min(Math.ceil(x1), image.width);

      let areaTotal = 0;
      let alphaTotal = 0;
      let rTotal = 0;
      let gTotal = 0;
      let bTotal = 0;

      for (let sy = sy0; sy < sy1; sy += 1) {
        const wy = Math.min(sy + 1, y1) - Math.max(sy, y0);
        if (wy <= 0) continue;
        for (let sx = sx0; sx < sx1; sx += 1) {
          const wx = Math.min(sx + 1, x1) - Math.max(sx, x0);
          if (wx <= 0) continue;
          const w = wx * wy;
          const o = (sy * image.width + sx) * 4;
          const alpha = image.data[o + 3]! / 255;
          areaTotal += w;
          alphaTotal += w * alpha;
          const aw = w * alpha;
          rTotal += srgbToLinear(image.data[o]!) * aw;
          gTotal += srgbToLinear(image.data[o + 1]!) * aw;
          bTotal += srgbToLinear(image.data[o + 2]!) * aw;
        }
      }

      const dst = (ty * targetWidth + tx) * 4;
      if (areaTotal <= 0 || alphaTotal <= 0) {
        out.data[dst] = 0;
        out.data[dst + 1] = 0;
        out.data[dst + 2] = 0;
        out.data[dst + 3] = 0;
        continue;
      }
      out.data[dst] = linearToSrgb(rTotal / alphaTotal);
      out.data[dst + 1] = linearToSrgb(gTotal / alphaTotal);
      out.data[dst + 2] = linearToSrgb(bTotal / alphaTotal);
      out.data[dst + 3] = clamp(roundHalfUp((alphaTotal / areaTotal) * 255), 0, 255);
    }
  }
  return out;
}

/** Nearest-neighbour sampling at cell centres, from the target/source ratio. */
export function resampleNearest(
  image: RgbaImage,
  targetWidth: number,
  targetHeight: number,
): RgbaImage {
  if (!Number.isInteger(targetWidth) || !Number.isInteger(targetHeight)) {
    throw new AlgoError("InvalidDimensions", "目标尺寸必须是整数");
  }
  if (targetWidth <= 0 || targetHeight <= 0) {
    throw new AlgoError("InvalidDimensions", `目标尺寸 ${targetWidth}×${targetHeight} 必须为正`);
  }
  const scaleX = image.width / targetWidth;
  const scaleY = image.height / targetHeight;
  const out = createImage(targetWidth, targetHeight);
  for (let ty = 0; ty < targetHeight; ty += 1) {
    const sy = clamp(Math.floor((ty + 0.5) * scaleY), 0, image.height - 1);
    for (let tx = 0; tx < targetWidth; tx += 1) {
      const sx = clamp(Math.floor((tx + 0.5) * scaleX), 0, image.width - 1);
      copyPixel(image, sx, sy, out, tx, ty);
    }
  }
  return out;
}

/**
 * The lattice a detected up-scale describes: cell borders sit at
 * `offset + k·cell`, so a non-zero offset means the source was cropped part-way
 * through the first logical pixel.
 */
export interface CellLattice {
  readonly cellWidth: number;
  readonly cellHeight: number;
  readonly offsetX: number;
  readonly offsetY: number;
}

/**
 * Logical pixels along one axis. Both a leading remnant (`offset > 0`) and a
 * truncated trailing cell count as logical pixels: a cropped screenshot of
 * pixel art has really lost part of those cells, not the cells themselves.
 */
export function latticeCells(span: number, cell: number, offset: number): number {
  const lead = offset > 0 ? 1 : 0;
  return Math.max(1, lead + Math.ceil((span - offset) / cell));
}

/** The span `[start, end)` of source pixels logical pixel `index` covers. */
function latticeSpan(
  index: number,
  span: number,
  cell: number,
  offset: number,
): { start: number; end: number } {
  if (offset > 0 && index === 0) return { start: 0, end: Math.min(offset, span) };
  const start = offset + (index - (offset > 0 ? 1 : 0)) * cell;
  return { start, end: Math.min(start + cell, span) };
}

function copyPixel(
  source: RgbaImage,
  sx: number,
  sy: number,
  target: RgbaImage,
  tx: number,
  ty: number,
): void {
  const src = (sy * source.width + sx) * 4;
  const dst = (ty * target.width + tx) * 4;
  target.data[dst] = source.data[src]!;
  target.data[dst + 1] = source.data[src + 1]!;
  target.data[dst + 2] = source.data[src + 2]!;
  target.data[dst + 3] = source.data[src + 3]!;
}

/**
 * Undo an integer up-scale by reading one source pixel per lattice cell.
 *
 * The sample point comes from the detected geometry, never from a
 * `sourceSize / targetSize` ratio: when the last cell is truncated the two
 * disagree, and the ratio walks the sample point across cell borders until a
 * whole logical column is read from the wrong cell.
 */
export function collapseLattice(image: RgbaImage, lattice: CellLattice): RgbaImage {
  const { cellWidth, cellHeight, offsetX, offsetY } = lattice;
  if (cellWidth < 1 || cellHeight < 1) {
    throw new AlgoError("InvalidDimensions", `格尺寸 ${cellWidth}×${cellHeight} 必须 ≥ 1`);
  }
  const cols = latticeCells(image.width, cellWidth, offsetX);
  const rows = latticeCells(image.height, cellHeight, offsetY);

  const out = createImage(cols, rows);
  for (let ty = 0; ty < rows; ty += 1) {
    const row = latticeSpan(ty, image.height, cellHeight, offsetY);
    const sy = clamp(Math.floor((row.start + row.end) / 2), 0, image.height - 1);
    for (let tx = 0; tx < cols; tx += 1) {
      const column = latticeSpan(tx, image.width, cellWidth, offsetX);
      const sx = clamp(Math.floor((column.start + column.end) / 2), 0, image.width - 1);
      copyPixel(image, sx, sy, out, tx, ty);
    }
  }
  return out;
}

/**
 * A pegboard, in bead cells. Mirrors `bead_core::fit::BoardSpec` so a fixture
 * written by the oracle can be replayed here without translating its framing.
 */
export interface BoardSpec {
  readonly name: string;
  readonly width: number;
  readonly height: number;
}

/** How to read source pixels when the cell grid is coarser than the image. */
export type Sampling = "nearest" | "box-average";

/** The rectangle of source pixels that ends up on the boards, in pixel units. */
export interface SourceRect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** A framing decision: what to sample, and onto what. */
export interface FitPlan {
  readonly board: BoardSpec;
  readonly cellsWide: number;
  readonly cellsHigh: number;
  readonly boardsAcross: number;
  readonly boardsDown: number;
  readonly source: SourceRect;
  /** Share of the source image left outside `source`, 0…1. */
  readonly croppedFraction: number;
}

/** The largest centred rectangle of the given aspect that fits in the source. */
function coverRect(sourceWidth: number, sourceHeight: number, targetAspect: number): SourceRect {
  if (sourceWidth / sourceHeight > targetAspect) {
    const width = sourceHeight * targetAspect;
    return { x: (sourceWidth - width) / 2, y: 0, width, height: sourceHeight };
  }
  const height = sourceWidth / targetAspect;
  return { x: 0, y: (sourceHeight - height) / 2, width: sourceWidth, height };
}

/**
 * `bead_core::fit::fixed_boards`: a bead count the user already owns the boards
 * for, with the image centre-cropped to that shape.
 */
export function planFixedBoards(
  sourceWidth: number,
  sourceHeight: number,
  board: BoardSpec,
  cols: number,
  rows: number,
): FitPlan {
  if (sourceWidth <= 0 || sourceHeight <= 0) {
    throw new AlgoError("InvalidDimensions", `${sourceWidth}×${sourceHeight} 没有像素`);
  }
  if (board.width <= 0 || board.height <= 0 || cols <= 0 || rows <= 0) {
    throw new AlgoError("InvalidDimensions", "拼板尺寸与块数都必须为正");
  }
  const cellsWide = board.width * cols;
  const cellsHigh = board.height * rows;
  const source = coverRect(sourceWidth, sourceHeight, cellsWide / cellsHigh);

  const whole = sourceWidth * sourceHeight;
  const overlapW = Math.min(source.x + source.width, sourceWidth) - Math.max(source.x, 0);
  const overlapH = Math.min(source.y + source.height, sourceHeight) - Math.max(source.y, 0);
  const kept = clamp(Math.max(overlapW, 0) * Math.max(overlapH, 0), 0, whole);
  return {
    board,
    cellsWide,
    cellsHigh,
    boardsAcross: Math.ceil(cellsWide / board.width),
    boardsDown: Math.ceil(cellsHigh / board.height),
    source,
    croppedFraction: 1 - kept / whole,
  };
}

/** The pixel at `(x, y)`, or fully transparent outside the image. */
function samplePixel(image: RgbaImage, x: number, y: number): [number, number, number, number] {
  if (x < 0 || y < 0 || x >= image.width || y >= image.height) return [0, 0, 0, 0];
  const o = (y * image.width + x) * 4;
  return [image.data[o]!, image.data[o + 1]!, image.data[o + 2]!, image.data[o + 3]!];
}

/**
 * Mean of the covered pixels, colour averaged in linear light.
 *
 * Alpha is averaged over every covered sample and then re-thresholded, but
 * colour is averaged over the opaque samples only, so a transparent neighbour
 * cannot drag an edge cell towards whatever sits behind the alpha.
 */
function boxAverageCell(
  image: RgbaImage,
  x0: number,
  y0: number,
  cellW: number,
  cellH: number,
): [number, number, number, number] {
  const firstX = Math.floor(x0);
  const firstY = Math.floor(y0);
  const lastX = Math.max(Math.ceil(x0 + cellW) - 1, firstX);
  const lastY = Math.max(Math.ceil(y0 + cellH) - 1, firstY);

  let linearR = 0;
  let linearG = 0;
  let linearB = 0;
  let opaque = 0;
  let alphaSum = 0;
  let total = 0;

  for (let y = firstY; y <= lastY; y += 1) {
    for (let x = firstX; x <= lastX; x += 1) {
      const [r, g, b, a] = samplePixel(image, x, y);
      total += 1;
      alphaSum += a;
      if (!isOpaque(a)) continue;
      opaque += 1;
      linearR += srgbToLinear(r);
      linearG += srgbToLinear(g);
      linearB += srgbToLinear(b);
    }
  }

  if (total === 0 || opaque === 0 || alphaSum / Math.max(total, 1) < OPAQUE_ALPHA_THRESHOLD) {
    return [0, 0, 0, 0];
  }
  return [
    linearToSrgb(linearR / opaque),
    linearToSrgb(linearG / opaque),
    linearToSrgb(linearB / opaque),
    255,
  ];
}

/** `bead_core::fit::render`: sample `image` through `plan` into cell pixels. */
export function renderFit(image: RgbaImage, plan: FitPlan, sampling: Sampling): RgbaImage {
  const { cellsWide, cellsHigh, source } = plan;
  const cellW = source.width / cellsWide;
  const cellH = source.height / cellsHigh;

  const out = createImage(cellsWide, cellsHigh);
  for (let cy = 0; cy < cellsHigh; cy += 1) {
    const y0 = source.y + cy * cellH;
    for (let cx = 0; cx < cellsWide; cx += 1) {
      const x0 = source.x + cx * cellW;
      const pixel =
        sampling === "nearest"
          ? samplePixel(image, Math.floor(x0 + cellW / 2), Math.floor(y0 + cellH / 2))
          : boxAverageCell(image, x0, y0, cellW, cellH);
      const dst = (cy * cellsWide + cx) * 4;
      out.data[dst] = pixel[0];
      out.data[dst + 1] = pixel[1];
      out.data[dst + 2] = pixel[2];
      out.data[dst + 3] = pixel[3];
    }
  }
  return out;
}

/** Applies a framing choice and returns the framed RGBA buffer. */
export function applyFraming(image: RgbaImage, framing: Framing): RgbaImage {
  if (image.width <= 0 || image.height <= 0) {
    throw new AlgoError("InvalidDimensions", "源图尺寸为零");
  }
  switch (framing.mode) {
    case "board":
      return resampleBox(image, framing.size, framing.size);
    case "fixed-boards":
      return renderFit(
        image,
        planFixedBoards(image.width, image.height, framing.board, framing.cols, framing.rows),
        framing.sampling,
      );
    case "aspect": {
      const target = aspectTarget(image.width, image.height, framing.maxSide ?? DEFAULT_MAX_SIDE);
      return resampleBox(image, target.width, target.height);
    }
    case "manual": {
      if (!Number.isFinite(framing.scale) || framing.scale <= 0) {
        throw new AlgoError("InvalidScale", `缩放倍率 ${framing.scale} 必须 > 0`);
      }
      const cropped = cropImage(image, framing.crop);
      const width = Math.max(1, roundHalfUp(cropped.width * framing.scale));
      const height = Math.max(1, roundHalfUp(cropped.height * framing.scale));
      return resampleBox(cropped, width, height);
    }
  }
}
