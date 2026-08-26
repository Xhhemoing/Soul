import { createBeadV1Repository } from "./bead-v1.ts";
import {
  STORAGE_KEY,
  parsePersistedState,
  sanitizeProgress,
} from "./persisted.ts";
import { toPatternDoc } from "./patterns.ts";
import type { PatternId, ProjectId } from "./ids.ts";
import {
  EMPTY_STATE,
  type InventoryEntry,
  type PatternDoc,
  type PersistedState,
  type ProgressCursor,
  type Project,
} from "./types.ts";

// D-UI-5: every signature here is async even though B02 stored to
// localStorage. `bead-v1` swapped in behind the same interface, and the six
// original methods did not change shape when it did.

export interface ProjectRepository {
  loadProjects(): Promise<Project[]>;
  saveProjects(projects: Project[]): Promise<void>;
  loadFavorites(): Promise<PatternId[]>;
  saveFavorites(favorites: PatternId[]): Promise<void>;
  loadProgress(): Promise<ProgressCursor[]>;
  saveProgress(cursors: ProgressCursor[]): Promise<void>;
}

/**
 * round3 IA §2.2: the three-method increment `bead-v1` adds. Per-project
 * progress accessors are deliberately absent — the array signatures above
 * already have every caller they need, and an interface nobody calls is an
 * interface nobody keeps correct.
 */
export interface PatternRepository {
  loadPatternDoc(id: ProjectId): Promise<PatternDoc | null>;
  /** Rejects on a failed write; D-UP-10 mints no project when it does. */
  savePatternDoc(doc: PatternDoc): Promise<void>;
  /** One transaction, `patterns` + `progress`. The `projects` array is the reducer's. */
  deleteProject(id: ProjectId): Promise<void>;
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

export type Repository = ProjectRepository &
  PatternRepository &
  InventoryRepository &
  PersistenceStatus;

export { STORAGE_KEY, parsePersistedState, sanitizeProgress };
export { isInventoryEntry, toInventoryEntry, toProgressCursor } from "./persisted.ts";

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
 * The pre-`bead-v1` backing, kept for the embeddings that have localStorage but
 * no IndexedDB (and for the tests that drive a failing `Storage`).
 *
 * It carries metadata only. BD19's red line is that a `Grid` never goes into
 * `bead.state`, so `savePatternDoc` here does not serialise one somewhere else
 * either — it reports that it cannot store the document, which is the truth and
 * which stops D-UP-10 from minting a project pointing at nothing.
 */
function createLocalStateRepository(storage: Storage | undefined): Repository {
  const fallback = new MemoryBacking();
  const backing = storage;

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

  const patterns = new Map<string, PatternDoc>();

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
    async loadPatternDoc(id) {
      return patterns.get(id) ?? null;
    },
    async savePatternDoc(doc) {
      const checked = toPatternDoc(doc);
      if (checked === null) throw new Error("豆图文档形状不合法，拒绝写入");
      patterns.set(checked.projectId, checked);
      // Rejecting is the whole answer here. `persistenceFailed` is not raised:
      // the metadata backing is working, and flipping it would move projects,
      // favourites and inventory onto the empty in-memory copy — losing the
      // user's real data to report a failure in a different store.
      throw new Error("这个环境没有 IndexedDB，豆图无法保存");
    },
    async deleteProject(id) {
      patterns.delete(id);
    },
  };
}

function defaultStorage(): Storage | undefined {
  try {
    return globalThis.localStorage ?? undefined;
  } catch {
    return undefined;
  }
}

function defaultIndexedDb(): IDBFactory | undefined {
  try {
    return globalThis.indexedDB ?? undefined;
  } catch {
    return undefined;
  }
}

/**
 * BD19: `bead-v1` when the platform has IndexedDB, which is every browser this
 * app targets. `storage` stays in the signature because it is still the
 * migration source — the one-way import of the old `bead.state` blob — and
 * because it remains the whole backing in the degraded case.
 */
export function createRepository(
  storage: Storage | undefined = defaultStorage(),
  factory: IDBFactory | null | undefined = defaultIndexedDb(),
): Repository {
  if (factory === undefined || factory === null) return createLocalStateRepository(storage);
  return createBeadV1Repository(factory, { legacyStorage: storage });
}

/** Test double: same async contract, no storage at all. */
export function createInMemoryRepository(
  seed: Partial<PersistedState> = {},
  seedPatterns: readonly PatternDoc[] = [],
): Repository {
  let state: PersistedState = { ...EMPTY_STATE, ...seed };
  const patterns = new Map<string, PatternDoc>(
    seedPatterns.map((doc) => [doc.projectId, doc] as const),
  );
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
    async loadPatternDoc(id) {
      return patterns.get(id) ?? null;
    },
    async savePatternDoc(doc) {
      patterns.set(doc.projectId, doc);
    },
    async deleteProject(id) {
      patterns.delete(id);
      state = {
        ...state,
        progress: state.progress.filter((cursor) => cursor.projectId !== id),
      };
    },
  };
}
