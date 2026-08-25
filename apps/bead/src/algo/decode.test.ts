import { describe, expect, it, vi } from "vitest";

import { DECODE_OPTIONS, decodeImage } from "./decode.ts";

/**
 * T-PAR-2 (browser half). jsdom has neither `createImageBitmap` nor a real 2D
 * context, so what is asserted here is the part that actually breaks parity:
 * the decode must ask for no colour-space conversion, no premultiply, and must
 * not resample. The bytes themselves are covered by the raw-RGBA parity
 * fixture, which never touches this path.
 */
describe("T-PAR-2 上传解码中和 ICC", () => {
  function stubDecode(width: number, height: number, bytes: Uint8ClampedArray) {
    const close = vi.fn();
    const drawImage = vi.fn();
    const getImageData = vi.fn(() => ({ data: bytes }));
    const context = {
      imageSmoothingEnabled: true,
      drawImage,
      getImageData,
    };
    const createImageBitmap = vi.fn(async () => ({ width, height, close }));
    vi.stubGlobal("createImageBitmap", createImageBitmap);
    vi.stubGlobal("OffscreenCanvas", undefined);
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
      context as unknown as CanvasRenderingContext2D,
    );
    return { createImageBitmap, context, drawImage, getImageData, close };
  }

  it("请求 colorSpaceConversion: none 与 premultiplyAlpha: none", async () => {
    const bytes = new Uint8ClampedArray([1, 2, 3, 255, 4, 5, 6, 255]);
    const stub = stubDecode(2, 1, bytes);
    const blob = new Blob([]);

    const image = await decodeImage(blob);

    expect(stub.createImageBitmap).toHaveBeenCalledWith(blob, DECODE_OPTIONS);
    expect(DECODE_OPTIONS).toEqual({
      colorSpaceConversion: "none",
      premultiplyAlpha: "none",
    });
    expect(image.width).toBe(2);
    expect([...image.data]).toEqual([1, 2, 3, 255, 4, 5, 6, 255]);
  });

  it("以自然尺寸 1:1 绘制并关掉平滑，解码阶段不重采样", async () => {
    const stub = stubDecode(3, 2, new Uint8ClampedArray(3 * 2 * 4));
    await decodeImage(new Blob([]));

    expect(stub.context.imageSmoothingEnabled).toBe(false);
    // 只有 (source, 0, 0)：没有目标尺寸参数就没有缩放。
    expect(stub.drawImage).toHaveBeenCalledTimes(1);
    expect(stub.drawImage.mock.calls[0]!.length).toBe(3);
    expect(stub.getImageData).toHaveBeenCalledWith(0, 0, 3, 2);
  });

  it("解码完释放 bitmap", async () => {
    const stub = stubDecode(1, 1, new Uint8ClampedArray(4));
    await decodeImage(new Blob([]));
    expect(stub.close).toHaveBeenCalledTimes(1);
  });

  it("零尺寸解码结果报 InvalidDimensions 而不是继续跑", async () => {
    const stub = stubDecode(0, 4, new Uint8ClampedArray(0));
    await expect(decodeImage(new Blob([]))).rejects.toThrow(/InvalidDimensions/);
    expect(stub.close).toHaveBeenCalledTimes(1);
  });
});
