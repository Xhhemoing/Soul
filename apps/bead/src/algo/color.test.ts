import { describe, expect, it } from "vitest";

import { ciede2000, linearToSrgb, rgbToLab, roundHalfUp, srgbToLinear, type Lab } from "./color.ts";

/**
 * Sharma, Wu & Dalal (2005) Table I, transcribed in
 * `docs/bead/fixtures-ciede2000.md`. Row 34 is not in that fixture file but the
 * review names it explicitly (near-black, low chroma), and the paper's value is
 * quoted there too, so it is included as the last row.
 */
const SHARMA: ReadonlyArray<readonly [number, Lab, Lab, number]> = [
  [1, { l: 50, a: 2.6772, b: -79.7751 }, { l: 50, a: 0, b: -82.7485 }, 2.0425],
  [2, { l: 50, a: 3.1571, b: -77.2803 }, { l: 50, a: 0, b: -82.7485 }, 2.8615],
  [3, { l: 50, a: 2.8361, b: -74.02 }, { l: 50, a: 0, b: -82.7485 }, 3.4412],
  [4, { l: 50, a: -1.3802, b: -84.2814 }, { l: 50, a: 0, b: -82.7485 }, 1.0],
  [5, { l: 50, a: -1.1848, b: -84.8006 }, { l: 50, a: 0, b: -82.7485 }, 1.0],
  [6, { l: 50, a: -0.9009, b: -85.5211 }, { l: 50, a: 0, b: -82.7485 }, 1.0],
  [7, { l: 50, a: 0, b: 0 }, { l: 50, a: -1, b: 2 }, 2.3669],
  [8, { l: 50, a: -1, b: 2 }, { l: 50, a: 0, b: 0 }, 2.3669],
  [9, { l: 50, a: 2.49, b: -0.001 }, { l: 50, a: -2.49, b: 0.0009 }, 7.1792],
  [10, { l: 50, a: 2.49, b: -0.001 }, { l: 50, a: -2.49, b: 0.001 }, 7.1792],
  [11, { l: 50, a: 2.49, b: -0.001 }, { l: 50, a: -2.49, b: 0.0011 }, 7.2195],
  [12, { l: 50, a: 2.49, b: -0.001 }, { l: 50, a: -2.49, b: 0.0012 }, 7.2195],
  [13, { l: 50, a: -0.001, b: 2.49 }, { l: 50, a: 0.0009, b: -2.49 }, 4.8045],
  [14, { l: 50, a: -0.001, b: 2.49 }, { l: 50, a: 0.001, b: -2.49 }, 4.8045],
  [15, { l: 50, a: -0.001, b: 2.49 }, { l: 50, a: 0.0011, b: -2.49 }, 4.7461],
  [16, { l: 50, a: 2.5, b: 0 }, { l: 50, a: 0, b: -2.5 }, 4.3065],
  [17, { l: 50, a: 2.5, b: 0 }, { l: 73, a: 25, b: -18 }, 27.1492],
  [18, { l: 50, a: 2.5, b: 0 }, { l: 61, a: -5, b: 29 }, 22.8977],
  [19, { l: 50, a: 2.5, b: 0 }, { l: 56, a: -27, b: -3 }, 31.903],
  [20, { l: 50, a: 2.5, b: 0 }, { l: 58, a: 24, b: 15 }, 19.4535],
  [34, { l: 2.0776, a: 0.0795, b: -1.135 }, { l: 0.9033, a: -0.0636, b: -0.5514 }, 0.9082],
];

describe("T-DE-1 CIEDE2000 对 Sharma 2005 表", () => {
  it.each(SHARMA)("第 %i 组", (_row, first, second, expected) => {
    expect(ciede2000(first, second)).toBeCloseTo(expected, 4);
  });

  it("覆盖到审查点名的 8 组分支用例", () => {
    const rows = SHARMA.map(([row]) => row);
    for (const row of [1, 2, 3, 7, 9, 13, 17, 34]) expect(rows).toContain(row);
  });
});

describe("T-DE-2 对称性", () => {
  it.each([9, 13])("第 %i 组两向相等", (row) => {
    const found = SHARMA.find(([id]) => id === row)!;
    expect(ciede2000(found[1], found[2])).toBe(ciede2000(found[2], found[1]));
  });
});

describe("T-DE-3 自反性", () => {
  it.each([
    { l: 50, a: 0, b: 0 },
    { l: 0, a: 0, b: 0 },
    { l: 100, a: 0, b: 0 },
    { l: 32.297, a: 79.1875, b: -107.8602 },
  ])("ΔE00(x, x) = 0：%j", (lab) => {
    expect(ciede2000(lab, lab)).toBe(0);
  });
});

/**
 * T-SRGB-1. The pinned matrix (contract G2) reproduces the review's anchors to
 * four decimals, so the tolerance can be tighter than the ±0.01 it allowed.
 */
describe("T-SRGB-1 sRGB→Lab 锚点", () => {
  it.each([
    [0, 0, 0, 0.0, 0.0, 0.0],
    [255, 255, 255, 100.0, 0.0, 0.0],
    [255, 0, 0, 53.2408, 80.0925, 67.2032],
    [0, 255, 0, 87.7347, -86.1827, 83.1793],
    [0, 0, 255, 32.297, 79.1875, -107.8602],
    [128, 128, 128, 53.585, 0.0, 0.0],
  ])("(%i,%i,%i)", (r, g, b, l, a, bb) => {
    const lab = rgbToLab({ r, g, b });
    expect(lab.l).toBeCloseTo(l, 4);
    expect(lab.a).toBeCloseTo(a, 4);
    expect(lab.b).toBeCloseTo(bb, 4);
  });

  // The matrix rows sum to the white point at 7 decimals, so neutrals land on
  // a* = b* = 0 to within 1e-5 without any white-point special case.
  it("中性色的 a*/b* 在 1e-4 内为 0，不需要白点特判", () => {
    for (const v of [0, 64, 128, 200, 255]) {
      const lab = rgbToLab({ r: v, g: v, b: v });
      expect(Math.abs(lab.a)).toBeLessThan(1e-4);
      expect(Math.abs(lab.b)).toBeLessThan(1e-4);
    }
  });
});

describe("传递函数与舍入", () => {
  it("sRGB 码值经线性光往返不变", () => {
    for (let v = 0; v <= 255; v += 1) expect(linearToSrgb(srgbToLinear(v))).toBe(v);
  });

  it("舍入是 floor(x + 0.5)，.5 一律进位", () => {
    expect(roundHalfUp(0.5)).toBe(1);
    expect(roundHalfUp(1.5)).toBe(2);
    expect(roundHalfUp(2.4999)).toBe(2);
    expect(roundHalfUp(187.5)).toBe(188);
  });

  it("线性化夹在 [0,1]，编码夹在 [0,255]", () => {
    expect(srgbToLinear(-10)).toBe(0);
    expect(srgbToLinear(999)).toBe(1);
    expect(linearToSrgb(-1)).toBe(0);
    expect(linearToSrgb(2)).toBe(255);
  });
});
