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
import { AlgoError, createImage, type RgbaImage } from "./image.ts";

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
  | { readonly mode: "manual"; readonly scale: number; readonly crop: CropRect };

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

/** Nearest-neighbour sampling; only used to undo an integer pixel-art up-scale. */
export function resampleNearest(
  image: RgbaImage,
  targetWidth: number,
  targetHeight: number,
  offsetX = 0,
  offsetY = 0,
): RgbaImage {
  const out = createImage(targetWidth, targetHeight);
  const scaleX = image.width / targetWidth;
  const scaleY = image.height / targetHeight;
  for (let ty = 0; ty < targetHeight; ty += 1) {
    const sy = clamp(Math.floor(ty * scaleY) + offsetY, 0, image.height - 1);
    for (let tx = 0; tx < targetWidth; tx += 1) {
      const sx = clamp(Math.floor(tx * scaleX) + offsetX, 0, image.width - 1);
      const src = (sy * image.width + sx) * 4;
      const dst = (ty * targetWidth + tx) * 4;
      out.data[dst] = image.data[src]!;
      out.data[dst + 1] = image.data[src + 1]!;
      out.data[dst + 2] = image.data[src + 2]!;
      out.data[dst + 3] = image.data[src + 3]!;
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
