/**
 * `.beadproj` v1 — the archive format WP-B07 reads and writes (D-IE-3/4).
 *
 * The TypeScript below *is* the schema. There is no JSON Schema file and no
 * validation library: a hand-written rebuilding checker is what fails closed
 * (D-IE-4), and a schema document is exactly where an `$id` / `$schema` URL
 * would have to go, which BD15/BD17 forbid. `beadproj.md` next door carries the
 * prose; this file carries the only executable answer.
 *
 * One shape covers both directions and both scopes: a single-project export is
 * an archive of length one, so there is one validator and one import path.
 * Nothing here touches the DOM or storage — the panels own that.
 */

import type { ImageKind } from "../algo/classify.ts";
import { GENERIC_5MM } from "../algo/palette.ts";
import type { SplitMode } from "../algo/steps.ts";
import { isPatternId } from "../stores/ids.ts";
import { EMPTY_CELL, decodeCells } from "../stores/patterns.ts";
import { isInventoryEntry, toProgressCursor } from "../stores/repository.ts";
import {
  GENERIC_5MM_PALETTE,
  type BackdropKind,
  type InventoryEntry,
  type PaletteNamespaceId,
  type PatternDoc,
  type ProgressCursor,
  type Project,
  type ProjectStatus,
} from "../stores/types.ts";

export const BEADPROJ_FORMAT = "beadproj";
export const BEADPROJ_VERSION = 1;

/** D-IE-16: the archive名 is fixed; a single project uses its own title. */
export const BEADPROJ_ARCHIVE_FILE_NAME = "bead-projects.beadproj";
export const BEADPROJ_FALLBACK_FILE_STEM = "bead-project";
export const BEADPROJ_EXTENSION = ".beadproj";
export const BEADPROJ_MIME = "application/json";

/** D-IE-6 ①: refused before a single byte is read. */
export const MAX_IMPORT_FILE_BYTES = 20 * 1024 * 1024;
/** D-IE-6 ②: what the format itself allows per axis. */
export const MAX_CONTRACT_SIDE = 512;
/** D-IE-6 ③: what v0 has a consumer for. Format-legal above this, still refused. */
export const MAX_V0_SIDE = 56;
/** round2-data §6.3 quota gate. Longer titles are refused, never truncated. */
export const MAX_IMPORT_TITLE_LENGTH = 256;
export const UNTITLED_IMPORT = "未命名导入";

const STATUSES: readonly ProjectStatus[] = ["todo", "active", "draft", "done"];
const BACKDROPS: readonly BackdropKind[] = ["black", "white", "custom"];
const PALETTE_SIZE = GENERIC_5MM.entries.length;

/**
 * D-IE-19: a closed set. Every parse failure lands on one of these, and each
 * one is rendered inline by the panel that hit it — a caught error that lets
 * the flow continue as if nothing happened is still a defect.
 */
export type ImportErrorCode =
  | "FILE_TOO_LARGE"
  | "SCHEMA_INVALID"
  | "UNSUPPORTED_VERSION"
  | "GRID_TOO_LARGE_FOR_V0"
  | "UNSUPPORTED_FORMAT";

export interface ImportFailure {
  readonly code: ImportErrorCode;
  readonly message: string;
}

/** An entry the file got wrong. Carries its position so the report can point. */
export interface BeadprojEntryError {
  /** 1-based position in the file's `projects` array. */
  readonly index: number;
  readonly code: ImportErrorCode;
  readonly message: string;
}

export interface BeadprojProject {
  title: string;
  /** Null ⇔ this entry carries a `pattern` (D-IE-9). */
  sourcePatternId: string | null;
  status: ProjectStatus;
  createdAt: number;
  backdrop: BackdropKind;
  backdropColor: string;
}

/** v0's only indexable namespace (round2-data §6.4); `gallery` has no cell indices. */
export type BeadprojPaletteId = Extract<PaletteNamespaceId, "generic-5mm">;

