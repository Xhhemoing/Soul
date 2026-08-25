import { describe, expect, it } from "vitest";

import {
  classificationFeatures,
  classifyImage,
  DEGENERATE_CONFIDENCE,
  detectGrid,
} from "./classify.ts";
import { createImage, imageFromPixels, type RgbaImage } from "./image.ts";

const SPRITE_COLORS = [
  [255, 0, 0],
  [0, 255, 0],
  [0, 0, 255],
  [255, 255, 0],
  [255, 0, 255],
  [0, 255, 255],
  [255, 255, 255],
  [0, 0, 0],
  [128, 64, 32],
  [32, 128, 64],
  [64, 32, 128],
  [200, 200, 60],
] as const;

/** 24×24, 12 colours, nearest-neighbour up-scaled 8× with an optional phase. */
function sprite8x(offset = 0): RgbaImage {
  return imageFromPixels(192, 192, (x, y) => {
    const cx = Math.floor((x + 8 - offset) / 8);
    const cy = Math.floor((y + 8 - offset) / 8);
    const c = SPRITE_COLORS[(((cx * 5 + cy * 7) % 12) + 12) % 12]!;
    return [c[0], c[1], c[2], 255];
  });
}

describe("T-CLS-1 合成像素图", () => {
  it("24×24 / 12 色 sprite 放大 8× → PixelArt 且置信度 ≥ 0.8", () => {
    const classification = classifyImage(sprite8x());
    expect(classification.kind).toBe("PixelArt");
    expect(classification.confidence).toBeGreaterThanOrEqual(0.8);
    expect(classification.confidence).toBeCloseTo(0.9304, 4);
  });

  it("特征值可解释：12 个独特色、格内完全一致", () => {
    const features = classificationFeatures(sprite8x());
    expect(features.uniqueColors).toBe(12);
    expect(features.gridEvidence).toBe(1);
    expect(features.opaquePixels).toBe(192 * 192);
  });
});

describe("T-CLS-2 照片样张", () => {
  it("256×256 平滑双向渐变 → Photo", () => {
    const photo = imageFromPixels(256, 256, (x, y) => [x, y, (x + y) >> 1, 255]);
    const classification = classifyImage(photo);
    expect(classification.kind).toBe("Photo");
    expect(classification.confidence).toBe(1);
    expect(classificationFeatures(photo).uniqueColors).toBeGreaterThanOrEqual(10000);
  });
});

describe("T-CLS-3 置信度有界", () => {
  const noise = (() => {
    let state = 12345;
    return imageFromPixels(64, 64, () => {
      state = (state * 1103515245 + 12345) & 0x7fffffff;
      return [(state >> 3) & 255, (state >> 11) & 255, (state >> 19) & 255, 255];
    });
  })();

  it.each([
    ["1×1", imageFromPixels(1, 1, () => [10, 20, 30, 255])],
    ["单色", imageFromPixels(20, 20, () => [10, 20, 30, 255])],
    ["噪声", noise],
    ["全透明", createImage(8, 8)],
  ] as const)("%s 图的置信度在 [0,1] 且不是 NaN", (_name, image) => {
    const { confidence } = classifyImage(image);
    expect(Number.isNaN(confidence)).toBe(false);
    expect(confidence).toBeGreaterThanOrEqual(0);
    expect(confidence).toBeLessThanOrEqual(1);
  });
});

describe("T-CLS-4 全透明与单色的判定钉死", () => {
  it("全透明图判 PixelArt，置信度取契约默认值 0.5", () => {
    const classification = classifyImage(createImage(8, 8));
    expect(classification).toEqual({ kind: "PixelArt", confidence: DEGENERATE_CONFIDENCE });
    expect(DEGENERATE_CONFIDENCE).toBe(0.5);
  });

  it("单色图判 PixelArt", () => {
    const classification = classifyImage(imageFromPixels(20, 20, () => [10, 20, 30, 255]));
    expect(classification.kind).toBe("PixelArt");
    expect(classification.confidence).toBeCloseTo(0.9992, 4);
  });

  it("1×1 图判 PixelArt，相邻对为空时取「平坦」", () => {
    const classification = classifyImage(imageFromPixels(1, 1, () => [10, 20, 30, 255]));
    expect(classification.kind).toBe("PixelArt");
    expect(classificationFeatures(imageFromPixels(1, 1, () => [1, 2, 3, 255])).flatNeighbourRatio).toBe(1);
  });
});

describe("T-GRID-1 detectGrid", () => {
  it("8× 放大图：cell 8×8、offset (0,0)", () => {
    expect(detectGrid(sprite8x())).toEqual({
      cellWidth: 8,
      cellHeight: 8,
      offsetX: 0,
      offsetY: 0,
    });
  });

  it("带 3px 相位偏移的变体：offset (3,3)", () => {
    expect(detectGrid(sprite8x(3))).toEqual({
      cellWidth: 8,
      cellHeight: 8,
      offsetX: 3,
      offsetY: 3,
    });
  });

  it("逐像素都在变的图退化为 1×1 格", () => {
    const photo = imageFromPixels(64, 64, (x, y) => [x * 4, y * 4, 0, 255]);
    expect(detectGrid(photo)).toEqual({
      cellWidth: 1,
      cellHeight: 1,
      offsetX: 0,
      offsetY: 0,
    });
  });

  it("完全均匀的图没有周期可推断，整幅算一格", () => {
    expect(detectGrid(imageFromPixels(9, 5, () => [7, 7, 7, 255]))).toEqual({
      cellWidth: 9,
      cellHeight: 5,
      offsetX: 0,
      offsetY: 0,
    });
  });

  it("透明与不透明的分界也算变化位置", () => {
    const halves = imageFromPixels(8, 4, (x) => [10, 10, 10, x < 4 ? 0 : 255]);
    expect(detectGrid(halves).cellWidth).toBe(8);
  });
});
