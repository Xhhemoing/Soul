import { describe, expect, it } from "vitest";

import { GRIDDED_PATTERN_IDS, decodeFixtureRows, fixtureGridFor } from "./grids.ts";
import { PATTERNS } from "./catalog.ts";
import { BOARD_28, BOARD_56 } from "../algo/framing.ts";
import { occupiedCount } from "../algo/grid.ts";
import { asPatternId } from "../stores/ids.ts";
import type { BoardKind } from "../stores/types.ts";

const SIDE: Partial<Record<BoardKind, number>> = {
  "square-28": BOARD_28,
  "square-56": BOARD_56,
};

// T-ASM-1 / D-ASM-2 §4.2: the grid is the authority for bead counts. When a
// fixture is redrawn this file is what says the catalog numbers went stale.
describe("fixture 网格与 catalog 一致（T-ASM-1）", () => {
  it("必做的两张网格都在，各证一种板型", () => {
    expect(GRIDDED_PATTERN_IDS).toContain(asPatternId("gal-slime-01"));
    expect(GRIDDED_PATTERN_IDS).toContain(asPatternId("gal-arcade-05"));
  });

  it.each(GRIDDED_PATTERN_IDS)("%s 的尺寸符合板型", (patternId) => {
    const pattern = PATTERNS.find((candidate) => candidate.id === patternId);
    const fixture = fixtureGridFor(patternId);
    expect(pattern).toBeDefined();
    expect(fixture).not.toBeNull();
    const side = SIDE[pattern!.board];
    expect(side).toBeDefined();
    expect(fixture!.grid.width).toBe(side);
    expect(fixture!.grid.height).toBe(side);
    expect(fixture!.grid.cells).toHaveLength(side! * side!);
  });

  it.each(GRIDDED_PATTERN_IDS)("%s 的逐色颗数与总数由网格派生", (patternId) => {
    const pattern = PATTERNS.find((candidate) => candidate.id === patternId)!;
    const { grid, palette } = fixtureGridFor(patternId)!;

    const counted = pattern.palette.map((_entry, index) =>
      grid.cells.reduce<number>((total, cell) => (cell === index ? total + 1 : total), 0),
    );
    expect(counted).toEqual(pattern.palette.map((entry) => entry.beads));
    expect(occupiedCount(grid)).toBe(pattern.beadCount);
    // Every declared colour is actually drawn: a palette entry at zero would
    // make the catalog claim a bead that never appears on the board.
    expect(counted.every((count) => count > 0)).toBe(true);
    expect(palette.map((swatch) => swatch.code)).toEqual(
      pattern.palette.map((entry) => entry.code),
    );
  });

  it("色板下标不越界，展示色板就是画廊码（BD20）", () => {
    for (const patternId of GRIDDED_PATTERN_IDS) {
      const { grid, palette } = fixtureGridFor(patternId)!;
      for (const cell of grid.cells) {
        if (cell === null) continue;
        expect(cell).toBeGreaterThanOrEqual(0);
        expect(cell).toBeLessThan(palette.length);
      }
      // generic-5mm 的 G 码形如 G07；画廊码里 G07 是苔绿，两套码空间不得混用。
      const pattern = PATTERNS.find((candidate) => candidate.id === patternId)!;
      expect(palette).toEqual(
        pattern.palette.map(({ code, name, hex }) => ({ code, name, hex })),
      );
    }
  });

  it("没有网格的图纸返回 null，立体拼豆图纸永不上网格", () => {
    expect(fixtureGridFor(asPatternId("gal-cakebox-03"))).toBeNull();
    expect(fixtureGridFor(asPatternId("gal-unknown-99"))).toBeNull();
    expect(GRIDDED_PATTERN_IDS).not.toContain(asPatternId("gal-cakebox-03"));
  });

  it("解码器对坏 fixture 抛错而不是悄悄留白", () => {
    expect(() => decodeFixtureRows(["0.", ".1"], 2, 2)).not.toThrow();
    expect(() => decodeFixtureRows(["0x", ".1"], 2, 2)).toThrow(/未知字符/);
    expect(() => decodeFixtureRows(["0.", ".9"], 2, 2)).toThrow(/超出色板长度/);
    expect(() => decodeFixtureRows(["0..", ".1"], 2, 2)).toThrow(/列/);
    expect(() => decodeFixtureRows(["0."], 2, 2)).toThrow(/行/);
  });

  it("同一张网格重复取到的是同一个对象，不会每次重解", () => {
    const first = fixtureGridFor(asPatternId("gal-slime-01"));
    expect(fixtureGridFor(asPatternId("gal-slime-01"))).toBe(first);
  });
});