export interface BeadprojProvenance {
  kind: ImageKind;
  ditherApplied: boolean;
}

export interface BeadprojPattern {
  paletteId: BeadprojPaletteId;
  width: number;
  height: number;
  /** Row-major, `-1` for empty, otherwise an index into `generic-5mm`. */
  cells: number[];
  provenance?: BeadprojProvenance;
}

/** BD19's cursor minus `projectId`: nesting is the association (D-IE-8). */
export interface BeadprojProgress {
  mode: SplitMode;
  stepIndex: number;
  elapsedMs: number;
  updatedAt: number;
}

export interface BeadprojEntry {
  project: BeadprojProject;
  pattern?: BeadprojPattern;
  progress?: BeadprojProgress;
}

/** D-IE-12: `paletteId` is mandatory here. An entry without one is dropped, never stamped. */
export interface BeadprojInventoryEntry {
  paletteId: InventoryEntry["paletteId"];
  code: string;
  name: string;
  hex: string;
  beads: number;
}

export interface BeadprojFile {
  format: typeof BEADPROJ_FORMAT;
  version: typeof BEADPROJ_VERSION;
  projects: BeadprojEntry[];
  inventory?: BeadprojInventoryEntry[];
}

export interface BeadprojAcceptedEntry {
  /** 1-based position in the file, kept so a later write failure can be located. */
  readonly index: number;
  readonly entry: BeadprojEntry;
}

/** What survived the file-level check: the good entries plus a full account of the rest. */
export interface BeadprojDocument {
  /** How many entries the file claimed, including the rejected ones. */
  readonly total: number;
  readonly entries: readonly BeadprojAcceptedEntry[];
  readonly entryErrors: readonly BeadprojEntryError[];
  readonly inventory: readonly BeadprojInventoryEntry[];
  /** Inventory rows with a missing or unknown `paletteId` (D-IE-12). */
  readonly droppedInventory: number;
}

export type BeadprojParseResult =
  | { readonly ok: true; readonly document: BeadprojDocument }
  | { readonly ok: false; readonly failure: ImportFailure };

function fail(code: ImportErrorCode, message: string): BeadprojParseResult {
  return { ok: false, failure: { code, message } };
}

/**
 * D-IE-6 ①: the size gate runs on `File.size`, before `text()` allocates
 * anything. Returns the failure rather than throwing so the caller renders it
 * inline like every other named error.
 */
