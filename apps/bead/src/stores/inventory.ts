import { buildBom } from "../algo/bom.ts";
import type { Rgb } from "../algo/color.ts";
import { GENERIC_5MM } from "../algo/palette.ts";
import { findSubstitutes, type InventoryColor } from "../algo/substitutes.ts";
import { findPattern } from "./catalog.ts";
import { boardSourceOf, rgbToHex } from "./patterns.ts";
import { selectInProgress } from "./projects.ts";
import {
  GALLERY_PALETTE,
  GENERIC_5MM_PALETTE,
  PALETTE_NAMESPACE_LABEL,
  type InventoryEntry,
  type PaletteNamespaceId,
  type Pattern,
  type PatternDoc,
  type Project,
} from "./types.ts";

/**
 * WP-B05 inventory selectors, carried into BD20's palette namespaces.
 *
 * D-INV-1 held the BOM source at the source pattern's `palette` until a palette
 * namespace existed, because `buildBom` speaks `generic-5mm` G-codes while the
 * detail page, the stock form and the beads in the user's hand speak gallery
 * codes — and the two spaces really collide (gallery `G07 苔绿 #4c7a44` vs
 * `generic-5mm` `G07 Silver #B7BFC6`). D-UP-15 introduces the namespace, so the
 * other half of D-INV-1 lands here: converted projects get their requirements
 * from `buildBom(grid, GENERIC_5MM)`, tagged `generic-5mm`, and nothing in the
 * gallery path changes.
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

/** BD20 identity key. Codes are only unique inside their own palette. */
export interface PaletteCode {
  paletteId: PaletteNamespaceId;
  code: string;
}

export interface RequirementRow extends PaletteCode {
  name: string;
  hex: string;
  required: number;
}

export interface ShortageRow extends RequirementRow {
  inStock: number;
  shortage: number;
}

export interface SubstituteCandidateRow extends PaletteCode {
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

/** Joins the two halves of the identity key into one map key. */
export function paletteCodeKey({ paletteId, code }: PaletteCode): string {
  return `${paletteId}\u0000${code}`;
}

export function namespaceLabel(paletteId: PaletteNamespaceId): string {
  return PALETTE_NAMESPACE_LABEL[paletteId];
}

/** Namespace first, then code: rows from one palette stay together in the list. */
export function selectStock(entries: readonly InventoryEntry[]): InventoryEntry[] {
  return [...entries].sort(
    (a, b) => a.paletteId.localeCompare(b.paletteId) || a.code.localeCompare(b.code),
  );
}

export function totalBeads(entries: readonly InventoryEntry[]): number {
  return entries.reduce((sum, entry) => sum + entry.beads, 0);
}

/** The code is half the identity key, so it is normalised once at the entry boundary. */
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

function addRequirement(rows: Map<string, RequirementRow>, row: RequirementRow): void {
  const key = paletteCodeKey(row);
  const existing = rows.get(key);
  if (existing === undefined) rows.set(key, { ...row });
  else existing.required += row.required;
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
      addRequirement(rows, {
        paletteId: GALLERY_PALETTE,
        code: entry.code,
        name: entry.name,
        hex: entry.hex,
        required: entry.beads,
      });
    }
  }
  return [...rows.values()];
}

/**
 * §4.3: a converted project's BOM is `buildBom(grid, GENERIC_5MM)`, computed
 * from the stored document and thrown away again (D-INV-13). Names and hexes
 * come off the palette entries, so nothing about the colour table is duplicated
 * into storage.
 */
export function conversionRequirements(docs: readonly PatternDoc[]): RequirementRow[] {
  const rows = new Map<string, RequirementRow>();
  for (const doc of docs) {
    const board = boardSourceOf(doc);
    if (board === null) continue;
    for (const row of buildBom(board.grid, GENERIC_5MM)) {
      const entry = GENERIC_5MM.entries[row.index];
      if (entry === undefined) continue;
      addRequirement(rows, {
        paletteId: GENERIC_5MM_PALETTE,
        code: row.code,
        name: row.displayName,
        hex: rgbToHex(entry.rgb),
        required: row.count,
      });
    }
  }
  return [...rows.values()];
}

