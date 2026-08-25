import { describe, expect, it } from "vitest";

import { buildBom, totalBeads } from "./bom.ts";
import { cellAt, createGrid, isEmptyGrid, occupiedCount, type Cell } from "./grid.ts";
import { GENERIC_5MM } from "./palette.ts";

function grid(rows: readonly string[]) {
  const width = rows[0]?.length ?? 0;
  const cells: Cell[] = [];
  for (const row of rows) for (const ch of row) cells.push(ch === "." ? null : Number(ch));
  return createGrid(width, rows.length, cells);
}

describe("T-BOM-1 冻结网格的行集", () => {
  const subject = grid(["0011", "0.11", "2211", "...1"]);

  it("(色号, 显示名, 颗数) 精确", () => {
    expect(buildBom(subject, GENERIC_5MM)).toEqual([
      { index: 1, code: "G02", displayName: "米白", count: 7 },
      { index: 0, code: "G01", displayName: "纯白", count: 3 },
      { index: 2, code: "G03", displayName: "浅灰", count: 2 },
    ]);
  });

  it("Σ颗数 == 非空格总数", () => {
    expect(totalBeads(buildBom(subject, GENERIC_5MM))).toBe(occupiedCount(subject));
    expect(occupiedCount(subject)).toBe(12);
  });
});

describe("T-BOM-2 空网格与单色网格", () => {
  it("0×0 与全透明都给出空 BOM", () => {
    expect(buildBom(createGrid(0, 0), GENERIC_5MM)).toEqual([]);
    expect(buildBom(createGrid(3, 3), GENERIC_5MM)).toEqual([]);
    expect(isEmptyGrid(createGrid(3, 3))).toBe(true);
  });

  it("单色网格恰 1 行", () => {
    const bom = buildBom(grid(["55", "5."]), GENERIC_5MM);
    expect(bom).toEqual([{ index: 5, code: "G06", displayName: "纯黑", count: 3 }]);
  });
});

describe("T-BOM-3 排序锁定", () => {
  it("颗数降序，同颗数按色板索引升序", () => {
    // 索引 4 与 索引 1 各 2 格，索引 7 有 3 格。
    const bom = buildBom(grid(["7774", "41.1"]), GENERIC_5MM);
    expect(bom.map((it) => [it.index, it.count])).toEqual([
      [7, 3],
      [1, 2],
      [4, 2],
    ]);
  });
});

describe("Grid 基本约束", () => {
  it("越界读取返回空格而不是抛错", () => {
    const subject = grid(["12", "34"]);
    expect(cellAt(subject, -1, 0)).toBeNull();
    expect(cellAt(subject, 0, 9)).toBeNull();
    expect(cellAt(subject, 1, 1)).toBe(4);
  });

  it("单元数与尺寸不符时拒绝构造", () => {
    expect(() => createGrid(2, 2, [1, 2, 3])).toThrow(RangeError);
    expect(() => createGrid(-1, 2)).toThrow(RangeError);
  });
});