export function checkImportFileSize(size: number): ImportFailure | null {
  if (size <= MAX_IMPORT_FILE_BYTES) return null;
  const mb = (size / (1024 * 1024)).toFixed(1);
  return {
    code: "FILE_TOO_LARGE",
    message: `文件太大：上限 ${MAX_IMPORT_FILE_BYTES / (1024 * 1024)}MB，这个文件约 ${mb}MB`,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isPositiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0;
}

type EntryCheck =
  | { readonly ok: true; readonly entry: BeadprojEntry }
  | { readonly ok: false; readonly code: ImportErrorCode; readonly message: string };

function rejectEntry(message: string, code: ImportErrorCode = "SCHEMA_INVALID"): EntryCheck {
  return { ok: false, code, message };
}

function toBeadprojProject(value: unknown): BeadprojProject | string {
  if (!isRecord(value)) return "project 不是对象";

  const title = value["title"];
  if (typeof title !== "string") return "title 不是字符串";
  // The quota gate refuses rather than truncates: a silently shortened title is
  // a different project than the one the archive described.
  if (title.length > MAX_IMPORT_TITLE_LENGTH) {
    return `title 超过 ${MAX_IMPORT_TITLE_LENGTH} 字`;
  }

  const status = value["status"];
  if (typeof status !== "string" || !STATUSES.includes(status as ProjectStatus)) {
    return "status 不在 todo / active / draft / done 之内";
  }

  const createdAt = value["createdAt"];
  if (typeof createdAt !== "number" || !Number.isFinite(createdAt) || createdAt <= 0) {
    return "createdAt 不是有限正数";
  }

  const backdrop = value["backdrop"];
  if (typeof backdrop !== "string" || !BACKDROPS.includes(backdrop as BackdropKind)) {
    return "backdrop 不在 black / white / custom 之内";
  }
  const backdropColor = value["backdropColor"];
  if (typeof backdropColor !== "string") return "backdropColor 不是字符串";

  const sourcePatternId = value["sourcePatternId"];
  if (sourcePatternId !== null && (typeof sourcePatternId !== "string" || !isPatternId(sourcePatternId))) {
    return "sourcePatternId 既不是 null 也不是画廊图纸 id";
  }

  return {
    title: title.trim() === "" ? UNTITLED_IMPORT : title,
    sourcePatternId,
    status: status as ProjectStatus,
    createdAt,
    backdrop: backdrop as BackdropKind,
    backdropColor,
  };
}

/** D-IE-6: three gates in order, so 513 is a format error and 57 is a v0 refusal. */
function checkSide(width: number, height: number): ImportFailure | null {
  if (width > MAX_CONTRACT_SIDE || height > MAX_CONTRACT_SIDE) {
    return {
      code: "SCHEMA_INVALID",
      message: `pattern 尺寸 ${width}×${height} 超过格式上限 ${MAX_CONTRACT_SIDE}`,
    };
  }
  if (width > MAX_V0_SIDE || height > MAX_V0_SIDE) {
    return {
      code: "GRID_TOO_LARGE_FOR_V0",
      message: `这份豆图是 ${width}×${height}，v0 支持上限 ${MAX_V0_SIDE}×${MAX_V0_SIDE}`,
    };
  }
  return null;
}

function toProvenance(value: unknown): BeadprojProvenance | null {
  if (!isRecord(value)) return null;
  const kind = value["kind"];
  if (kind !== "PixelArt" && kind !== "Photo") return null;
  const ditherApplied = value["ditherApplied"];
  if (typeof ditherApplied !== "boolean") return null;
  return { kind, ditherApplied };
}

function toBeadprojPattern(value: unknown): BeadprojPattern | ImportFailure {
  if (!isRecord(value)) return { code: "SCHEMA_INVALID", message: "pattern 不是对象" };
  if (value["paletteId"] !== GENERIC_5MM_PALETTE) {
    return { code: "SCHEMA_INVALID", message: `pattern.paletteId 只能是 ${GENERIC_5MM_PALETTE}` };
  }

  const { width, height } = value;
  if (!isPositiveInteger(width) || !isPositiveInteger(height)) {
    return { code: "SCHEMA_INVALID", message: "pattern 尺寸不是正整数" };
  }
  const sideFailure = checkSide(width, height);
  if (sideFailure !== null) return sideFailure;

  const cells = value["cells"];
  if (!Array.isArray(cells)) return { code: "SCHEMA_INVALID", message: "cells 不是数组" };
  if (cells.length !== width * height) {
    return {
      code: "SCHEMA_INVALID",
      message: `cells 有 ${cells.length} 个，与 ${width}×${height} 不符`,
    };
  }
  for (const cell of cells) {
    if (typeof cell !== "number" || !Number.isInteger(cell)) {
      return { code: "SCHEMA_INVALID", message: "cells 里有非整数" };
    }
    if (cell !== EMPTY_CELL && (cell < 0 || cell >= PALETTE_SIZE)) {
      return { code: "SCHEMA_INVALID", message: `cells 里的 ${cell} 不是 -1 或 0…${PALETTE_SIZE - 1}` };
    }
  }

  const provenance = toProvenance(value["provenance"]);
  return {
    paletteId: "generic-5mm",
    width,
    height,
    cells: cells.map((cell) => cell as number),
    ...(provenance === null ? {} : { provenance }),
  };
}

/**
 * D-IE-13: the cursor rules are `toProgressCursor`'s, not a second copy of
 * them. The file carries no project id (D-IE-8), so a placeholder stands in
 * for the check and the real one is stamped at import time.
 */
const CURSOR_CHECK_ID = "proj-beadproj-check";

function toBeadprojProgress(value: unknown): BeadprojProgress | null {
  if (!isRecord(value)) return null;
  const checked = toProgressCursor({ ...value, projectId: CURSOR_CHECK_ID });
  if (checked === null) return null;
  return {
    mode: checked.mode,
    stepIndex: checked.stepIndex,
    elapsedMs: checked.elapsedMs,
    updatedAt: checked.updatedAt,
  };
}

function toBeadprojEntry(value: unknown): EntryCheck {
  if (!isRecord(value)) return rejectEntry("entry 不是对象");

  const project = toBeadprojProject(value["project"]);
  if (typeof project === "string") return rejectEntry(project);

  const rawPattern = value["pattern"];
  const hasPattern = rawPattern !== undefined && rawPattern !== null;

  // D-IE-9's bijection, checked before the pattern itself: a gallery project
  // that also ships a grid, and a null-source project with none, are both
  // records no minting path in this app can produce.
  if (project.sourcePatternId === null && !hasPattern) {
    return rejectEntry("sourcePatternId 为 null 却没有 pattern");
  }
  if (project.sourcePatternId !== null && hasPattern) {
    return rejectEntry("画廊来源的 entry 不该带 pattern");
  }

  let pattern: BeadprojPattern | undefined;
  if (hasPattern) {
    const checked = toBeadprojPattern(rawPattern);
    if ("code" in checked) return rejectEntry(checked.message, checked.code);
    pattern = checked;
  }

  const rawProgress = value["progress"];
  let progress: BeadprojProgress | undefined;
  if (rawProgress !== undefined && rawProgress !== null) {
    const checked = toBeadprojProgress(rawProgress);
    if (checked === null) return rejectEntry("progress 游标不合法");
    progress = checked;
  }

  return {
    ok: true,
    entry: {
      project,
      ...(pattern === undefined ? {} : { pattern }),
      ...(progress === undefined ? {} : { progress }),
    },
  };
}

function toBeadprojInventoryEntry(value: unknown): BeadprojInventoryEntry | null {
  // `isInventoryEntry` is the same check storage uses, and it already demands a
  // known namespace. `toInventoryEntry` is deliberately *not* used: it stamps a
  // missing namespace `gallery`, which is exactly the BD20 collision this
  // format refuses to re-create.
  if (!isInventoryEntry(value)) return null;
  const { paletteId, code, name, hex, beads } = value;
  return { paletteId, code, name, hex, beads };
}

/**
 * The whole file-level gate. `value` is whatever `JSON.parse` produced; the
 * result is rebuilt key by key, so anything else the file carried is gone
 * rather than written back on the next export.
 */
export function parseBeadproj(value: unknown): BeadprojParseResult {
  if (!isRecord(value)) return fail("SCHEMA_INVALID", "文件内容不是一个 .beadproj 对象");
  if (value["format"] !== BEADPROJ_FORMAT) {
    return fail("SCHEMA_INVALID", `format 不是 ${BEADPROJ_FORMAT}`);
  }
  // Fail closed: a future version is not guessed at, half-read or downgraded.
  if (value["version"] !== BEADPROJ_VERSION) {
    return fail(
      "UNSUPPORTED_VERSION",
      `这个文件是 version ${String(value["version"])}，本版本只读 version ${BEADPROJ_VERSION}`,
    );
  }

  const rawProjects = value["projects"];
  if (!Array.isArray(rawProjects)) return fail("SCHEMA_INVALID", "projects 不是数组");

  const rawInventory = value["inventory"];
  if (rawInventory !== undefined && !Array.isArray(rawInventory)) {
    return fail("SCHEMA_INVALID", "inventory 不是数组");
  }

  const entries: BeadprojAcceptedEntry[] = [];
  const entryErrors: BeadprojEntryError[] = [];
  rawProjects.forEach((raw, position) => {
    const index = position + 1;
    const checked = toBeadprojEntry(raw);
    if (checked.ok) entries.push({ index, entry: checked.entry });
    else entryErrors.push({ index, code: checked.code, message: checked.message });
  });

  const inventory: BeadprojInventoryEntry[] = [];
  let droppedInventory = 0;
  for (const raw of rawInventory ?? []) {
    const entry = toBeadprojInventoryEntry(raw);
    if (entry === null) droppedInventory += 1;
    else inventory.push(entry);
  }

  return {
    ok: true,
    document: { total: rawProjects.length, entries, entryErrors, inventory, droppedInventory },
  };
}

/** Text in, the same gate out. Bad JSON is `SCHEMA_INVALID`, not a thrown error. */
export function parseBeadprojText(text: string): BeadprojParseResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return fail("SCHEMA_INVALID", "这个文件不是合法的 JSON");
  }
  return parseBeadproj(parsed);
}

