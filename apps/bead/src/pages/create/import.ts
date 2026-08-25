/**
 * Page-private pure functions for the two import panels (D-IE-20, the
 * `session.ts` / `editor.ts` precedent): what a file looks like from its first
 * bytes, and the write sequence a parsed `.beadproj` turns into.
 *
 * Nothing here renders, and nothing here decodes an image: `algo/decode.ts`
 * stays the one byte→pixel door in the app (D-IE-17).
 */

import { createGrid, type Cell } from "../../algo/grid.ts";
import type { BeadprojDocument, BeadprojEntry, BeadprojEntryError } from "../../schema/beadproj.ts";
import { mintProjectId, type PatternId, type ProjectId } from "../../stores/ids.ts";
import { normalizeCode, paletteCodeKey } from "../../stores/inventory.ts";
import { EMPTY_CELL, createPatternDoc } from "../../stores/patterns.ts";
import type { ProgressCursorInput } from "../../stores/store.tsx";
import type { InventoryEntry, PatternDoc, Project } from "../../stores/types.ts";

/**
 * D-IE-18: the sniff reads at most 16 bytes and interprets none of the rest.
 * `.pat` and `.gamedev` have no public specification and no real sample in the
 * repo (round1-map §5), so v0 recognises them and stops — inventing a parser
 * would produce a grid nobody can check.
 */
export const SNIFF_BYTES = 16;

export type SniffedKind = "png" | "jpeg" | "json" | "unknown";

const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47];
const JPEG_MAGIC = [0xff, 0xd8, 0xff];
const UTF8_BOM = [0xef, 0xbb, 0xbf];
const WHITESPACE = new Set([0x09, 0x0a, 0x0d, 0x20]);
const OPENING_BRACE = 0x7b;

function startsWith(bytes: Uint8Array, magic: readonly number[]): boolean {
  if (bytes.length < magic.length) return false;
  return magic.every((byte, index) => bytes[index] === byte);
}

export function sniffBytes(bytes: Uint8Array): SniffedKind {
  if (startsWith(bytes, PNG_MAGIC)) return "png";
  if (startsWith(bytes, JPEG_MAGIC)) return "jpeg";

  // A byte-order mark and leading whitespace are not content, and an editor
  // that saved either of them should not turn a `.beadproj` into an unknown
  // format. Past those, the first character decides.
  let cursor = startsWith(bytes, UTF8_BOM) ? UTF8_BOM.length : 0;
  while (cursor < bytes.length && WHITESPACE.has(bytes[cursor]!)) cursor += 1;
  return bytes[cursor] === OPENING_BRACE ? "json" : "unknown";
}

/** Reads the head of the file only — a 15-byte file is not an out-of-range read. */
export async function sniffFile(file: File): Promise<SniffedKind> {
  const head = await file.slice(0, SNIFF_BYTES).arrayBuffer();
  return sniffBytes(new Uint8Array(head));
}

export function hasBeadprojExtension(name: string): boolean {
  return name.toLowerCase().endsWith(".beadproj");
}

export interface BeadprojImportDeps {
  /** D-UP-10's order, reused: the document is awaited before the project exists. */
  readonly savePatternDoc: (doc: PatternDoc) => Promise<void>;
  readonly addProject: (project: Project) => void;
  readonly upsertProgress: (cursor: ProgressCursorInput) => void;
  readonly addInventoryEntry: (entry: InventoryEntry) => void;
  /** The stock already on this machine; local wins on a `(paletteId, code)` clash. */
  readonly inventory: readonly InventoryEntry[];
  readonly mintId?: () => ProjectId;
}

export interface BeadprojImportResult {
  /** Entries the file claimed, rejected ones included. */
  readonly total: number;
  readonly imported: number;
  /** 1-based position of the entry whose write failed, if one did. */
  readonly failedIndex: number | null;
  readonly entryErrors: readonly BeadprojEntryError[];
  readonly inventoryAdded: number;
  readonly inventorySkipped: number;
  readonly inventoryDropped: number;
  readonly projectIds: readonly ProjectId[];
}

