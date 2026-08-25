import { describe, expect, it } from "vitest";

import { createGrid, occupiedCount, type Cell, type CellRef, type Grid } from "./grid.ts";
import {
  colorByColor,
  outlineInfill,
  rowByRow,
  splitSteps,
  SPLIT_MODES,
  tileSplit,
} from "./steps.ts";
import { seededInt, seededRandom } from "../test/random.ts";

/** `.` is an empty cell; any digit is that palette index. */
function grid(rows: readonly string[]): Grid {
  const width = rows[0]?.length ?? 0;
  const cells: Cell[] = [];
  for (const row of rows) {
    expect(row.length).toBe(width);
    for (const ch of row) cells.push(ch === "." ? null : Number.parseInt(ch, 10));
  }
  return createGrid(width, rows.length, cells);
}

function coords(cells: readonly CellRef[]): string[] {
  return cells.map((it) => `${it.x},${it.y}`);
}

describe("T-SPL-0 划分性质", () => {
  it("固定种子的 100 个含透明格网格，四模式都恰好覆盖全部非空格", () => {
    const next = seededRandom(0x5b1f00d);
    for (let round = 0; round < 100; round += 1) {
      const width = 1 + seededInt(next, 9);
      const height = 1 + seededInt(next, 9);
      const cells: Cell[] = [];
      for (let i = 0; i < width * height; i += 1) {
        cells.push(next() < 0.3 ? null : seededInt(next, 4));
      }
      const subject = createGrid(width, height, cells);
      const expected = new Set<string>();
      for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
          if (cells[y * width + x] !== null) expected.add(`${x},${y}`);
        }
      }

      for (const mode of SPLIT_MODES) {
        const seen: string[] = [];
        for (const step of splitSteps(subject, mode, { tileSize: 3 })) {
          expect(step.cells.length).toBeGreaterThan(0);
          for (const cell of step.cells) {
            expect(cell.x).toBeGreaterThanOrEqual(0);
            expect(cell.y).toBeGreaterThanOrEqual(0);
            expect(cell.x).toBeLessThan(width);
            expect(cell.y).toBeLessThan(height);
            expect(cells[cell.y * width + cell.x]).toBe(cell.color);
            seen.push(`${cell.x},${cell.y}`);
          }
        }
        expect(new Set(seen).size).toBe(seen.length);
        expect(new Set(seen)).toEqual(expected);
        expect(seen.length).toBe(occupiedCount(subject));
      }
    }
  });
});

describe("T-SPL-1 空网格与全透明网格", () => {
  it.each(SPLIT_MODES)("%s：0×0 与全透明都是 0 步骤", (mode) => {
    expect(splitSteps(createGrid(0, 0), mode)).toEqual([]);
    expect(splitSteps(createGrid(4, 3), mode)).toEqual([]);
  });
});

describe("T-SPL-2 确定性", () => {
  const subject = grid(["1.22", "133.", ".2.1"]);

  it.each(SPLIT_MODES)("%s：跑两遍逐字节相等", (mode) => {
    expect(JSON.stringify(splitSteps(subject, mode, { tileSize: 2 }))).toBe(
      JSON.stringify(splitSteps(subject, mode, { tileSize: 2 })),
    );
  });
});

describe("T-CBC-1 Color-by-Color 计数与 tie-break", () => {
  // A = 索引 5（1 格），B = 索引 1（5 格），C = 索引 2（5 格）。
  const subject = grid(["51111122222"]);

  it("默认升序：点缀色先行，同计数按色板索引升序", () => {
    const steps = colorByColor(subject);
    expect(steps.map((it) => it.color)).toEqual([5, 1, 2]);
    expect(steps.map((it) => it.cells.length)).toEqual([1, 5, 5]);
  });

  it("切降序：多的先行，同计数仍按色板索引升序", () => {
    const steps = colorByColor(subject, { order: "descending" });
    expect(steps.map((it) => it.color)).toEqual([1, 2, 5]);
    expect(steps.map((it) => it.cells.length)).toEqual([5, 5, 1]);
  });
});

describe("T-CBC-2 单色网格", () => {
  it("恰 1 步，格序行优先", () => {
    const steps = colorByColor(grid(["77", "7."]));
    expect(steps.length).toBe(1);
    expect(steps[0]!.color).toBe(7);
    expect(coords(steps[0]!.cells)).toEqual(["0,0", "1,0", "0,1"]);
  });
});

describe("T-TIL-1 Tile 顺序", () => {
  it("56×56 满网格恰 4 个 tile，顺序 (0,0)→(28,0)→(0,28)→(28,28)", () => {
    const subject = createGrid(56, 56, new Array<Cell>(56 * 56).fill(3));
    const steps = tileSplit(subject);
    expect(steps.length).toBe(4);
    expect(steps.map((it) => `${it.cells[0]!.x},${it.cells[0]!.y}`)).toEqual([
      "0,0",
      "28,0",
      "0,28",
      "28,28",
    ]);
    for (const step of steps) expect(step.cells.length).toBe(28 * 28);
    // 行优先：第一个 tile 的前两格是 (0,0) 与 (1,0)。
    expect(coords(steps[0]!.cells).slice(0, 2)).toEqual(["0,0", "1,0"]);
  });
});

