/**
 * Raw RGBA buffers and the typed errors the pipeline raises instead of
 * panicking (contract gap G8).
 */

import type { Rgb } from "./color.ts";

export type AlgoErrorCode =
  | "InvalidDimensions"
  | "InvalidScale"
  | "InvalidCrop"
  | "CropOutOfBounds"
  | "EmptyPalette";

export class AlgoError extends Error {
  readonly code: AlgoErrorCode;

  constructor(code: AlgoErrorCode, message: string) {
    super(`${code}: ${message}`);
    this.name = "AlgoError";
    this.code = code;
  }
}

/** Row-major RGBA, 4 bytes per pixel — the same layout as `ImageData`. */
export interface RgbaImage {
  readonly width: number;
  readonly height: number;
  readonly data: Uint8ClampedArray;
}

/**
 * G1: alpha ≥ 128 is opaque, everything below is an empty cell. Empty cells
 * stay out of the BOM, out of every step, and out of the dither error flow.
 */
export const OPAQUE_ALPHA_THRESHOLD = 128;

export function isOpaque(alpha: number): boolean {
  return alpha >= OPAQUE_ALPHA_THRESHOLD;
}

export function createImage(
  width: number,
  height: number,
  data?: ArrayLike<number>,
): RgbaImage {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width <= 0 || height <= 0) {
    throw new AlgoError("InvalidDimensions", `${width}×${height} 不是正整数尺寸`);
  }
  const expected = width * height * 4;
  if (data !== undefined && data.length !== expected) {
    throw new AlgoError(
      "InvalidDimensions",
      `RGBA 长度 ${data.length} 与 ${width}×${height} 需要的 ${expected} 不符`,
    );
  }
  const buffer = new Uint8ClampedArray(expected);
  if (data !== undefined) buffer.set(data);
  return { width, height, data: buffer };
}

/** Builds an image from `width * height` RGBA quadruples given as flat rows. */
export function imageFromPixels(
  width: number,
  height: number,
  pixel: (x: number, y: number) => readonly [number, number, number, number],
): RgbaImage {
  const image = createImage(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = pixel(x, y);
      const o = (y * width + x) * 4;
      image.data[o] = r;
      image.data[o + 1] = g;
      image.data[o + 2] = b;
      image.data[o + 3] = a;
    }
  }
  return image;
}

export function pixelOffset(image: RgbaImage, x: number, y: number): number {
  return (y * image.width + x) * 4;
}

export function readRgb(image: RgbaImage, x: number, y: number): Rgb {
  const o = pixelOffset(image, x, y);
  return { r: image.data[o]!, g: image.data[o + 1]!, b: image.data[o + 2]! };
}

export function readAlpha(image: RgbaImage, x: number, y: number): number {
  return image.data[pixelOffset(image, x, y) + 3]!;
}
