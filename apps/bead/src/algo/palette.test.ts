import { describe, expect, it } from "vitest";

import { ciede2000Rgb, rgbToLab } from "./color.ts";
import {
  entryAt,
  GENERIC_5MM,
  nearestEntry,
  nearestIndex,
  preparePalette,
  type Palette,
} from "./palette.ts";
import { seededInt, seededRandom } from "../test/random.ts";

describe("generic-5mm 色板", () => {
  it("条目齐备且没有重复 RGB", () => {
    const seen = new Set<string>();
    for (const item of GENERIC_5MM.entries) {
      expect(item.id).toMatch(/^G\d\d$/);
      expect(item.displayName.length).toBeGreaterThan(0);
      const key = `${item.rgb.r},${item.rgb.g},${item.rgb.b}`;
      expect(seen.has(key)).toBe(false);
      seen.add(key);
    }
    expect(GENERIC_5MM.entries.length).toBe(40);
  });

  // If two palette entries were closer than the substitute threshold the
  // "missing colour" advice would be noise, so the spacing is part of the data.
  it("最接近的两个色号也拉开 ΔE00 > 3", () => {
    const entries = GENERIC_5MM.entries;
    for (let i = 0; i < entries.length; i += 1) {
      for (let j = i + 1; j < entries.length; j += 1) {
        expect(ciede2000Rgb(entries[i]!.rgb, entries[j]!.rgb)).toBeGreaterThan(3);
      }
    }
  });
});

describe("T-PAL-1 色板内颜色映射回自身", () => {
  it("每个色号的 RGB 都以 ΔE00 = 0 命中自己", () => {
    const prepared = preparePalette(GENERIC_5MM);
    GENERIC_5MM.entries.forEach((item, index) => {
      const match = nearestEntry(rgbToLab(item.rgb), prepared);
      expect(match.index).toBe(index);
      expect(match.deltaE).toBe(0);
    });
  });
});

describe("T-PAL-2 平局取索引最小者", () => {
  const withDuplicate: Palette = {
    id: "tie-break-fixture",
    displayName: "平局夹具",
    entries: [
      { id: "T0", displayName: "零", rgb: { r: 0, g: 0, b: 0 } },
      { id: "T1", displayName: "一", rgb: { r: 255, g: 255, b: 255 } },
      { id: "T2", displayName: "二", rgb: { r: 10, g: 20, b: 30 } },
      { id: "T3", displayName: "三", rgb: { r: 120, g: 40, b: 200 } },
      { id: "T4", displayName: "四", rgb: { r: 200, g: 30, b: 40 } },
      { id: "T5", displayName: "五", rgb: { r: 30, g: 200, b: 40 } },
      { id: "T6", displayName: "六", rgb: { r: 40, g: 30, b: 200 } },
      { id: "T7", displayName: "七", rgb: { r: 120, g: 40, b: 200 } },
    ],
  };

  it("索引 3 与 7 同色时命中 3", () => {
    const prepared = preparePalette(withDuplicate);
    expect(nearestIndex({ r: 120, g: 40, b: 200 }, prepared)).toBe(3);
    // Also on a colour that is merely nearest to the duplicated pair.
    expect(nearestIndex({ r: 122, g: 42, b: 198 }, prepared)).toBe(3);
  });

  it("次优 ΔE 一并给出，平局时 margin 为 0", () => {
    const prepared = preparePalette(withDuplicate);
    const match = nearestEntry(rgbToLab({ r: 120, g: 40, b: 200 }), prepared);
    expect(match.deltaE).toBe(0);
    expect(match.runnerUpDeltaE).toBe(0);
  });
});

describe("T-PAL-3 健壮性", () => {
  it("固定种子的 1000 个随机 RGB 都落在合法索引上", () => {
    const prepared = preparePalette(GENERIC_5MM);
    const next = seededRandom(20260825);
    for (let i = 0; i < 1000; i += 1) {
      const rgb = {
        r: seededInt(next, 256),
        g: seededInt(next, 256),
        b: seededInt(next, 256),
      };
      const index = nearestIndex(rgb, prepared);
      expect(Number.isInteger(index)).toBe(true);
      expect(index).toBeGreaterThanOrEqual(0);
      expect(index).toBeLessThan(GENERIC_5MM.entries.length);
      expect(entryAt(GENERIC_5MM, index).id).toMatch(/^G\d\d$/);
    }
  });

  it("单条色板永远命中那一条，次优为 Infinity", () => {
    const single: Palette = {
      id: "single",
      displayName: "单色",
      entries: [{ id: "S0", displayName: "唯一", rgb: { r: 12, g: 34, b: 56 } }],
    };
    const prepared = preparePalette(single);
    const match = nearestEntry(rgbToLab({ r: 200, g: 200, b: 200 }), prepared);
    expect(match.index).toBe(0);
    expect(match.runnerUpDeltaE).toBe(Infinity);
  });

  it("空色板抛错而不是返回哨兵索引", () => {
    const prepared = preparePalette({ id: "empty", displayName: "空", entries: [] });
    expect(() => nearestEntry(rgbToLab({ r: 0, g: 0, b: 0 }), prepared)).toThrow(/EmptyPalette/);
  });

  it("越界索引取条目会抛 RangeError", () => {
    expect(() => entryAt(GENERIC_5MM, 999)).toThrow(RangeError);
  });
});
