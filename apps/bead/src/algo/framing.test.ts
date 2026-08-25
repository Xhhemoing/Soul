import { describe, expect, it } from "vitest";

import {
  applyFraming,
  aspectTarget,
  cropImage,
  resampleBox,
  resampleNearest,
} from "./framing.ts";
import { AlgoError, createImage, imageFromPixels, readAlpha, readRgb } from "./image.ts";
import { seededInt, seededRandom } from "../test/random.ts";

describe("T-SCL-1 固定板的 box 恒等用例", () => {
  it("56×56 每 2×2 块内均匀 → 28×28 与逐块颜色一一对应", () => {
    const next = seededRandom(56_28);
    const blocks: number[][] = [];
    for (let by = 0; by < 28; by += 1) {
      const row: number[] = [];
      for (let bx = 0; bx < 28; bx += 1) row.push(seededInt(next, 256));
      blocks.push(row);
    }
    const source = imageFromPixels(56, 56, (x, y) => {
      const v = blocks[y >> 1]![x >> 1]!;
      return [v, 255 - v, (v * 2) % 256, 255];
    });

    const out = applyFraming(source, { mode: "board", size: 28 });
    expect(out.width).toBe(28);
    expect(out.height).toBe(28);
    for (let y = 0; y < 28; y += 1) {
      for (let x = 0; x < 28; x += 1) {
        const v = blocks[y]![x]!;
        expect(readRgb(out, x, y)).toEqual({ r: v, g: 255 - v, b: (v * 2) % 256 });
        expect(readAlpha(out, x, y)).toBe(255);
      }
    }
  });

  it("112×112 每 2×2 块内均匀 → 56×56 同理", () => {
    const source = imageFromPixels(112, 112, (x, y) => {
      const v = ((x >> 1) * 3 + (y >> 1) * 5) % 256;
      return [v, v, v, 255];
    });
    const out = applyFraming(source, { mode: "board", size: 56 });
    expect(out.width).toBe(56);
    for (let y = 0; y < 56; y += 1) {
      for (let x = 0; x < 56; x += 1) {
        const v = (x * 3 + y * 5) % 256;
        expect(readRgb(out, x, y)).toEqual({ r: v, g: v, b: v });
      }
    }
  });
});

describe("T-SCL-2 按比例适配的舍入锁定", () => {
  it.each([
    [100, 50, 28, 14],
    [50, 100, 14, 28],
    [29, 29, 28, 28],
    [3000, 1, 28, 1],
    [1, 3000, 1, 28],
  ])("%i×%i → %i×%i", (w, h, tw, th) => {
    expect(aspectTarget(w, h, 28)).toEqual({ width: tw, height: th });
  });

  it("最小维夹到 1，不会出现 0 尺寸", () => {
    const source = createImage(3000, 1);
    const out = applyFraming(source, { mode: "aspect" });
    expect(out.width).toBe(28);
    expect(out.height).toBe(1);
  });

  it("maxSide 可调，默认 28", () => {
    expect(aspectTarget(100, 50, 56)).toEqual({ width: 56, height: 28 });
    expect(aspectTarget(10, 10, 28)).toEqual({ width: 28, height: 28 });
  });
});

describe("T-SCL-3 手动缩放 + 裁剪", () => {
  const source = imageFromPixels(10, 10, (x, y) => [x * 25, y * 25, 0, 255]);

  it("crop 全在图内：原样取出", () => {
    const out = cropImage(source, { x: 2, y: 3, width: 4, height: 2 });
    expect(out.width).toBe(4);
    expect(out.height).toBe(2);
    expect(readRgb(out, 0, 0)).toEqual({ r: 50, g: 75, b: 0 });
    expect(readRgb(out, 3, 1)).toEqual({ r: 125, g: 100, b: 0 });
  });

  it("crop 部分越界：夹取 + 越界区透明填充", () => {
    const out = cropImage(source, { x: -2, y: -2, width: 5, height: 5 });
    expect(out.width).toBe(5);
    expect(readAlpha(out, 0, 0)).toBe(0);
    expect(readAlpha(out, 1, 1)).toBe(0);
    expect(readAlpha(out, 2, 2)).toBe(255);
    expect(readRgb(out, 2, 2)).toEqual({ r: 0, g: 0, b: 0 });
    expect(readRgb(out, 4, 4)).toEqual({ r: 50, g: 50, b: 0 });
  });

  it("crop 完全在图外：类型化错误", () => {
    expect(() => cropImage(source, { x: 20, y: 0, width: 4, height: 4 })).toThrow(AlgoError);
    try {
      cropImage(source, { x: 20, y: 0, width: 4, height: 4 });
    } catch (error) {
      expect((error as AlgoError).code).toBe("CropOutOfBounds");
    }
  });

  it("scale 与 crop 组合决定输出尺寸", () => {
    const out = applyFraming(source, {
      mode: "manual",
      scale: 0.5,
      crop: { x: 0, y: 0, width: 9, height: 9 },
    });
    // 9 × 0.5 = 4.5 → 半进位 → 5
    expect(out.width).toBe(5);
    expect(out.height).toBe(5);
  });
});