function toPatternDocFor(id: ProjectId, entry: BeadprojEntry): PatternDoc | null {
  const pattern = entry.pattern;
  if (pattern === undefined) return null;
  // The JSON array becomes an `Int16Array` through the store's own codec
  // (D-IE-5): a second encoder is a second thing that can disagree with disk.
  const cells: Cell[] = pattern.cells.map((cell) => (cell === EMPTY_CELL ? null : cell));
  const grid = createGrid(pattern.width, pattern.height, cells);
  return createPatternDoc(id, grid, pattern.provenance);
}

function toProject(id: ProjectId, entry: BeadprojEntry): Project {
  const { title, sourcePatternId, status, createdAt, backdrop, backdropColor } = entry.project;
  return {
    id,
    title,
    // The validator already proved this is null or a `gal-` id; a project keeps
    // the reference even when the catalog no longer has that pattern, because
    // both consumers of a dangling reference already degrade gracefully.
    sourcePatternId: sourcePatternId as PatternId | null,
    status,
    createdAt,
    backdrop,
    backdropColor,
  };
}

/**
 * D-IE-10: entries in file order, each one mirroring the conversion save —
 * `savePatternDoc` awaited, then `addProject`, then the cursor if there is one.
 * A write that rejects stops the run: entries already landed stay (an orphan
 * document is the harmless half of R-UP-2, a project pointing at nothing is
 * not), and the summary says how far it got.
 *
 * Inventory merges after every entry, because it belongs to the archive rather
 * than to any one project.
 */
export async function importBeadproj(
  document: BeadprojDocument,
  deps: BeadprojImportDeps,
): Promise<BeadprojImportResult> {
  const mint = deps.mintId ?? mintProjectId;
  const projectIds: ProjectId[] = [];
  let failedIndex: number | null = null;

  for (const accepted of document.entries) {
    const id = mint();
    const doc = toPatternDocFor(id, accepted.entry);
    if (doc !== null) {
      try {
        await deps.savePatternDoc(doc);
      } catch {
        failedIndex = accepted.index;
        break;
      }
    }
    deps.addProject(toProject(id, accepted.entry));
    const progress = accepted.entry.progress;
    if (progress !== undefined) {
      // `updatedAt` is the action's to stamp (BD19), so it is the one field an
      // import cannot restore verbatim.
      deps.upsertProgress({
        projectId: id,
        mode: progress.mode,
        stepIndex: progress.stepIndex,
        elapsedMs: progress.elapsedMs,
      });
    }
    projectIds.push(id);
  }

  const seen = new Set(
    deps.inventory.map((entry) =>
      paletteCodeKey({ paletteId: entry.paletteId, code: normalizeCode(entry.code) }),
    ),
  );
  let inventoryAdded = 0;
  let inventorySkipped = 0;
  for (const row of document.inventory) {
    const key = paletteCodeKey({ paletteId: row.paletteId, code: normalizeCode(row.code) });
    if (seen.has(key)) {
      inventorySkipped += 1;
      continue;
    }
    seen.add(key);
    deps.addInventoryEntry({ ...row });
    inventoryAdded += 1;
  }

  return {
    total: document.total,
    imported: projectIds.length,
    failedIndex,
    entryErrors: document.entryErrors,
    inventoryAdded,
    inventorySkipped,
    inventoryDropped: document.droppedInventory,
    projectIds,
  };
}

/** The one-line account the panel puts in its `role="status"` region. */
export function describeImportResult(result: BeadprojImportResult): string {
  const parts = [`已导入 ${result.imported} / ${result.total} 个项目`];
  if (result.failedIndex !== null) {
    parts.push(`第 ${result.failedIndex} 条写入失败，其后的条目没有继续导入`);
  }
  if (result.inventoryAdded + result.inventorySkipped + result.inventoryDropped > 0) {
    parts.push(
      `库存新增 ${result.inventoryAdded} / 跳过 ${result.inventorySkipped} / 丢弃 ${result.inventoryDropped}`,
    );
  }
  return `${parts.join("；")}。`;
}
