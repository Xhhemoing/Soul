import { describe, expect, it } from "vitest";

import { ciede2000Rgb, rgbToLab, type Lab } from "./color.ts";
import {
  findSubstitutes,
  SUBSTITUTE_MAX_DELTA_E,
  substitutesForLab,
  type InventoryColor,
} from "./substitutes.ts";

const SHARMA_2_A: Lab = { l: 50, a: 3.1571, b: -77.2803 };
const SHARMA_2_B: Lab = { l: 50, a: 0, b: -82.7485 }; // ΔE00 = 2.8615 → 入选
const SHARMA_3_A: Lab = { l: 50, a: 2.8361, b: -74.02 }; // 对 SHARMA_2_B 是 3.4412 → 排除

/**
 * Frozen by search over the pinned CIEDE2000: these two straddle the boundary
 * from either side by ~5e-3, which is far wider than any float drift but far
 * narrower than any plausible re-tuning of the threshold.
 */
const BOUNDARY_BASE = { r: 120, g: 130, b: 140 };
const JUST_BELOW_3 = { r: 128, g: 138, b: 149 }; // ΔE00 ≈ 2.9952
const JUST_ABOVE_3 = { r: 128, g: 132, b: 141 }; // ΔE00 ≈ 3.0012

describe("T-SUB-1 严格小于 3", () => {
  it("Sharma 第 2 组入选、第 3 组排除", () => {
    const hits = substitutesForLab(SHARMA_2_B, [SHARMA_2_A, SHARMA_3_A]);
    expect(hits.map((it) => it.index)).toEqual([0]);
    expect(hits[0]!.deltaE).toBeCloseTo(2.8615, 4);
  });

  it("冻结的 RGB 对钉住 2.9952 入选 / 3.0012 排除", () => {
    expect(ciede2000Rgb(BOUNDARY_BASE, JUST_BELOW_3)).toBeCloseTo(2.9952, 4);
    expect(ciede2000Rgb(BOUNDARY_BASE, JUST_ABOVE_3)).toBeCloseTo(3.0012, 4);

    const inventory: InventoryColor[] = [
      { code: "I0", displayName: "偏下", rgb: JUST_BELOW_3 },
      { code: "I1", displayName: "偏上", rgb: JUST_ABOVE_3 },
    ];
    expect(findSubstitutes(BOUNDARY_BASE, inventory).map((it) => it.code)).toEqual(["I0"]);
  });

  it("ΔE00 恰好等于阈值时排除（`<` 不是 `<=`）", () => {
    const wanted: Lab = { l: 50, a: 0, b: 0 };
    const exact = substitutesForLab(wanted, [{ l: 53, a: 0, b: 0 }], { maxDeltaE: 0 });
    expect(exact).toEqual([]);
    const self = substitutesForLab(wanted, [wanted], { maxDeltaE: 0 });
    expect(self).toEqual([]);
    expect(SUBSTITUTE_MAX_DELTA_E).toBe(3);
  });
});

describe("T-SUB-2 排序与确定性", () => {
  const inventory: InventoryColor[] = [
    { code: "I0", displayName: "远", rgb: { r: 128, g: 132, b: 141 } },
    { code: "I1", displayName: "近", rgb: { r: 122, g: 132, b: 142 } },
    { code: "I2", displayName: "更近", rgb: { r: 121, g: 131, b: 141 } },
  ];

  it("按 ΔE 升序", () => {
    const hits = findSubstitutes(BOUNDARY_BASE, inventory);
    expect(hits.map((it) => it.code)).toEqual(["I2", "I1"]);
    expect(hits[0]!.deltaE).toBeLessThan(hits[1]!.deltaE);
  });

  it("同 ΔE 按库存下标升序", () => {
    const same = { r: 121, g: 131, b: 141 };
    const tied: InventoryColor[] = [
      { code: "A", displayName: "甲", rgb: same },
      { code: "B", displayName: "乙", rgb: same },
      { code: "C", displayName: "丙", rgb: same },
    ];
    expect(findSubstitutes(BOUNDARY_BASE, tied).map((it) => it.code)).toEqual(["A", "B", "C"]);
  });

  it("同一输入两遍结果全等", () => {
    const first = findSubstitutes(BOUNDARY_BASE, inventory);
    const second = findSubstitutes(BOUNDARY_BASE, inventory);
    expect(JSON.stringify(first)).toBe(JSON.stringify(second));
  });
});

describe("T-SUB-3 空结果与自身在库存", () => {
  it("没有 ΔE<3 的候选时返回空列表而不是报错", () => {
    const inventory: InventoryColor[] = [
      { code: "I0", displayName: "黑", rgb: { r: 0, g: 0, b: 0 } },
      { code: "I1", displayName: "白", rgb: { r: 255, g: 255, b: 255 } },
    ];
    expect(findSubstitutes({ r: 200, g: 30, b: 40 }, inventory)).toEqual([]);
    expect(findSubstitutes({ r: 0, g: 0, b: 0 }, [])).toEqual([]);
  });

  it("库存里有完全相同的颜色时它以 ΔE 0 排第一，核心函数不特判", () => {
    const wanted = { r: 200, g: 30, b: 40 };
    const inventory: InventoryColor[] = [
      { code: "I0", displayName: "近", rgb: { r: 202, g: 32, b: 42 } },
      { code: "I1", displayName: "同色", rgb: wanted },
    ];
    const hits = findSubstitutes(wanted, inventory);
    expect(hits[0]!.code).toBe("I1");
    expect(hits[0]!.deltaE).toBe(0);
    expect(hits.length).toBe(2);
  });

  it("Lab 级 API 与 RGB 级 API 结论一致", () => {
    const inventory: InventoryColor[] = [
      { code: "I0", displayName: "偏下", rgb: JUST_BELOW_3 },
      { code: "I1", displayName: "偏上", rgb: JUST_ABOVE_3 },
    ];
    const viaLab = substitutesForLab(
      rgbToLab(BOUNDARY_BASE),
      inventory.map((it) => rgbToLab(it.rgb)),
    );
    expect(viaLab.map((it) => it.index)).toEqual([0]);
  });
});
