import type { Rgb } from "../algo/color.ts";
import { findSubstitutes, type InventoryColor } from "../algo/substitutes.ts";
import { findPattern } from "./catalog.ts";
import { selectInProgress } from "./projects.ts";
import type { InventoryEntry, Pattern, Project } from "./types.ts";

/**
 * WP-B05 inventory selectors.
 *
 * D-INV-1: the v0 BOM source is the source pattern's `palette`, not
 * `buildBom`. The fixture patterns have no persisted grid (BD19 keeps grids off
 * disk), and `buildBom` speaks `generic-5mm` G-codes while every other surface —
 * detail page, stock form, the beads in the user's hand — speaks gallery codes.
 * BD20 pins the two code spaces apart until the first PR that persists a grid
 * introduces a palette namespace; blank projects (`sourcePatternId === null`)
 * simply have no BOM.
 *
 * D-INV-13: everything below is recomputed per read. None of it is persisted.
 */

/** Quota ceiling, mirroring the bounded-field precedent in round2-data §6.3. */
export const MAX_BEADS = 99_999;
export const MAX_CODE_LENGTH = 16;
export const MAX_NAME_LENGTH = 64;

/** Deterministic for a given state — no timestamp — so tests can assert the whole string. */
export const PURCHASE_FILE_NAME = "bead-purchase-list.txt";

const HEX_PATTERN = /^#?([0-9a-fA-F]{6})$/;

export interface RequirementRow {
  code: string;
  name: string;
  hex: string;
  required: number;
}

export interface ShortageRow {
  code: string;
  name: string;
  hex: string;
  required: number;
  inStock: number;
  shortage: number;
}

export interface SubstituteCandidateRow {
  code: string;
  name: string;
  hex: string;
  deltaE: number;
  /** Beads left over after the candidate's own colour is served. */
  remaining: number;
}

export interface SubstituteGroup {
  wanted: ShortageRow;
  candidates: SubstituteCandidateRow[];
}

export function selectStock(entries: readonly InventoryEntry[]): InventoryEntry[] {
  return [...entries].sort((a, b) => a.code.localeCompare(b.code));
}

export function totalBeads(entries: readonly InventoryEntry[]): number {
  return entries.reduce((sum, entry) => sum + entry.beads, 0);
}

/** The code is the identity key, so it is normalised once at the entry boundary. */
export function normalizeCode(code: string): string {
  return code.trim().toUpperCase();
}

/**
 * D-INV-10: the one hex→Rgb bridge, shared by the stock form and the substitute
 * pool. It lives here rather than in `algo/` because the layering is one-way —
 * stores may import `algo/`, `algo/` never imports stores.
 *
 * Strict six digits, `#` optional on input. Three-digit shorthand is rejected:
 * accepting it would make `#abc` and `#aabbcc` two spellings of one identity.
 */
export function parseHexColor(hex: string): Rgb | null {
  const match = HEX_PATTERN.exec(hex.trim());
  if (match === null) return null;
  const digits = match[1]!;
  return {
    r: Number.parseInt(digits.slice(0, 2), 16),
    g: Number.parseInt(digits.slice(2, 4), 16),
    b: Number.parseInt(digits.slice(4, 6), 16),
  };
}

/** Storage form: lower-case, always `#`-prefixed. */
export function normalizeHex(hex: string): string | null {
  const match = HEX_PATTERN.exec(hex.trim());
  return match === null ? null : `#${match[1]!.toLowerCase()}`;
}

export function clampBeads(beads: number): number {
  if (!Number.isFinite(beads)) return 0;
  return Math.min(MAX_BEADS, Math.max(0, Math.trunc(beads)));
}

/**
 * D-INV-2: demand is every `todo`/`active` project's source palette, summed by
 * exact code. Two instances of the same pattern want twice the beads, because
 * assembling it twice really does. Drafts and finished projects do not count.
 *
 * `lookup` exists so tests can supply patterns without touching the fixture.
 */
export function selectRequirements(
  projects: readonly Project[],
  lookup: (id: string) => Pattern | undefined = findPattern,
): RequirementRow[] {
  const rows = new Map<string, RequirementRow>();
  for (const project of selectInProgress(projects)) {
    if (project.sourcePatternId === null) continue;
    const pattern = lookup(project.sourcePatternId);
    if (pattern === undefined) continue;
    for (const entry of pattern.palette) {
      const existing = rows.get(entry.code);
      if (existing === undefined) {
        rows.set(entry.code, {
          code: entry.code,
          name: entry.name,
          hex: entry.hex,
          required: entry.beads,
        });
      } else {
        existing.required += entry.beads;
      }
    }
  }
  return [...rows.values()];
}

