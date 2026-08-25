/**
 * D-ASM-2: the v0 way a real grid reaches the assemble canvas. These are
 * read-only build-time fixtures keyed by `PatternId` — no upload path (that is
 * WP-B03) and nothing here is ever persisted (BD19).
 *
 * Encoding is one string per row: `.` is an empty cell and `0`–`9` index into
 * the pattern's own `palette` in `catalog.ts`. Those are gallery colour codes,
 * never the `generic-5mm` G-codes (BD20) — the two code spaces collide and
 * mixing them is what BD20 forbids.
 *
 * The grid is the authority for bead counts: `catalog.ts` carries the numbers
 * this file produces, and `grids.test.ts` fails the moment they drift.
 */

import { BOARD_28, BOARD_56 } from "../algo/framing.ts";
import { createGrid, type Cell, type Grid } from "../algo/grid.ts";
import { asPatternId, type PatternId } from "../stores/ids.ts";
import { PATTERNS } from "./catalog.ts";

const EMPTY = ".";

/** What `<AssembleCanvas>` eats. Deliberately narrower than `PaletteEntry`. */
export interface FixtureSwatch {
  readonly code: string;
  readonly name: string;
  readonly hex: string;
}

export interface FixtureGrid {
  readonly grid: Grid;
  readonly palette: readonly FixtureSwatch[];
}

/**
 * Exported so a sick fixture dies in the test run rather than half-rendering a
 * board: an unknown character, a ragged row or an out-of-range palette index is
 * a throw, never a silently empty cell.
 */
export function decodeFixtureRows(
  rows: readonly string[],
  size: number,
  paletteSize: number,
): Grid {
  if (rows.length !== size) {
    throw new RangeError(`fixture 网格应有 ${size} 行，实际 ${rows.length} 行`);
  }
  const cells: Cell[] = [];
  rows.forEach((row, y) => {
    if (row.length !== size) {
      throw new RangeError(`fixture 第 ${y} 行应有 ${size} 列，实际 ${row.length} 列`);
    }
    for (const char of row) {
      if (char === EMPTY) {
        cells.push(null);
        continue;
      }
      const index = "0123456789".indexOf(char);
      if (index === -1) throw new RangeError(`fixture 第 ${y} 行出现未知字符 ${JSON.stringify(char)}`);
      if (index >= paletteSize) {
        throw new RangeError(`fixture 第 ${y} 行的色号下标 ${index} 超出色板长度 ${paletteSize}`);
      }
      cells.push(index);
    }
  });
  return createGrid(size, size, cells);
}

/* ---- geometry ---------------------------------------------------------
 * Hand-typing 3136 characters is not a plan, so the larger motifs are written
 * as shapes and the row strings fall out of them. Every predicate takes integer
 * cell coordinates and answers for that one cell.
 */

function inRect(x: number, y: number, x0: number, y0: number, x1: number, y1: number): boolean {
  return x >= x0 && x <= x1 && y >= y0 && y <= y1;
}

function inDisc(x: number, y: number, cx: number, cy: number, radius: number): boolean {
  const dx = x - cx;
  const dy = y - cy;
  return dx * dx + dy * dy <= radius * radius;
}

function inEllipse(x: number, y: number, cx: number, cy: number, rx: number, ry: number): boolean {
  const dx = (x - cx) / rx;
  const dy = (y - cy) / ry;
  return dx * dx + dy * dy <= 1;
}

function inRoundedRect(
  x: number,
  y: number,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
  radius: number,
): boolean {
  const nearestX = Math.min(Math.max(x, x0 + radius), x1 - radius);
  const nearestY = Math.min(Math.max(y, y0 + radius), y1 - radius);
  return inDisc(x, y, nearestX, nearestY, radius);
}

function paint(size: number, charAt: (x: number, y: number) => string): string[] {
  return Array.from({ length: size }, (_unusedRow, y) =>
    Array.from({ length: size }, (_unusedCell, x) => charAt(x, y)).join(""),
  );
}

/* ---- gal-slime-01 · 28×28 ------------------------------------------- */

// One slime, 12×10. 0 薄荷绿身体 / 1 深松绿底部阴影 / 2 纯白眼白 / 3 墨黑描边。
const SLIME_SPRITE: readonly string[] = [
  "....3333....",
  "..33000033..",
  ".3000000003.",
  ".3022002203.",
  ".3023003203.",
  "300000000003",
  "300003300003",
  "300111111003",
  "311111111113",
  ".3333333333.",
];

/** Top-left corners of the three squad members; they never overlap. */
const SLIME_ANCHORS: ReadonlyArray<readonly [number, number]> = [
  [1, 2],
  [15, 2],
  [8, 15],
];

function stamp(
  canvas: string[][],
  sprite: readonly string[],
  [originX, originY]: readonly [number, number],
): void {
  sprite.forEach((row, y) => {
    [...row].forEach((char, x) => {
      if (char === EMPTY) return;
      const line = canvas[originY + y];
      if (line === undefined || originX + x >= line.length) {
        throw new RangeError(`sprite 落在画布外：(${originX + x}, ${originY + y})`);
      }
      line[originX + x] = char;
    });
  });
}