/**
 * The writer side of the same schema (D-IE-14). The bijection is enforced here
 * too: a gallery-sourced project exports as metadata only, whatever documents
 * happen to be lying around under its id.
 */
export function buildBeadprojEntry(
  project: Project,
  doc: PatternDoc | null,
  cursor: ProgressCursor | null,
): BeadprojEntry {
  const gallery = project.sourcePatternId !== null;
  const grid = gallery || doc === null ? null : decodeCells(doc);
  return {
    project: {
      title: project.title,
      sourcePatternId: project.sourcePatternId,
      status: project.status,
      createdAt: project.createdAt,
      backdrop: project.backdrop,
      backdropColor: project.backdropColor,
    },
    ...(grid === null || doc === null
      ? {}
      : {
          pattern: {
            paletteId: "generic-5mm",
            width: doc.width,
            height: doc.height,
            cells: grid.cells.map((cell) => cell ?? EMPTY_CELL),
            ...(doc.provenance === undefined ? {} : { provenance: { ...doc.provenance } }),
          },
        }),
    ...(cursor === null
      ? {}
      : {
          progress: {
            mode: cursor.mode,
            stepIndex: cursor.stepIndex,
            elapsedMs: cursor.elapsedMs,
            updatedAt: cursor.updatedAt,
          },
        }),
  };
}

