import { describe, expect, it } from "vitest";

import { buildBom } from "../../algo/bom.ts";
import { createGrid } from "../../algo/grid.ts";
import {
  GENERIC_5MM,
  entryAt,
  nearestIndex,
  preparePalette,
  type Palette,
} from "../../algo/palette.ts";
import { PATTERNS } from "../../fixtures/catalog.ts";
import { fixtureGridFor } from "../../fixtures/grids.ts";
import { asPatternId } from "../../stores/ids.ts";
import { parseHexColor } from "../../stores/inventory.ts";
import {
  FORK_TITLE_SUFFIX,
  forkColorOf,
  forkFixture,
  forkGrid,
  forkTitle,
  mergedTargets,
  quantizeGalleryPalette,
} from "./fork.ts";

/**
 * T-GAL-1 / T-GAL-2: the quantisation, with no DOM in the room.
 *
 * The table below was produced by running the implementation once and reading
 * every row — that is the point of writing it out as a literal. A change to
 * `GENERIC_5MM`, to a gallery hex or to the ΔE00 code moves a fork's colours
 * under people who already forked, so it has to fail here first.
 */
const EXPECTED_MAP: Readonly<Record<string, readonly number[]>> = {
  "gal-slime-01": [27, 26, 0, 8],
  "gal-torii-02": [14, 40, 26, 8],
  "gal-cakebox-03": [10, 0, 40, 2],
  "gal-lantern-04": [14, 21, 8],
  "gal-arcade-05": [2, 8, 14],
  "gal-gift-06": [32, 21, 8, 0],
  "gal-mochi-07": [42, 10, 0, 24, 8],
  "gal-mush-08": [14, 0, 8, 40],
  "gal-quilt-09": [33, 1, 22, 14],
};

/** The same table said in G-codes, which is what a person can actually check. */
const EXPECTED_CODES: Readonly<Record<string, string>> = {
  "#1b1b1f": "G09",
  "#2f3a72": "G34",
  "#2f7d55": "G27",
  "#3f7fbf": "G33",
  "#4c7a44": "G27",
  "#7fd6a2": "G28",
  "#9fc55a": "G25",
  "#a9713f": "G43",
  "#c9ccd4": "G03",
  "#d8412f": "G15",
  "#e0a526": "G23",
  "#e8c97a": "G41",
  "#f2a7c3": "G11",
  "#f2ead8": "G02",
  "#f5d13b": "G22",
  "#ffffff": "G01",
};

