/**
 * The `PatternDoc` half of the `bead-v1` contract (round2-data §5.2): pure
 * functions only, so the encoding, the read-back check and the palette
 * adaptation can all be tested without an IndexedDB in the room.
 *
 * D-UP-12: a grid reaches disk as a row-major `Int16Array` with `-1` for an
 * empty cell. Structured clone stores that as a typed array — 56×56 is 3136
 * cells, about 6.3KB — where the same grid as JSON would be several times that
 * and would arrive back as a plain array of numbers plus nulls.
 */

import type { Rgb } from "../algo/color.ts";
import { createGrid, type Cell, type Grid } from "../algo/grid.ts";
import { GENERIC_5MM, type Palette } from "../algo/palette.ts";
import { isProjectId, type ProjectId } from "./ids.ts";
import {
  GENERIC_5MM_PALETTE,
  isPaletteNamespaceId,
  type PaletteNamespaceId,
  type PatternDoc,
  type PatternProvenance,
} from "./types.ts";

/** The sentinel an empty cell is written as. Palette indices are `≥ 0`. */
export const EMPTY_CELL = -1;

/** What `<AssembleCanvas>` eats — the same shape `fixtures/grids.ts` produces. */
export interface BoardSwatch {
  readonly code: string;
  readonly name: string;
  readonly hex: string;
}

export interface BoardSource {
  readonly grid: Grid;
  readonly palette: readonly BoardSwatch[];
}

/**
 * Only `generic-5mm` names an indexable palette. `gallery` is a namespace for
 * codes a person typed in, not a colour table with stable indices, so a pattern
 * document claiming it has no way to resolve its own cells and is rejected.
 */
export function paletteForNamespace(id: PaletteNamespaceId): Palette | null {
  return id === GENERIC_5MM_PALETTE ? GENERIC_5MM : null;
}

export function rgbToHex({ r, g, b }: Rgb): string {
  return `#${[r, g, b].map((channel) => channel.toString(16).padStart(2, "0")).join("")}`;
}

/** Swatches are recomputed from the palette on every read, never stored (D-UP-12). */
export function paletteSwatches(palette: Palette): BoardSwatch[] {
  return palette.entries.map((entry) => ({
    code: entry.id,
    name: entry.displayName,
    hex: rgbToHex(entry.rgb),
  }));
}

export function encodeCells(grid: Grid): Int16Array {
  const cells = new Int16Array(grid.width * grid.height);
  for (let index = 0; index < cells.length; index += 1) {
    cells[index] = grid.cells[index] ?? EMPTY_CELL;
  }
  return cells;
}

export function createPatternDoc(
  projectId: ProjectId,
  grid: Grid,
  provenance?: PatternProvenance,
): PatternDoc {
  return {
    projectId,
    paletteId: GENERIC_5MM_PALETTE,
    width: grid.width,
    height: grid.height,
    cells: encodeCells(grid),
    ...(provenance === undefined ? {} : { provenance }),
  };
}

function isSize(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

/**
 * `instanceof` is bound to a realm and a structured clone is not: the value
 * IndexedDB hands back is a genuine `Int16Array`, but not always one built from
 * the same constructor this module closed over. The brand check is the portable
 * question. A plain array is still refused — the contract says typed array, and
 * loosening that would let a hand-written JSON blob in through the same door.
 */
function asInt16Array(value: unknown): Int16Array | null {
  if (Object.prototype.toString.call(value) !== "[object Int16Array]") return null;
  return Int16Array.from(value as ArrayLike<number>);
}

function toProvenance(value: unknown): PatternProvenance | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Record<string, unknown>;
  const kind = candidate["kind"];
  if (kind !== "PixelArt" && kind !== "Photo") return null;
  if (typeof candidate["ditherApplied"] !== "boolean") return null;
  return { kind, ditherApplied: candidate["ditherApplied"] };
}

/**
 * D-UP-12 read-back check, in the spirit of `parsePersistedState`: a document
 * whose cell count disagrees with its own dimensions, or that indexes past the
 * palette it names, is discarded whole rather than half-rendered. Half a board
 * is worse than none — the missing beads look like empty cells and the user
 * would assemble them.
 *
 * The returned document is rebuilt field by field, so extra keys a future build
 * (or a hand-edited devtools session) left behind do not travel any further.
 */
export function toPatternDoc(value: unknown): PatternDoc | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Record<string, unknown>;

  const projectId = candidate["projectId"];
  if (typeof projectId !== "string" || !isProjectId(projectId)) return null;

  const paletteId = candidate["paletteId"];
  if (!isPaletteNamespaceId(paletteId)) return null;
  const palette = paletteForNamespace(paletteId);
  if (palette === null) return null;

  const { width, height } = candidate;
  if (!isSize(width) || !isSize(height)) return null;

  const cells = asInt16Array(candidate["cells"]);
  if (cells === null || cells.length !== width * height) return null;
  for (const cell of cells) {
    if (cell === EMPTY_CELL) continue;
    if (cell < 0 || cell >= palette.entries.length) return null;
  }

  const provenance = toProvenance(candidate["provenance"]);
  return {
    projectId,
    paletteId,
    width,
    height,
    cells,
    ...(provenance === null ? {} : { provenance }),
  };
}

export function decodeCells(doc: PatternDoc): Grid {
  const cells: Cell[] = Array.from(doc.cells, (cell) => (cell === EMPTY_CELL ? null : cell));
  return createGrid(doc.width, doc.height, cells);
}

/**
 * D-UP-14: the adapter that lets a converted project walk through the same door
 * a gallery fixture does. Returns null for a document the read-back check
 * already rejected, so callers get the「没有豆图网格」state instead of a crash.
 */
export function boardSourceOf(doc: PatternDoc): BoardSource | null {
  const palette = paletteForNamespace(doc.paletteId);
  if (palette === null) return null;
  return { grid: decodeCells(doc), palette: paletteSwatches(palette) };
}
