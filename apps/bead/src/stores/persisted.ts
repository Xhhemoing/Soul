/**
 * Shape checks for everything that crosses the storage boundary.
 *
 * These sit apart from the repository engines because both of them need the
 * same answers: the localStorage backing parses a JSON blob, and the `bead-v1`
 * IndexedDB backing re-checks the structured clones it reads back. Storage is
 * hand-editable in devtools either way, so anything that fails a check is
 * dropped rather than allowed to crash a page (SH-4 precedent).
 */

import { SPLIT_MODES, type SplitMode } from "../algo/steps.ts";
import { isPatternId, isProjectId, type PatternId, type ProjectId } from "./ids.ts";
import {
  EMPTY_STATE,
  GALLERY_PALETTE,
  isPaletteNamespaceId,
  type BackdropKind,
  type InventoryEntry,
  type PersistedState,
  type ProgressCursor,
  type Project,
} from "./types.ts";

/** The pre-`bead-v1` localStorage key. Still read once, by the migration. */
export const STORAGE_KEY = "bead.state";

const BACKDROP_KINDS: readonly BackdropKind[] = ["black", "white", "custom"];

// localStorage is a boundary the user can hand-edit, so `backdrop` is checked
// like the rest: a record without it survives the filter and then makes
// `readableTextColor(undefined)` throw the first time /assemble renders it.
export function isProject(value: unknown): value is Project {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate["id"] === "string" &&
    isProjectId(candidate["id"]) &&
    typeof candidate["title"] === "string" &&
    typeof candidate["status"] === "string" &&
    BACKDROP_KINDS.includes(candidate["backdrop"] as BackdropKind) &&
    typeof candidate["backdropColor"] === "string"
  );
}

// Every field is checked, not just the identity key: a stored entry missing
// `hex` renders a swatch with `background: undefined`, and a fractional or NaN
// `beads` poisons every shortage sum downstream of it.
export function isInventoryEntry(value: unknown): value is InventoryEntry {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  const beads = candidate["beads"];
  return (
    isPaletteNamespaceId(candidate["paletteId"]) &&
    typeof candidate["code"] === "string" &&
    typeof candidate["name"] === "string" &&
    typeof candidate["hex"] === "string" &&
    Number.isInteger(beads) &&
    (beads as number) >= 0
  );
}

/**
 * BD20 arrives after v0 stock was already typed in, and every code that got
 * typed in was a gallery code (D-INV-3) — the UI never offered another palette.
 * So an entry with no namespace at all is stamped `gallery` on the way through,
 * which is also what makes the `bead-v1` migration a plain re-put. An entry
 * naming a namespace this build does not know is a different matter and is
 * dropped, because guessing which palette a code belongs to is how G07 becomes
 * ambiguous again.
 */
export function toInventoryEntry(value: unknown): InventoryEntry | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Record<string, unknown>;
  const paletteId = candidate["paletteId"] ?? GALLERY_PALETTE;
  const stamped = { ...candidate, paletteId };
  if (!isInventoryEntry(stamped)) return null;
  const { code, name, hex, beads } = stamped;
  return { paletteId: stamped.paletteId, code, name, hex, beads };
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

/**
 * Rebuilds the cursor from scratch instead of passing the parsed object
 * through: whatever extra keys a hand-edited blob (or an older build that got
 * ahead of BD19) carried are dropped here rather than round-tripped back to
 * disk.
 */
export function toProgressCursor(value: unknown): ProgressCursor | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Record<string, unknown>;
  const projectId = candidate["projectId"];
  const mode = candidate["mode"];
  if (typeof projectId !== "string" || !isProjectId(projectId)) return null;
  if (typeof mode !== "string" || !SPLIT_MODES.includes(mode as SplitMode)) return null;
  const { stepIndex, elapsedMs, updatedAt } = candidate;
  if (!isCount(stepIndex) || !isCount(elapsedMs)) return null;
  if (typeof updatedAt !== "number" || !Number.isFinite(updatedAt)) return null;
  return { projectId, mode: mode as SplitMode, stepIndex, elapsedMs, updatedAt };
}

/**
 * Two convergences on top of the shape check, both there to keep the array
 * bounded: one cursor per project (the freshest `updatedAt` wins, DATA-5's
 * last-writer-wins carried down to the entry level), and cursors whose project
 * is gone are pruned instead of accumulating forever.
 */
export function sanitizeProgress(
  values: readonly unknown[],
  projects: readonly Project[],
): ProgressCursor[] {
  const known = new Set<string>(projects.map((project) => project.id));
  const latest = new Map<ProjectId, ProgressCursor>();
  for (const value of values) {
    const cursor = toProgressCursor(value);
    if (cursor === null || !known.has(cursor.projectId)) continue;
    const previous = latest.get(cursor.projectId);
    if (previous === undefined || cursor.updatedAt >= previous.updatedAt) {
      latest.set(cursor.projectId, cursor);
    }
  }
  return [...latest.values()];
}

export function parseProjects(value: unknown): Project[] {
  return Array.isArray(value) ? value.filter(isProject) : [];
}

export function parseFavorites(value: unknown): PatternId[] {
  return Array.isArray(value)
    ? value.filter((id): id is PatternId => typeof id === "string" && isPatternId(id))
    : [];
}

export function parseInventory(value: unknown): InventoryEntry[] {
  if (!Array.isArray(value)) return [];
  const entries: InventoryEntry[] = [];
  for (const candidate of value) {
    const entry = toInventoryEntry(candidate);
    if (entry !== null) entries.push(entry);
  }
  return entries;
}

/** Anything that fails the shape check is dropped rather than crashing the app. */
export function parsePersistedState(raw: string | null): PersistedState {
  if (raw === null) return EMPTY_STATE;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return EMPTY_STATE;
  }
  if (typeof parsed !== "object" || parsed === null) return EMPTY_STATE;
  const record = parsed as Record<string, unknown>;
  const projects = parseProjects(record["projects"]);
  const favorites = parseFavorites(record["favorites"]);
  const inventory = parseInventory(record["inventory"]);
  const progress = Array.isArray(record["progress"])
    ? sanitizeProgress(record["progress"], projects)
    : [];
  return { projects, favorites, inventory, progress };
}