describe("T-GAL-1 画廊 → generic-5mm 量化表（D-GAL-6）", () => {
  it("每张图纸的逐条映射就是这张字面表", () => {
    const actual = Object.fromEntries(
      PATTERNS.map((pattern) => [pattern.id, quantizeGalleryPalette(pattern.palette)]),
    );
    expect(actual).toEqual(EXPECTED_MAP);
  });

  it("表里每个下标对应的 G 码也钉死", () => {
    const actual = Object.fromEntries(
      PATTERNS.flatMap((pattern) =>
        pattern.palette.map((entry) => [entry.hex, entryAt(GENERIC_5MM, forkColorOf(entry.hex)).id]),
      ),
    );
    expect(actual).toEqual(EXPECTED_CODES);
  });

  it("#RRGGBB 解析对全部画廊颜色正确，坏值抛错而不是悄悄变白", () => {
    for (const pattern of PATTERNS) {
      for (const entry of pattern.palette) {
        const rgb = parseHexColor(entry.hex)!;
        const value = Number.parseInt(entry.hex.slice(1), 16);
        expect(rgb).toEqual({
          r: (value >> 16) & 255,
          g: (value >> 8) & 255,
          b: value & 255,
        });
      }
    }
    expect(forkColorOf("#FFFFFF")).toBe(forkColorOf("#ffffff"));
    for (const bad of ["#abc", "红色", "", "#12345g"]) {
      expect(() => forkColorOf(bad)).toThrow(/不是 #RRGGBB/);
    }
  });

  it("并列取最小下标，且 fork 不另立一套最近邻（G2/G6）", () => {
    const twins: Palette = {
      id: "twins",
      displayName: "并列",
      entries: [
        { id: "T1", displayName: "先来的", rgb: { r: 10, g: 20, b: 30 } },
        { id: "T2", displayName: "同色", rgb: { r: 10, g: 20, b: 30 } },
      ],
    };
    expect(nearestIndex({ r: 10, g: 20, b: 30 }, preparePalette(twins))).toBe(0);

    const prepared = preparePalette(GENERIC_5MM);
    for (const pattern of PATTERNS) {
      expect(quantizeGalleryPalette(pattern.palette)).toEqual(
        pattern.palette.map((entry) => nearestIndex(parseHexColor(entry.hex)!, prepared)),
      );
    }
  });

  it("量化结果全部落在 generic-5mm 的下标域内（toPatternDoc 的越界检查因此天然通过）", () => {
    for (const indices of Object.values(EXPECTED_MAP)) {
      for (const index of indices) {
        expect(index).toBeGreaterThanOrEqual(0);
        expect(index).toBeLessThan(GENERIC_5MM.entries.length);
      }
    }
  });
});

describe("T-GAL-2 forkGrid 保形与合并计数", () => {
  it("宽高与空格原位不动，实格按映射表重写", () => {
    const grid = createGrid(3, 2, [0, null, 1, 2, null, 0]);
    const forked = forkGrid(grid, [7, 8, 9]);
    expect(forked.width).toBe(3);
    expect(forked.height).toBe(2);
    expect(forked.cells).toEqual([7, null, 8, 9, null, 7]);
  });

  it("映射表覆盖不到的下标是抛错，不是悄悄留白", () => {
    expect(() => forkGrid(createGrid(1, 1, [3]), [0, 1])).toThrow(/不在映射表内/);
  });

  it("两条画廊色映到同一 G 码时合并，颗数相加", () => {
    // 深松绿 #2f7d55 与苔绿 #4c7a44 在 catalog 里是两张图纸的两条色，ΔE00 下都
    // 落在 G27 Dark Green——把它们放进同一张色板就是最小的合并案例。
    const palette = [{ hex: "#2f7d55" }, { hex: "#4c7a44" }, { hex: "#ffffff" }];
    const map = quantizeGalleryPalette(palette);
    expect(map).toEqual([26, 26, 0]);
    expect(mergedTargets(map)).toEqual([26]);

    const grid = createGrid(3, 2, [0, 0, 1, 1, 1, 2]);
    const bom = buildBom(forkGrid(grid, map), GENERIC_5MM);
    expect(bom).toEqual([
      { index: 26, code: "G27", displayName: "Dark Green", count: 5 },
      { index: 0, code: "G01", displayName: "White", count: 1 },
    ]);
  });

  it("没有合并时 mergedTargets 是空的", () => {
    expect(mergedTargets(EXPECTED_MAP["gal-slime-01"]!)).toEqual([]);
    expect(mergedTargets(EXPECTED_MAP["gal-quilt-09"]!)).toEqual([]);
  });

  it("forkFixture 把整张 fixture 网格搬过去，颗数一颗不多一颗不少", () => {
    for (const id of ["gal-slime-01", "gal-torii-02", "gal-quilt-09"] as const) {
      const fixture = fixtureGridFor(asPatternId(id))!;
      const pattern = PATTERNS.find((candidate) => candidate.id === asPatternId(id))!;
      const { grid, map } = forkFixture(fixture);

      expect(map).toEqual(EXPECTED_MAP[id]);
      expect(grid.width).toBe(fixture.grid.width);
      expect(grid.height).toBe(fixture.grid.height);
      expect(grid.cells.filter((cell) => cell !== null)).toHaveLength(pattern.beadCount);
      expect(
        grid.cells.every((cell, index) => (fixture.grid.cells[index] === null) === (cell === null)),
      ).toBe(true);
    }
  });
});

describe("出处只进标题（D-GAL-7 / R-GAL-2）", () => {
  it("标题是「原题（Fork）」", () => {
    expect(forkTitle("史莱姆小队")).toBe(`史莱姆小队${FORK_TITLE_SUFFIX}`);
    expect(forkTitle("  夏日鸟居  ")).toBe(`夏日鸟居${FORK_TITLE_SUFFIX}`);
  });

  it("含后缀不超过 64（D-UP-11 / D-ED-4 同限）", () => {
    const long = forkTitle("图".repeat(200));
    expect(long).toHaveLength(64);
    expect(long.endsWith(FORK_TITLE_SUFFIX)).toBe(true);
  });
});
