/**
 * png/jpg → raw RGBA.
 *
 * T-PAR-2: the browser is allowed to apply an embedded ICC profile and to
 * resample during `drawImage`, and either one silently forks the colour codes
 * away from the Rust oracle. So decoding pins two things:
 *   - `colorSpaceConversion: "none"` on `createImageBitmap`, i.e. bytes are
 *     taken as-is and treated as sRGB code values;
 *   - the bitmap is drawn 1:1 at its natural size with `imageSmoothingEnabled`
 *     off, so no resampling happens here. All scaling belongs to `framing.ts`.
 *
 * Parity fixtures never come through this path — they are raw RGBA arrays.
 */

import { AlgoError, createImage, type RgbaImage } from "./image.ts";

type BitmapSource = Parameters<typeof createImageBitmap>[0];

interface Canvas2d {
  readonly width: number;
  readonly height: number;
  getContext(id: "2d"): CanvasRenderingContext2D | null;
}

function makeCanvas(width: number, height: number): Canvas2d {
  if (typeof OffscreenCanvas !== "undefined") {
    return new OffscreenCanvas(width, height) as unknown as Canvas2d;
  }
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  return canvas;
}

export const DECODE_OPTIONS = {
  colorSpaceConversion: "none",
  premultiplyAlpha: "none",
} as const satisfies ImageBitmapOptions;

/** Decodes an uploaded png/jpg blob into raw, unresampled RGBA. */
export async function decodeImage(source: BitmapSource): Promise<RgbaImage> {
  const bitmap = await createImageBitmap(source, DECODE_OPTIONS);
  try {
    if (bitmap.width <= 0 || bitmap.height <= 0) {
      throw new AlgoError("InvalidDimensions", "解码得到零尺寸图像");
    }
    const canvas = makeCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext("2d");
    if (context === null) {
      throw new AlgoError("InvalidDimensions", "无法取得 2D 绘图上下文");
    }
    context.imageSmoothingEnabled = false;
    context.drawImage(bitmap as unknown as CanvasImageSource, 0, 0);
    const pixels = context.getImageData(0, 0, bitmap.width, bitmap.height);
    return createImage(bitmap.width, bitmap.height, pixels.data);
  } finally {
    bitmap.close();
  }
}
