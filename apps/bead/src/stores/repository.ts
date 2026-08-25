import {
  EMPTY_STATE,
  type BackdropKind,
  type InventoryEntry,
  type PersistedState,
  type Project,
} from "./types.ts";
import { isPatternId, isProjectId, type PatternId } from "./ids.ts";

// D-UI-5: every signature here is async even though B02 stores to
// localStorage. B09 swaps in IndexedDB behind the same interface, so no page
// or store has to change shape when that lands.

export interface ProjectRepository {
  loadProjects(): Promise<Project[]>;
  saveProjects(projects: Project[]): Promise<void>;
  loadFavorites(): Promise<PatternId[]>;
  saveFavorites(favorites: PatternId[]): Promise<void>;
}

export interface InventoryRepository {
  loadInventory(): Promise<InventoryEntry[]>;
  saveInventory(entries: InventoryEntry[]): Promise<void>;
}

export type Repository = ProjectRepository & InventoryRepository;

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
  return { projects, favorites, inventory };
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

  const read = (): PersistedState => {
    try {
      return parsePersistedState(backing ? backing.getItem(STORAGE_KEY) : fallback.read());
    } catch {
      return parsePersistedState(fallback.read());
    }
  };

  const write = (next: PersistedState): void => {
    const serialized = JSON.stringify(next);
    try {
      if (backing) backing.setItem(STORAGE_KEY, serialized);
      else fallback.write(serialized);
    } catch {
      fallback.write(serialized);
    }
  };

  return {
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
  };
}

/** Test double: same async contract, no storage at all. */
export function createInMemoryRepository(seed: Partial<PersistedState> = {}): Repository {
  let state: PersistedState = { ...EMPTY_STATE, ...seed };
  return {
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
  };
}
