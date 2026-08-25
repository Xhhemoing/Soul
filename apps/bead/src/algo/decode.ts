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

/**
 * DATA-3: the source ceiling, in pixels per axis. 4096×4096 is 64MB of RGBA,
 * which is the largest allocation this path will make on a caller's behalf; a
 * 20000×20000 PNG header would otherwise ask for 1.6GB before anyone can say
 * no. The gate lives here rather than in the upload page so that WP-B07's
 * import path inherits it by construction.
 */
export const MAX_SOURCE_SIDE = 4096;

/** Decodes an uploaded png/jpg blob into raw, unresampled RGBA. */
export async function decodeImage(source: BitmapSource): Promise<RgbaImage> {
  const bitmap = await createImageBitmap(source, DECODE_OPTIONS);
  try {
    if (bitmap.width <= 0 || bitmap.height <= 0) {
      throw new AlgoError("InvalidDimensions", "解码得到零尺寸图像");
    }
    // Checked off the bitmap header, before the canvas is sized and before
    // `drawImage`: past this point the allocation has already happened.
    if (bitmap.width > MAX_SOURCE_SIDE || bitmap.height > MAX_SOURCE_SIDE) {
      throw new AlgoError(
        "SourceTooLarge",
        `源图 ${bitmap.width}×${bitmap.height} 超过 ${MAX_SOURCE_SIDE}×${MAX_SOURCE_SIDE} 上限`,
      );
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