function stockByCode(inventory: readonly InventoryEntry[]): Map<string, number> {
  const totals = new Map<string, number>();
  for (const entry of inventory) {
    totals.set(entry.code, (totals.get(entry.code) ?? 0) + entry.beads);
  }
  return totals;
}

function requiredByCode(requirements: readonly RequirementRow[]): Map<string, number> {
  return new Map(requirements.map((row) => [row.code, row.required]));
}

/**
 * D-INV-8: an empty inventory is not a reason to stop. Zero stock means the
 * shortage is the whole requirement, which is exactly the first thing someone
 * buying beads wants to see.
 *
 * D-INV-12: shortage descending, then code ascending — the same ordering the
 * rust BOM uses for its rows.
 */
export function selectShortages(
  requirements: readonly RequirementRow[],
  inventory: readonly InventoryEntry[],
): ShortageRow[] {
  const stock = stockByCode(inventory);
  const rows: ShortageRow[] = [];
  for (const row of requirements) {
    const inStock = stock.get(row.code) ?? 0;
    const shortage = Math.max(0, row.required - inStock);
    if (shortage === 0) continue;
    rows.push({ ...row, inStock, shortage });
  }
  return rows.sort((a, b) =>
    a.shortage !== b.shortage ? b.shortage - a.shortage : a.code.localeCompare(b.code),
  );
}

/**
 * D-INV-9: the candidate pool follows the rust `check_stock` remainder rule —
 * a colour can only stand in with what is left after its own demand is served.
 * The pool keeps the inventory's own order so `findSubstitutes` tie-breaks on
 * inventory index (G6), and the threshold stays strictly `< 3` (G8).
 *
 * Entries whose hex cannot be parsed (a hand-edited blob) drop out of the pool
 * but stay in the stock list, where the code text still identifies them.
 */
export function selectSubstituteGroups(
  shortages: readonly ShortageRow[],
  requirements: readonly RequirementRow[],
  inventory: readonly InventoryEntry[],
): SubstituteGroup[] {
  const required = requiredByCode(requirements);
  return shortages.map((wanted) => {
    const wantedRgb = parseHexColor(wanted.hex);
    if (wantedRgb === null) return { wanted, candidates: [] };

    const pool: { entry: InventoryEntry; remaining: number; color: InventoryColor }[] = [];
    for (const entry of inventory) {
      if (entry.code === wanted.code) continue;
      const rgb = parseHexColor(entry.hex);
      if (rgb === null) continue;
      const remaining = Math.max(0, entry.beads - (required.get(entry.code) ?? 0));
      if (remaining <= 0) continue;
      pool.push({ entry, remaining, color: { code: entry.code, displayName: entry.name, rgb } });
    }

    const candidates = findSubstitutes(
      wantedRgb,
      pool.map((slot) => slot.color),
    ).map((hit) => {
      const slot = pool[hit.index]!;
      return {
        code: slot.entry.code,
        name: slot.entry.name,
        hex: slot.entry.hex,
        deltaE: hit.deltaE,
        remaining: slot.remaining,
      };
    });
    return { wanted, candidates };
  });
}

function formatDeltaE(deltaE: number): string {
  return deltaE.toFixed(2);
}

/**
 * D-INV-11 / BD15: plain text, no purchase links and no URLs of any kind. Each
 * line carries all four facts (code, name, hex, count) so the list is readable
 * without the app open, and nobody has to match a bead by colour alone.
 */
export function buildPurchaseText(
  shortages: readonly ShortageRow[],
  groups: readonly SubstituteGroup[],
): string {
  if (shortages.length === 0) return "";
  const byCode = new Map(groups.map((group) => [group.wanted.code, group]));
  const lines: string[] = ["拼豆采购清单（正在拼 / 待拼项目 vs 当前库存）", ""];
  let total = 0;
  for (const row of shortages) {
    total += row.shortage;
    lines.push(`${row.code} ${row.name} ${row.hex} 缺 ${row.shortage} 颗`);
    for (const candidate of byCode.get(row.code)?.candidates ?? []) {
      lines.push(
        `  可替代：${candidate.code} ${candidate.name} ${candidate.hex}` +
          `（ΔE00 ${formatDeltaE(candidate.deltaE)}，库存余 ${candidate.remaining} 颗）`,
      );
    }
  }
  lines.push("", `合计缺 ${total} 颗，共 ${shortages.length} 个色号`);
  return lines.join("\n");
}
