import { describe, expect, it } from "vitest";

import { totalBeads } from "./bom.ts";
import { occupiedCount } from "./grid.ts";
import { AlgoError, createImage, imageFromPixels } from "./image.ts";
import { entryAt, GENERIC_5MM } from "./palette.ts";
import { imageToPattern } from "./pipeline.ts";

const SWATCH = [
  [250, 220, 60],
  [230, 40, 50],
  [40, 120, 200],
] as const;

/** 6×6 logical sprite, nearest-neighbour up-scaled by `factor`. */
function upscaledSprite(factor: number) {
  return imageFromPixels(6 * factor, 6 * factor, (x, y) => {
    const c = SWATCH[(Math.floor(x / factor) + Math.floor(y / factor)) % 3]!;
    return [c[0], c[1], c[2], 255];
  });
}

describe("像素图路径", () => {
  it("先按 detectGrid 撤销放大，再框定，色号与逻辑像素一一对应", () => {
    const result = imageToPattern(upscaledSprite(5), {
      framing: { mode: "aspect", maxSide: 6 },
    });
    expect(result.kind).toBe("PixelArt");
    expect(result.detectedGrid).toEqual({
      cellWidth: 5,
      cellHeight: 5,
      offsetX: 0,
      offsetY: 0,
    });
    expect(result.grid.width).toBe(6);
    expect(result.grid.height).toBe(6);
    expect(result.bom.map((row) => row.code).sort()).toEqual(["G08", "G12", "G21"]);
  });

  it("像素图路径强制关闭抖动，并如实报告", () => {
    const result = imageToPattern(upscaledSprite(5), {
      framing: { mode: "aspect", maxSide: 6 },
      dither: true,
    });
    expect(result.kind).toBe("PixelArt");
    expect(result.ditherApplied).toBe(false);
  });

  it("强制走 Photo 路径就不撤放大，也允许抖动", () => {
    const result = imageToPattern(upscaledSprite(5), {
      framing: { mode: "aspect", maxSide: 6 },
      dither: true,
      kind: "Photo",
    });
    expect(result.kind).toBe("Photo");
    expect(result.detectedGrid).toBeNull();
    expect(result.ditherApplied).toBe(true);
    // 判定器的原始结论仍然保留，供 UI 提示「你覆盖了自动判定」。
    expect(result.classification.kind).toBe("PixelArt");
  });
});

/**
 * AT-2 of `docs/bead/reviews/round1-algo-ts-review.md`. A cropped screenshot of
 * pixel art is exactly the input `detectGrid`'s phase support exists for, and
 * its last cell is usually cut short. Deriving the nearest-neighbour sample
 * point from `sourceWidth / targetWidth` instead of the detected geometry made
 * the point drift across cell borders: the review's 9×4 case came back
 * `[G08, G08, G15]`, reading the second logical pixel out of the red cell and
 * dropping the blue one entirely.
 */
describe("AT-2 截断与相位偏移的放大图", () => {
  const RED = [255, 0, 0, 255] as const;
  const GREEN = [0, 255, 0, 255] as const;
  const BLUE = [0, 0, 255, 255] as const;
  const YELLOW = [255, 255, 0, 255] as const;

  /** A 4×-up-scaled strip whose cell borders sit at `offset + 4k`. */
  function strip(width: number, offset: number, colors: readonly (readonly number[])[]) {
    return imageFromPixels(width, 4, (x) => {
      const index = x < offset ? 0 : Math.floor((x - offset) / 4) + (offset > 0 ? 1 : 0);
      const c = colors[Math.min(index, colors.length - 1)]!;
      return [c[0]!, c[1]!, c[2]!, c[3]!];
    });
  }

  function codesOf(image: ReturnType<typeof imageFromPixels>, maxSide: number) {
    const result = imageToPattern(image, {
      framing: { mode: "aspect", maxSide },
      kind: "PixelArt",
    });
    return {
      detectedGrid: result.detectedGrid,
      size: [result.grid.width, result.grid.height],
      codes: result.grid.cells.map((cell) => entryAt(GENERIC_5MM, cell!).id),
    };
  }

  it("末格被截为 1px：三个逻辑像素仍是三个不同色号", () => {
    const image = strip(9, 0, [RED, GREEN, BLUE]);
    const actual = codesOf(image, 3);
    expect(actual.detectedGrid).toEqual({
      cellWidth: 4,
      cellHeight: 4,
      offsetX: 0,
      offsetY: 0,
    });
    expect(actual.size).toEqual([3, 1]);
    expect(actual.codes).toEqual(["G08", "G15", "G25"]);
    expect(new Set(actual.codes).size).toBe(3);
  });

  it("相位偏移 3px、末格完整：前导残格也是一个逻辑像素", () => {
    const image = strip(11, 3, [RED, GREEN, BLUE]);
    const actual = codesOf(image, 3);
    expect(actual.detectedGrid).toEqual({
      cellWidth: 4,
      cellHeight: 4,
      offsetX: 3,
      offsetY: 0,
    });
    expect(actual.size).toEqual([3, 1]);
    expect(actual.codes).toEqual(["G08", "G15", "G25"]);
  });

  it("相位偏移 3px 且末格被截：两端的残格都保留", () => {
    const image = strip(12, 3, [RED, GREEN, BLUE, YELLOW]);
    const actual = codesOf(image, 4);
    expect(actual.detectedGrid).toEqual({
      cellWidth: 4,
      cellHeight: 4,
      offsetX: 3,
      offsetY: 0,
    });
    expect(actual.size).toEqual([4, 1]);
    expect(actual.codes).toEqual(["G08", "G15", "G25", "G12"]);
  });
});

