import { describe, expect, it, vi } from "vitest";

import { DECODE_OPTIONS, MAX_SOURCE_SIDE, decodeImage } from "./decode.ts";
import { AlgoError } from "./image.ts";

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

  // T-UP-8 / DATA-3。上限按位图头判，且必须在 drawImage 之前——过了那一行，
  // 1.6GB 的 RGBA 已经要出来了，再拒绝也没有意义。
  describe("DATA-3 源尺寸上限", () => {
    it("正好 4096 的边放行", async () => {
      const bytes = new Uint8ClampedArray(MAX_SOURCE_SIDE * 4);
      stubDecode(MAX_SOURCE_SIDE, 1, bytes);
      expect((await decodeImage(new Blob([]))).width).toBe(MAX_SOURCE_SIDE);

      stubDecode(1, MAX_SOURCE_SIDE, bytes);
      expect((await decodeImage(new Blob([]))).height).toBe(MAX_SOURCE_SIDE);
    });

    it("4096×4096 的位图头过闸，走到 drawImage 那一步", async () => {
      // 这里不喂 64MB 的像素（那正是上限要挡的分配），只验证闸门放行：
      // drawImage 被调到就说明尺寸检查没有拦它。
      const stub = stubDecode(MAX_SOURCE_SIDE, MAX_SOURCE_SIDE, new Uint8ClampedArray(4));
      const error = await decodeImage(new Blob([])).catch((reason: unknown) => reason);
      expect(error).toBeInstanceOf(AlgoError);
      // 桩只给了 4 字节，所以它死在长度检查上——不是死在尺寸闸门上。
      expect((error as AlgoError).code).toBe("InvalidDimensions");
      expect(stub.drawImage).toHaveBeenCalledTimes(1);
    });

    it("任一轴超限就抛 SourceTooLarge，且不画、bitmap 照样释放", async () => {
      for (const [width, height] of [
        [MAX_SOURCE_SIDE + 1, 10],
        [10, MAX_SOURCE_SIDE + 1],
      ]) {
        const stub = stubDecode(width!, height!, new Uint8ClampedArray(4));
        await expect(decodeImage(new Blob([]))).rejects.toThrow(/SourceTooLarge/);
        expect(stub.drawImage).not.toHaveBeenCalled();
        expect(stub.close).toHaveBeenCalledTimes(1);
      }
    });
  });
});