export function buildBeadprojFile(
  entries: readonly BeadprojEntry[],
  inventory: readonly InventoryEntry[] = [],
): BeadprojFile {
  return {
    format: BEADPROJ_FORMAT,
    version: BEADPROJ_VERSION,
    projects: [...entries],
    ...(inventory.length === 0
      ? {}
      : {
          inventory: inventory.map(({ paletteId, code, name, hex, beads }) => ({
            paletteId,
            code,
            name,
            hex,
            beads,
          })),
        }),
  };
}

/**
 * D-IE-3: no timestamp anywhere in the file, so the bytes are a pure function
 * of the state and a test can compare two exports directly.
 */
export function serializeBeadproj(file: BeadprojFile): string {
  return `${JSON.stringify(file)}\n`;
}

const UNSAFE_FILE_NAME = /[^\p{Letter}\p{Number}_-]+/gu;
const MAX_FILE_STEM_LENGTH = 64;

/** D-IE-16: a determined file name, with the untitled case spelled out. */
export function beadprojFileName(title: string): string {
  const stem = title
    .trim()
    .replace(UNSAFE_FILE_NAME, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, MAX_FILE_STEM_LENGTH);
  return `${stem === "" ? BEADPROJ_FALLBACK_FILE_STEM : stem}${BEADPROJ_EXTENSION}`;
}