/** Two demand sources, one list. Rows only merge when both halves of the key match. */
export function mergeRequirements(...sources: readonly RequirementRow[][]): RequirementRow[] {
  const rows = new Map<string, RequirementRow>();
  for (const source of sources) for (const row of source) addRequirement(rows, row);
  return [...rows.values()];
}

function stockByCode(inventory: readonly InventoryEntry[]): Map<string, number> {
  const totals = new Map<string, number>();
  for (const entry of inventory) {
    const key = paletteCodeKey(entry);
    totals.set(key, (totals.get(key) ?? 0) + entry.beads);
  }
  return totals;
}

function requiredByCode(requirements: readonly RequirementRow[]): Map<string, number> {
  return new Map(requirements.map((row) => [paletteCodeKey(row), row.required]));
}

/**
 * D-INV-8: an empty inventory is not a reason to stop. Zero stock means the
 * shortage is the whole requirement, which is exactly the first thing someone
 * buying beads wants to see.
 *
 * D-INV-12: shortage descending, then code ascending — the same ordering the
 * rust BOM uses for its rows. Namespace breaks a code tie so the order stays
 * total once two palettes are in play.
 */
export function selectShortages(
  requirements: readonly RequirementRow[],
  inventory: readonly InventoryEntry[],
): ShortageRow[] {
  const stock = stockByCode(inventory);
  const rows: ShortageRow[] = [];
  for (const row of requirements) {
    const inStock = stock.get(paletteCodeKey(row)) ?? 0;
    const shortage = Math.max(0, row.required - inStock);
    if (shortage === 0) continue;
    rows.push({ ...row, inStock, shortage });
  }
  return rows.sort(
    (a, b) =>
      b.shortage - a.shortage ||
      a.code.localeCompare(b.code) ||
      a.paletteId.localeCompare(b.paletteId),
  );
}

/**
 * D-INV-9: the candidate pool follows the rust `check_stock` remainder rule —
 * a colour can only stand in with what is left after its own demand is served.
 * The pool keeps the inventory's own order so `findSubstitutes` tie-breaks on
 * inventory index (G6), and the threshold stays strictly `< 3` (G8).
 *
 * §4.4: the pool never crosses a namespace. A gallery bead cannot stand in for
 * a `generic-5mm` one — cross-palette substitution is a real feature that
 * round1-map §4 deferred out of B05, and letting it happen by accident here
 * would be that feature with none of its checks.
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
      if (entry.paletteId !== wanted.paletteId) continue;
      if (entry.code === wanted.code) continue;
      const rgb = parseHexColor(entry.hex);
      if (rgb === null) continue;
      const remaining = Math.max(0, entry.beads - (required.get(paletteCodeKey(entry)) ?? 0));
      if (remaining <= 0) continue;
      pool.push({ entry, remaining, color: { code: entry.code, displayName: entry.name, rgb } });
    }

    const candidates = findSubstitutes(
      wantedRgb,
      pool.map((slot) => slot.color),
    ).map((hit) => {
      const slot = pool[hit.index]!;
      return {
        paletteId: slot.entry.paletteId,
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
 * line carries all five facts (namespace, code, name, hex, count) so the list
 * is readable without the app open, nobody has to match a bead by colour alone,
 * and「G07」on its own is never asked to mean two different beads (§4.5).
 */
export function buildPurchaseText(
  shortages: readonly ShortageRow[],
  groups: readonly SubstituteGroup[],
): string {
  if (shortages.length === 0) return "";
  const byCode = new Map(groups.map((group) => [paletteCodeKey(group.wanted), group]));
  const lines: string[] = ["拼豆采购清单（正在拼 / 待拼项目 vs 当前库存）", ""];
  let total = 0;
  for (const row of shortages) {
    total += row.shortage;
    lines.push(`【${namespaceLabel(row.paletteId)}】${row.code} ${row.name} ${row.hex} 缺 ${row.shortage} 颗`);
    for (const candidate of byCode.get(paletteCodeKey(row))?.candidates ?? []) {
      lines.push(
        `  可替代：【${namespaceLabel(candidate.paletteId)}】${candidate.code} ${candidate.name} ${candidate.hex}` +
          `（ΔE00 ${formatDeltaE(candidate.deltaE)}，库存余 ${candidate.remaining} 颗）`,
      );
    }
  }
  lines.push("", `合计缺 ${total} 颗，共 ${shortages.length} 个色号`);
  return lines.join("\n");
}