describe("T-SCL-4 参数校验", () => {
  const source = createImage(4, 4);

  it.each([
    ["InvalidScale", { mode: "manual", scale: 0, crop: { x: 0, y: 0, width: 2, height: 2 } }],
    ["InvalidScale", { mode: "manual", scale: -1, crop: { x: 0, y: 0, width: 2, height: 2 } }],
    ["InvalidCrop", { mode: "manual", scale: 1, crop: { x: 0, y: 0, width: 0, height: 2 } }],
    ["InvalidCrop", { mode: "manual", scale: 1, crop: { x: 0, y: 0, width: 2, height: 0 } }],
    ["CropOutOfBounds", { mode: "manual", scale: 1, crop: { x: 9, y: 9, width: 2, height: 2 } }],
  ] as const)("%s 不 panic，抛 AlgoError", (code, framing) => {
    try {
      applyFraming(source, framing);
      throw new Error("应当抛错");
    } catch (error) {
      expect(error).toBeInstanceOf(AlgoError);
      expect((error as AlgoError).code).toBe(code);
    }
  });

  it("0 尺寸图像连构造都不允许", () => {
    expect(() => createImage(0, 4)).toThrow(AlgoError);
    expect(() => createImage(4, 0)).toThrow(AlgoError);
    expect(() => createImage(2, 2, [1, 2, 3])).toThrow(/InvalidDimensions/);
  });

  it("非整数目标尺寸报错", () => {
    expect(() => resampleBox(source, 2.5, 2)).toThrow(/InvalidDimensions/);
    expect(() => resampleBox(source, 0, 2)).toThrow(/InvalidDimensions/);
  });
});

describe("T-SCL-5 重采样空间锁定（线性光）", () => {
  it("2×1 的 [黑, 白] 缩到 1×1 得到 188，而不是 sRGB 码值平均的 128", () => {
    const source = imageFromPixels(2, 1, (x) =>
      x === 0 ? [0, 0, 0, 255] : [255, 255, 255, 255],
    );
    const out = resampleBox(source, 1, 1);
    expect(readRgb(out, 0, 0)).toEqual({ r: 188, g: 188, b: 188 });
    expect(readAlpha(out, 0, 0)).toBe(255);
  });

  it("透明像素不参与颜色平均，只拉低 alpha", () => {
    const source = imageFromPixels(2, 1, (x) =>
      x === 0 ? [255, 0, 0, 255] : [0, 0, 255, 0],
    );
    const out = resampleBox(source, 1, 1);
    expect(readRgb(out, 0, 0)).toEqual({ r: 255, g: 0, b: 0 });
    expect(readAlpha(out, 0, 0)).toBe(128);
  });

  it("整块透明的目标像素是透明黑", () => {
    const source = createImage(2, 2);
    const out = resampleBox(source, 1, 1);
    expect(readAlpha(out, 0, 0)).toBe(0);
    expect(readRgb(out, 0, 0)).toEqual({ r: 0, g: 0, b: 0 });
  });
});

describe("最近邻只用来撤销整数放大", () => {
  it("8× 放大图按相位取回逻辑像素", () => {
    const logical = imageFromPixels(3, 3, (x, y) => [x * 80, y * 80, 0, 255]);
    const upscaled = imageFromPixels(24, 24, (x, y) => {
      const src = readRgb(logical, Math.floor(x / 8), Math.floor(y / 8));
      return [src.r, src.g, src.b, 255];
    });
    const back = resampleNearest(upscaled, 3, 3);
    for (let y = 0; y < 3; y += 1) {
      for (let x = 0; x < 3; x += 1) {
        expect(readRgb(back, x, y)).toEqual(readRgb(logical, x, y));
      }
    }
  });
});
