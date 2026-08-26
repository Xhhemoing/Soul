import { describe, expect, it } from "vitest";

import { roundHalfUp } from "./color.ts";
import { quantize } from "./dither.ts";
import { createImage, imageFromPixels, type RgbaImage } from "./image.ts";
import { GENERIC_5MM, nearestIndex, preparePalette, type Palette } from "./palette.ts";
import { seededInt, seededRandom } from "../test/random.ts";

const MONO: Palette = {
  id: "mono-fixture",
  displayName: "黑白夹具",
  entries: [
    { id: "K", displayName: "黑", rgb: { r: 0, g: 0, b: 0 } },
    { id: "W", displayName: "白", rgb: { r: 255, g: 255, b: 255 } },
  ],
};

function cells(image: RgbaImage, palette: Palette, dither: boolean) {
  return quantize(image, { palette, dither }).grid.cells;
}

describe("T-FS-1 抖动关闭 ⇔ 逐像素独立最近色", () => {
  it("两条路径逐格全等", () => {
    const next = seededRandom(910);
    const image = imageFromPixels(17, 13, () => [
      seededInt(next, 256),
      seededInt(next, 256),
      seededInt(next, 256),
      255,
    ]);
    const prepared = preparePalette(GENERIC_5MM);
    const direct: number[] = [];
    for (let y = 0; y < 13; y += 1) {
      for (let x = 0; x < 17; x += 1) {
        const o = (y * 17 + x) * 4;
        direct.push(
          nearestIndex(
            { r: image.data[o]!, g: image.data[o + 1]!, b: image.data[o + 2]! },
            prepared,
          ),
        );
      }
    }
    expect(cells(image, GENERIC_5MM, false)).toEqual(direct);
  });

  it("单色图在抖动开与关下结果相同（误差恒为同一值，不撕裂）", () => {
    // Exactly G15 Red, so the residual error is zero and nothing can drift.
    const flat = imageFromPixels(8, 8, () => [228, 3, 46, 255]);
    expect(cells(flat, GENERIC_5MM, true)).toEqual(cells(flat, GENERIC_5MM, false));
  });
});

describe("T-FS-2 权重与扫描序", () => {
  it("2×2 全 128 灰 + 黑白板 → 棋盘 [白,黑;黑,白]", () => {
    const image = imageFromPixels(2, 2, () => [128, 128, 128, 255]);
    expect(cells(image, MONO, true)).toEqual([1, 0, 0, 1]);
    // 关掉抖动就是四格同色，正好证明棋盘是误差扩散造成的。
    expect(cells(image, MONO, false)).toEqual([1, 1, 1, 1]);
  });
});

describe("T-FS-3 1×8 灰阶梯", () => {
  it("输出序列冻结，最右像素的误差被丢弃", () => {
    const ramp = imageFromPixels(8, 1, (x) => [x * 32, x * 32, x * 32, 255]);
    expect(cells(ramp, MONO, true)).toEqual([0, 0, 0, 1, 0, 1, 1, 1]);
  });

  it("误差只向右传：改末位不影响前面的格子", () => {
    const ramp = imageFromPixels(8, 1, (x) => [x === 7 ? 0 : x * 32, x === 7 ? 0 : x * 32, x === 7 ? 0 : x * 32, 255]);
    expect(cells(ramp, MONO, true).slice(0, 7)).toEqual([0, 0, 0, 1, 0, 1, 1]);
  });
});

describe("T-FS-4 边界不越界", () => {
  it.each([
    [1, 1],
    [1, 9],
    [9, 1],
    [2, 3],
  ])("%i×%i 不抛错且格数正确", (w, h) => {
    const image = imageFromPixels(w, h, (x, y) => [(x * 37 + y * 91) % 256, 120, 200, 255]);
    const out = cells(image, GENERIC_5MM, true);
    expect(out.length).toBe(w * h);
    expect(out.every((c) => c !== null)).toBe(true);
  });
});

describe("T-FS-5 输出封闭性", () => {
  it("误差累计越界的构造用例仍只输出色板索引", () => {
    // 一行全黑接一行全白，误差会被推出 [0,255] 两侧。
    const image = imageFromPixels(16, 16, (_x, y) => {
      const v = y % 2 === 0 ? 0 : 255;
      return [v, v, v, 255];
    });
    for (const out of [cells(image, MONO, true), cells(image, GENERIC_5MM, true)]) {
      for (const cell of out) {
        expect(cell).not.toBeNull();
        expect(cell).toBeGreaterThanOrEqual(0);
      }
    }
    expect(new Set(cells(image, MONO, true))).toEqual(new Set([0, 1]));
  });
});

