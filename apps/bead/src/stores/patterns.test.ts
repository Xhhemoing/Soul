import { describe, expect, it } from "vitest";

import { createGrid } from "../algo/grid.ts";
import { GENERIC_5MM } from "../algo/palette.ts";
import { mintProjectId } from "./ids.ts";
import {
  EMPTY_CELL,
  boardSourceOf,
  createPatternDoc,
  decodeCells,
  encodeCells,
  paletteForNamespace,
  paletteSwatches,
  rgbToHex,
  toPatternDoc,
} from "./patterns.ts";
import { GALLERY_PALETTE, GENERIC_5MM_PALETTE, type PatternDoc } from "./types.ts";

// D-UP-12 / T-UP-5. The encoding and the read-back check are pure functions on
// purpose: neither needs an IndexedDB to be wrong in an interesting way.

const OWNER = mintProjectId();

/** 3×2 with a hole: exercises row-major order and the `-1` sentinel together. */
function sampleGrid() {
  return createGrid(3, 2, [0, 1, null, 47, null, 2]);
}

function sampleDoc(overrides: Partial<PatternDoc> = {}): PatternDoc {
  return { ...createPatternDoc(OWNER, sampleGrid()), ...overrides };
}

describe("Grid ↔ Int16Array 编解码（D-UP-12）", () => {
  it("空格写 -1，其余写色板下标，行主序", () => {
    expect([...encodeCells(sampleGrid())]).toEqual([0, 1, EMPTY_CELL, 47, EMPTY_CELL, 2]);
  });

  it("往返回来还是同一张网格", () => {
    const grid = sampleGrid();
    const back = decodeCells(createPatternDoc(OWNER, grid));
    expect(back).toEqual(grid);
  });

  it("56×56 的文档是 3136 格、6272 字节", () => {
    const doc = createPatternDoc(OWNER, createGrid(56, 56));
    expect(doc.cells).toHaveLength(56 * 56);
    expect(doc.cells.byteLength).toBe(56 * 56 * 2);
  });

  it("出处只在给了的时候才写进文档", () => {
    expect(createPatternDoc(OWNER, sampleGrid()).provenance).toBeUndefined();
    expect(
      createPatternDoc(OWNER, sampleGrid(), { kind: "Photo", ditherApplied: true }).provenance,
    ).toEqual({ kind: "Photo", ditherApplied: true });
  });
});

describe("T-UP-5 读回校验：坏文档整篇弃，不半渲染", () => {
  it("好文档原样通过，多余的键被剥掉", () => {
    const smuggled = { ...sampleDoc(), steps: [{ cells: [] }], bom: [] };
    const checked = toPatternDoc(smuggled);
    expect(checked).not.toBeNull();
    expect(Object.keys(checked!).sort()).toEqual([
      "cells",
      "height",
      "paletteId",
      "projectId",
      "width",
    ]);
  });

  it("cells 长度与宽高不符 → null", () => {
    expect(toPatternDoc(sampleDoc({ cells: new Int16Array([0, 1, 2]) }))).toBeNull();
    expect(toPatternDoc(sampleDoc({ width: 4 }))).toBeNull();
  });

  it("越界的色板下标 → null（半张板比没有板更糟）", () => {
    const past = GENERIC_5MM.entries.length;
    expect(toPatternDoc(sampleDoc({ cells: new Int16Array([0, 1, -1, past, -1, 2]) }))).toBeNull();
    expect(toPatternDoc(sampleDoc({ cells: new Int16Array([0, 1, -1, -2, -1, 2]) }))).toBeNull();
  });

  it("未知或不可索引的 paletteId → null", () => {
    expect(toPatternDoc({ ...sampleDoc(), paletteId: "hama-h" })).toBeNull();
    // 画廊是「用户敲进来的码」的命名空间，没有稳定下标，做不了网格的色板。
    expect(toPatternDoc(sampleDoc({ paletteId: GALLERY_PALETTE }))).toBeNull();
    expect(paletteForNamespace(GALLERY_PALETTE)).toBeNull();
    expect(paletteForNamespace(GENERIC_5MM_PALETTE)).toBe(GENERIC_5MM);
  });

  it("不是项目 id、不是 Int16Array、不是对象都被拒", () => {
    expect(toPatternDoc(sampleDoc({ projectId: "gal-slime-01" as never }))).toBeNull();
    expect(toPatternDoc({ ...sampleDoc(), cells: [0, 1, -1, 47, -1, 2] })).toBeNull();
    expect(toPatternDoc(null)).toBeNull();
    expect(toPatternDoc("豆图")).toBeNull();
  });

  it("出处坏掉时只丢出处，不丢整篇", () => {
    const checked = toPatternDoc({ ...sampleDoc(), provenance: { kind: "抽象画" } });
    expect(checked).not.toBeNull();
    expect(checked!.provenance).toBeUndefined();
  });
});

describe("generic-5mm 适配（D-UP-14）", () => {
  it("rgb→hex 补零成小写六位", () => {
    expect(rgbToHex({ r: 0, g: 0, b: 0 })).toBe("#000000");
    expect(rgbToHex({ r: 183, g: 191, b: 198 })).toBe("#b7bfc6");
  });

  it("swatch 从色板现算，长度与色板一致", () => {
    const swatches = paletteSwatches(GENERIC_5MM);
    expect(swatches).toHaveLength(GENERIC_5MM.entries.length);
    expect(swatches[6]).toEqual({ code: "G07", name: "Silver", hex: "#b7bfc6" });
  });

  it("boardSourceOf 给出与 fixture 同形状的 {grid, palette}", () => {
    const board = boardSourceOf(sampleDoc());
    expect(board).not.toBeNull();
    expect(board!.grid).toEqual(sampleGrid());
    expect(board!.palette[0]).toEqual({ code: "G01", name: "White", hex: "#ffffff" });
  });
});