describe("T-TIL-2 部分 tile 不补齐", () => {
  it("30×29 冻结为 28×28、2×28、28×1、2×1", () => {
    const subject = createGrid(30, 29, new Array<Cell>(30 * 29).fill(1));
    const steps = tileSplit(subject);
    expect(steps.map((it) => it.cells.length)).toEqual([28 * 28, 2 * 28, 28 * 1, 2 * 1]);
  });

  it("全透明的 tile 不产生步骤", () => {
    const cells = new Array<Cell>(30 * 29).fill(1);
    // 把右下角 2×1 的那个 tile 掏空。
    for (let y = 28; y < 29; y += 1) {
      for (let x = 28; x < 30; x += 1) cells[y * 30 + x] = null;
    }
    const steps = tileSplit(createGrid(30, 29, cells));
    expect(steps.map((it) => it.cells.length)).toEqual([28 * 28, 2 * 28, 28 * 1]);
  });
});

describe("T-OUT-1 实心块", () => {
  it("4×4 实心：外轮廓 12 格，再填充 4 格，顺序冻结", () => {
    const steps = outlineInfill(grid(["1111", "1111", "1111", "1111"]));
    expect(steps.map((it) => it.part)).toEqual(["outline", "fill"]);
    expect(coords(steps[0]!.cells)).toEqual([
      "0,0",
      "1,0",
      "2,0",
      "3,0",
      "0,1",
      "3,1",
      "0,2",
      "3,2",
      "0,3",
      "1,3",
      "2,3",
      "3,3",
    ]);
    expect(coords(steps[1]!.cells)).toEqual(["1,1", "2,1", "1,2", "2,2"]);
  });
});

describe("T-OUT-2 环形与洞", () => {
  it("7×7 挖空中心 3×3：外轮廓 24、内边界 16、填充 0", () => {
    const steps = outlineInfill(
      grid(["1111111", "1111111", "11...11", "11...11", "11...11", "1111111", "1111111"]),
    );
    expect(steps.map((it) => [it.part, it.cells.length])).toEqual([
      ["outline", 24],
      ["inner-border", 16],
    ]);
  });

  it("1 格宽的环（5×5 挖 3×3）：每格只出现一次且全归外轮廓", () => {
    const steps = outlineInfill(grid(["11111", "1...1", "1...1", "1...1", "11111"]));
    expect(steps.map((it) => [it.part, it.cells.length])).toEqual([["outline", 16]]);
    expect(new Set(coords(steps[0]!.cells)).size).toBe(16);
  });
});

describe("T-OUT-3 连通性是 4 邻接", () => {
  it("仅对角相触的两格算 2 个分量", () => {
    const steps = outlineInfill(grid(["1.", ".1"]));
    expect(steps.length).toBe(2);
    expect(steps.map((it) => it.group)).toEqual([0, 1]);
    expect(steps.map((it) => coords(it.cells))).toEqual([["0,0"], ["1,1"]]);
  });

  it("上下相邻的两格算 1 个分量", () => {
    const steps = outlineInfill(grid(["1.", "1."]));
    expect(steps.length).toBe(1);
    expect(coords(steps[0]!.cells)).toEqual(["0,0", "0,1"]);
  });
});

describe("T-OUT-4 分量顺序", () => {
  it("按首格最上、再最左排序", () => {
    const steps = outlineInfill(grid([".11.11", ".11.11", "......", "11....", "11...."]));
    expect(steps.map((it) => it.group)).toEqual([0, 1, 2]);
    expect(steps.map((it) => `${it.cells[0]!.x},${it.cells[0]!.y}`)).toEqual(["1,0", "4,0", "0,3"]);
  });
});

describe("T-ROW-1 Row-by-Row", () => {
  it("整行透明的行不产生步骤，行内自左而右", () => {
    const steps = rowByRow(grid(["1.2", "...", "3.4"]));
    expect(steps.length).toBe(2);
    expect(steps.map((it) => it.group)).toEqual([0, 2]);
    expect(coords(steps[0]!.cells)).toEqual(["0,0", "2,0"]);
    expect(coords(steps[1]!.cells)).toEqual(["0,2", "2,2"]);
  });

  it("每个非空行恰一步", () => {
    const subject = grid(["11", "22", "33"]);
    expect(rowByRow(subject).length).toBe(3);
  });
});

describe("tile 参数校验", () => {
  it("非正整数 tile 尺寸抛 RangeError", () => {
    expect(() => tileSplit(grid(["1"]), 0)).toThrow(RangeError);
    expect(() => tileSplit(grid(["1"]), 1.5)).toThrow(RangeError);
  });
});