describe("T-FS-6 均值守恒", () => {
  it("64×64 全 128 灰 + 黑白板：白占比与 128/255 偏差 ≤ 2%", () => {
    const image = imageFromPixels(64, 64, () => [128, 128, 128, 255]);
    const out = cells(image, MONO, true);
    const whiteShare = out.filter((c) => c === 1).length / out.length;
    expect(Math.abs(whiteShare - 128 / 255)).toBeLessThanOrEqual(0.02);
  });
});

describe("T-FS-7 透明格隔离", () => {
  const hole = imageFromPixels(6, 6, (x, y) => {
    const inHole = x >= 2 && x <= 3 && y >= 2 && y <= 3;
    return inHole ? [0, 0, 0, 0] : [128, 128, 128, 255];
  });

  it("洞不出现在输出里，周围格序列冻结", () => {
    expect(cells(hole, MONO, true)).toEqual([
      1, 0, 1, 0, 1, 0,
      0, 1, 0, 1, 0, 1,
      1, 0, null, null, 1, 0,
      0, 1, null, null, 0, 1,
      1, 0, 1, 0, 1, 0,
      0, 1, 0, 1, 0, 1,
    ]);
  });

  it("alpha 恰好 127 是空格、128 是不透明格", () => {
    const edge = imageFromPixels(2, 1, (x) => [200, 200, 200, x === 0 ? 127 : 128]);
    const out = cells(edge, MONO, false);
    expect(out[0]).toBeNull();
    expect(out[1]).not.toBeNull();
  });

  it("全透明图产出全空网格且不报错", () => {
    const out = quantize(createImage(4, 4), { palette: MONO, dither: true });
    expect(out.grid.cells.every((c) => c === null)).toBe(true);
    expect(out.minRunnerUpMargin).toBe(Infinity);
  });
});

describe("AL-2 查表前取整到整码值，与 oracle 的 to_channel 同式", () => {
  // 125 灰落在 G08 Slate 上，残差的 7/16 把第二格推到 241.875 —— 正好跨过 G02
  // 与 G01 的决策边界：按小数查表得 G02，按 to_channel 取整成 242 再查得 G01。
  const image = createImage(2, 1, [125, 125, 125, 255, 234, 234, 234, 255]);

  it("残差落在半个码值上时，取整与不取整挑到不同的色", () => {
    const prepared = preparePalette(GENERIC_5MM);
    const slate = GENERIC_5MM.entries[nearestIndex({ r: 125, g: 125, b: 125 }, prepared)]!;
    expect(slate.id).toBe("G08");

    const carried = {
      r: 234 + (7 / 16) * (125 - slate.rgb.r),
      g: 234 + (7 / 16) * (125 - slate.rgb.g),
      b: 234 + (7 / 16) * (125 - slate.rgb.b),
    };
    expect(carried).toEqual({ r: 241.875, g: 235.3125, b: 230.5 });

    const fractional = GENERIC_5MM.entries[nearestIndex(carried, prepared)]!.id;
    const rounded =
      GENERIC_5MM.entries[
        nearestIndex(
          {
            r: roundHalfUp(carried.r),
            g: roundHalfUp(carried.g),
            b: roundHalfUp(carried.b),
          },
          prepared,
        )
      ]!.id;
    expect(fractional).toBe("G02");
    expect(rounded).toBe("G01");
  });

  it("量化走的是取整那条：第二格是 G01", () => {
    const out = cells(image, GENERIC_5MM, true).map((cell) =>
      cell === null ? "" : GENERIC_5MM.entries[cell]!.id,
    );
    expect(out).toEqual(["G08", "G01"]);
  });
});

describe("近平局哨兵", () => {
  it("量化会报告全图最小的次优 ΔE 余量", () => {
    const image = imageFromPixels(4, 4, () => [230, 40, 50, 255]);
    const result = quantize(image, { palette: GENERIC_5MM, dither: false });
    expect(result.minRunnerUpMargin).toBeGreaterThan(1e-6);
  });

  it("色板里有重复色时余量为 0，正是 fixture 该被换掉的信号", () => {
    const dup: Palette = {
      id: "dup",
      displayName: "重复",
      entries: [
        { id: "A", displayName: "甲", rgb: { r: 10, g: 20, b: 30 } },
        { id: "B", displayName: "乙", rgb: { r: 10, g: 20, b: 30 } },
      ],
    };
    const image = imageFromPixels(1, 1, () => [10, 20, 30, 255]);
    const result = quantize(image, { palette: dup, dither: false });
    expect(result.minRunnerUpMargin).toBe(0);
    expect(result.grid.cells).toEqual([0]);
  });
});
