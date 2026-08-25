import {
  EMPTY_STATE,
  type BackdropKind,
  type InventoryEntry,
  type PersistedState,
  type ProgressCursor,
  type Project,
} from "./types.ts";
import { isPatternId, isProjectId, type PatternId, type ProjectId } from "./ids.ts";
import { SPLIT_MODES, type SplitMode } from "../algo/steps.ts";

// D-UI-5: every signature here is async even though B02 stores to
// localStorage. B09 swaps in IndexedDB behind the same interface, so no page
// or store has to change shape when that lands.

export interface ProjectRepository {
  loadProjects(): Promise<Project[]>;
  saveProjects(projects: Project[]): Promise<void>;
  loadFavorites(): Promise<PatternId[]>;
  saveFavorites(favorites: PatternId[]): Promise<void>;
  loadProgress(): Promise<ProgressCursor[]>;
  saveProgress(cursors: ProgressCursor[]): Promise<void>;
}

export interface InventoryRepository {
  loadInventory(): Promise<InventoryEntry[]>;
  saveInventory(entries: InventoryEntry[]): Promise<void>;
}

/**
 * DATA-1: storage that stops accepting writes must become an observable state,
 * never a quiet in-memory detour. Once this reports true nothing reaches disk
 * for the rest of the session, so the UI keeps a standing notice up rather than
 * a one-shot toast.
 */
export interface PersistenceStatus {
  isPersistenceFailed(): boolean;
  /** `useSyncExternalStore`-shaped: returns the unsubscribe. */
  subscribeToPersistence(listener: () => void): () => void;
}

export type Repository = ProjectRepository & InventoryRepository & PersistenceStatus;

export const STORAGE_KEY = "bead.state";

const BACKDROP_KINDS: readonly BackdropKind[] = ["black", "white", "custom"];

// localStorage is a boundary the user can hand-edit, so `backdrop` is checked
// like the rest: a record without it survives the filter and then makes
// `readableTextColor(undefined)` throw the first time /assemble renders it.
function isProject(value: unknown): value is Project {
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

function isInventoryEntry(value: unknown): value is InventoryEntry {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate["code"] === "string" && typeof candidate["beads"] === "number";
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
  const projects = Array.isArray(record["projects"]) ? record["projects"].filter(isProject) : [];
  const favorites = Array.isArray(record["favorites"])
    ? record["favorites"].filter((id): id is PatternId => typeof id === "string" && isPatternId(id))
    : [];
  const inventory = Array.isArray(record["inventory"])
    ? record["inventory"].filter(isInventoryEntry)
    : [];
  const progress = Array.isArray(record["progress"])
    ? sanitizeProgress(record["progress"], projects)
    : [];
  return { projects, favorites, inventory, progress };
}

class MemoryBacking {
  private value: string | null = null;

  read(): string | null {
    return this.value;
  }

  write(next: string): void {
    this.value = next;
  }
}

/**
 * localStorage is absent in some embeddings and throws in private modes, so the
 * repository degrades to an in-memory backing instead of taking the app down.
 *
 * The degradation is one-way and total (DATA-1): the moment a write cannot
 * reach storage, reads move to the same in-memory copy the writes land in and
 * `isPersistenceFailed()` flips. Serving the pre-failure localStorage copy to a
 * later read would make the next read-modify-write rebase onto data that is
 * already stale, which loses everything written after the first failure with no
 * signal at all.
 */
export function createRepository(storage?: Storage): Repository {
  const fallback = new MemoryBacking();
  const backing =
    storage ??
    (() => {
      try {
        return globalThis.localStorage ?? undefined;
      } catch {
        return undefined;
      }
    })();

  // No backing at all is the same condition arrived at early: this session
  // persists nothing, so say so instead of pretending the memory copy survives.
  let persistenceFailed = backing === undefined;
  const listeners = new Set<() => void>();

  const markFailed = (): void => {
    if (persistenceFailed) return;
    persistenceFailed = true;
    for (const listener of [...listeners]) listener();
  };

  const read = (): PersistedState => {
    if (persistenceFailed || !backing) return parsePersistedState(fallback.read());
    try {
      return parsePersistedState(backing.getItem(STORAGE_KEY));
    } catch {
      // Storage that cannot be read cannot be written either; the writes that
      // follow would be lost silently, so fail the whole backing now.
      markFailed();
      return parsePersistedState(fallback.read());
    }
  };

  const write = (next: PersistedState): void => {
    const serialized = JSON.stringify(next);
    if (persistenceFailed || !backing) {
      fallback.write(serialized);
      return;
    }
    try {
      backing.setItem(STORAGE_KEY, serialized);
    } catch {
      // `next` was merged onto a successful read, so the fallback starts out
      // complete; from here reads follow it.
      fallback.write(serialized);
      markFailed();
    }
  };

  return {
    isPersistenceFailed() {
      return persistenceFailed;
    },
    subscribeToPersistence(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    async loadProjects() {
      return read().projects;
    },
    async saveProjects(projects) {
      write({ ...read(), projects });
    },
    async loadFavorites() {
      return read().favorites;
    },
    async saveFavorites(favorites) {
      write({ ...read(), favorites });
    },
    async loadInventory() {
      return read().inventory;
    },
    async saveInventory(inventory) {
      write({ ...read(), inventory });
    },
    async loadProgress() {
      return read().progress;
    },
    async saveProgress(cursors) {
      const current = read();
      write({ ...current, progress: sanitizeProgress(cursors, current.projects) });
    },
  };
}

/** Test double: same async contract, no storage at all. */
export function createInMemoryRepository(seed: Partial<PersistedState> = {}): Repository {
  let state: PersistedState = { ...EMPTY_STATE, ...seed };
  return {
    isPersistenceFailed() {
      return false;
    },
    subscribeToPersistence() {
      return () => {};
    },
    async loadProjects() {
      return state.projects;
    },
    async saveProjects(projects) {
      state = { ...state, projects };
    },
    async loadFavorites() {
      return state.favorites;
    },
    async saveFavorites(favorites) {
      state = { ...state, favorites };
    },
    async loadInventory() {
      return state.inventory;
    },
    async saveInventory(inventory) {
      state = { ...state, inventory };
    },
    async loadProgress() {
      return state.progress;
    },
    async saveProgress(progress) {
      state = { ...state, progress };
    },
  };
}