function slimeRows(): string[] {
  const canvas = Array.from({ length: BOARD_28 }, () => new Array<string>(BOARD_28).fill(EMPTY));
  for (const anchor of SLIME_ANCHORS) stamp(canvas, SLIME_SPRITE, anchor);
  return canvas.map((row) => row.join(""));
}

/* ---- gal-lantern-04 · 28×28 ------------------------------------------ */

// 0 朱红灯身 / 1 明黄透光与流苏 / 2 墨黑骨架。
function lanternChar(x: number, y: number): string {
  // 提绳、上下灯盖与流苏：先画，它们压在灯身之外。
  if (inRect(x, y, 13, 0, 14, 2)) return "2";
  if (inRect(x, y, 10, 3, 17, 5)) return "2";
  if (inRect(x, y, 10, 23, 17, 25)) return "2";
  if (inRect(x, y, 13, 26, 14, 27)) return "1";

  if (!inEllipse(x, y, 13.5, 14.5, 10, 9)) return EMPTY;
  if (!inEllipse(x, y, 13.5, 14.5, 8.6, 7.7)) return "2";
  // 竖骨：把灯身分成几瓣，颜色之外再给一层形状线索。
  if (x === 9 || x === 18) return "2";
  if (inEllipse(x, y, 13.5, 14.5, 4, 5.6)) return "1";
  return "0";
}

function lanternRows(): string[] {
  return paint(BOARD_28, lanternChar);
}

/* ---- gal-arcade-05 · 56×56 ------------------------------------------ */

// 0 浅灰机身 / 1 墨黑外壳与十字键 / 2 朱红按键。

/** Body slab plus the two grip lobes, as one silhouette. */
function inArcadeBody(x: number, y: number): boolean {
  return (
    inRoundedRect(x, y, 5, 14, 50, 34, 8) ||
    inDisc(x, y, 14, 37, 9) ||
    inDisc(x, y, 41, 37, 9)
  );
}

const SHELL_THICKNESS = 2;

// Taking the shell as "within 2 of the outside" rather than as the gap between
// two inset silhouettes keeps it an even thickness and leaves no dark seam
// where the slab meets a grip.
function isArcadeShell(x: number, y: number): boolean {
  for (let dy = -SHELL_THICKNESS; dy <= SHELL_THICKNESS; dy += 1) {
    for (let dx = -SHELL_THICKNESS; dx <= SHELL_THICKNESS; dx += 1) {
      if (!inArcadeBody(x + dx, y + dy)) return true;
    }
  }
  return false;
}

const ARCADE_BUTTONS: ReadonlyArray<readonly [number, number]> = [
  [40, 22],
  [45, 27],
  [40, 32],
  [35, 27],
];

function arcadeChar(x: number, y: number): string {
  if (!inArcadeBody(x, y)) return EMPTY;
  if (isArcadeShell(x, y)) return "1";
  // D-pad cross.
  if (inRect(x, y, 10, 24, 22, 28) || inRect(x, y, 14, 20, 18, 32)) return "1";
  for (const [cx, cy] of ARCADE_BUTTONS) if (inDisc(x, y, cx, cy, 2.6)) return "2";
  // Start / select.
  if (inRect(x, y, 24, 25, 26, 26) || inRect(x, y, 30, 25, 32, 26)) return "1";
  return "0";
}

function arcadeRows(): string[] {
  return paint(BOARD_56, arcadeChar);
}

/* ---- registry -------------------------------------------------------- */

interface FixtureSpec {
  readonly size: number;
  readonly rows: () => readonly string[];
}

// gal-cakebox-03 is deliberately absent: 2260 beads do not fit a single 28×28
// board, multi-board assembly is post-v0 (round1-map §3), and it is the natural
// in-catalog case for the「暂无网格」state (D-ASM-12).
const SPECS: ReadonlyMap<string, FixtureSpec> = new Map([
  ["gal-slime-01", { size: BOARD_28, rows: slimeRows }],
  ["gal-lantern-04", { size: BOARD_28, rows: lanternRows }],
  ["gal-arcade-05", { size: BOARD_56, rows: arcadeRows }],
]);

/** Ids this file draws, in catalog order. Tests iterate this. */
export const GRIDDED_PATTERN_IDS: readonly PatternId[] = [...SPECS.keys()].map(asPatternId);

const cache = new Map<string, FixtureGrid>();

/**
 * Decoded on first read and kept — the grid is derived data, so it is rebuilt
 * from the fixture rather than stored, but rebuilding it on every render of a
 * 56×56 board would be wasteful.
 */
export function fixtureGridFor(patternId: PatternId): FixtureGrid | null {
  const cached = cache.get(patternId);
  if (cached !== undefined) return cached;

  const spec = SPECS.get(patternId);
  if (spec === undefined) return null;
  const pattern = PATTERNS.find((candidate) => candidate.id === patternId);
  if (pattern === undefined) return null;

  const decoded: FixtureGrid = {
    grid: decodeFixtureRows(spec.rows(), spec.size, pattern.palette.length),
    palette: pattern.palette.map(({ code, name, hex }) => ({ code, name, hex })),
  };
  cache.set(patternId, decoded);
  return decoded;
}