describe("照片路径", () => {
  const photo = imageFromPixels(112, 112, (x, y) => [x * 2, y * 2, (x + y) % 256, 255]);

  it("固定板 28×28 与 56×56", () => {
    for (const size of [28, 56] as const) {
      const result = imageToPattern(photo, { framing: { mode: "board", size } });
      expect(result.kind).toBe("Photo");
      expect(result.grid.width).toBe(size);
      expect(result.grid.height).toBe(size);
      expect(totalBeads(result.bom)).toBe(size * size);
    }
  });

  it("抖动开关会改变色号序列", () => {
    const flat = imageToPattern(photo, { framing: { mode: "board", size: 28 }, dither: false });
    const dithered = imageToPattern(photo, { framing: { mode: "board", size: 28 }, dither: true });
    expect(dithered.ditherApplied).toBe(true);
    expect(dithered.grid.cells).not.toEqual(flat.grid.cells);
    expect(totalBeads(dithered.bom)).toBe(totalBeads(flat.bom));
  });

  it("手动缩放 + 裁剪走同一条量化路径", () => {
    const result = imageToPattern(photo, {
      framing: { mode: "manual", scale: 0.25, crop: { x: 8, y: 8, width: 56, height: 56 } },
      kind: "Photo",
    });
    expect(result.grid.width).toBe(14);
    expect(result.grid.height).toBe(14);
  });
});

describe("BOM 与网格始终一致", () => {
  it("Σ颗数 == 非空格数（含透明输入）", () => {
    const holed = imageFromPixels(32, 32, (x, y) => {
      const clear = x >= 8 && x < 24 && y >= 8 && y < 24;
      return clear ? [0, 0, 0, 0] : [x * 8, y * 8, 90, 255];
    });
    const result = imageToPattern(holed, { framing: { mode: "board", size: 28 }, kind: "Photo" });
    expect(totalBeads(result.bom)).toBe(occupiedCount(result.grid));
    expect(totalBeads(result.bom)).toBeLessThan(28 * 28);
  });

  it("全透明输入产出空 BOM 而不是报错", () => {
    const result = imageToPattern(createImage(16, 16), { framing: { mode: "board", size: 28 } });
    expect(result.bom).toEqual([]);
    expect(occupiedCount(result.grid)).toBe(0);
  });
});

describe("管线的类型化失败", () => {
  it("空色板报 EmptyPalette", () => {
    try {
      imageToPattern(createImage(4, 4), {
        framing: { mode: "board", size: 28 },
        palette: { id: "empty", displayName: "空", entries: [] },
      });
      throw new Error("应当抛错");
    } catch (error) {
      expect(error).toBeInstanceOf(AlgoError);
      expect((error as AlgoError).code).toBe("EmptyPalette");
    }
  });

  it("默认色板就是 generic-5mm", () => {
    const result = imageToPattern(createImage(4, 4), { framing: { mode: "board", size: 28 } });
    expect(result.palette).toBe(GENERIC_5MM);
  });

  it("同一输入跑两遍逐字节相等", () => {
    const options = { framing: { mode: "board", size: 28 }, dither: true } as const;
    const first = imageToPattern(upscaledSprite(4), options);
    const second = imageToPattern(upscaledSprite(4), options);
    expect(JSON.stringify(first.grid)).toBe(JSON.stringify(second.grid));
    expect(JSON.stringify(first.bom)).toBe(JSON.stringify(second.bom));
  });
});
