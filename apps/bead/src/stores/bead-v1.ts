/**
 * The `bead-v1` IndexedDB backing (round2-data §5, reconciled by round3
 * `/create` IA §2).
 *
 * BD19 pins the trigger: the first work package that persists a `Grid` owes the
 * whole contract, because a 56×56 board is 3136 cells today and B07's imports
 * are budgeted at 512×512 — two of those in a localStorage blob exhaust an
 * origin's quota, and every write there is a full synchronous re-serialisation
 * on the main thread. Four stores, one one-way migration, no wrapper library.
 *
 *   `state`     explicit keys, whole arrays — the same read-modify-write
 *               semantics `bead.state` had, so `store.tsx` and its four save
 *               effects are untouched (D-UI-5 finally paying out).
 *   `patterns`  keyPath `projectId`, one `PatternDoc` each. Kept apart from the
 *               hot metadata so the Workspace list never deserialises a grid.
 *   `progress`  keyPath `projectId`, one cursor each. Same five keys as before.
 *   `meta`      `{ schemaVersion }`; the migration marker and nothing else.
 *
 * DATA-1 carries over unchanged: a backing that stops accepting writes becomes
 * an observable state and the whole read/write view moves to memory with it.
 * There is no catch-then-pretend-it-worked anywhere in this file.
 */

import type { PatternId } from "./ids.ts";
import { toPatternDoc } from "./patterns.ts";
import {
  STORAGE_KEY,
  parseFavorites,
  parseInventory,
  parsePersistedState,
  parseProjects,
  sanitizeProgress,
  toProgressCursor,
} from "./persisted.ts";
import type { Repository } from "./repository.ts";
import {
  EMPTY_STATE,
  type InventoryEntry,
  type PatternDoc,
  type PersistedState,
  type ProgressCursor,
  type Project,
} from "./types.ts";

export const DB_NAME = "bead-v1";
export const DB_VERSION = 1;

export const STORE = {
  state: "state",
  patterns: "patterns",
  progress: "progress",
  meta: "meta",
} as const;

/** The three explicit keys the `state` store holds, one whole array each. */
export const STATE_KEY = {
  projects: "projects",
  favorites: "favorites",
  inventory: "inventory",
} as const;

export const META_SCHEMA_KEY = "schema";

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("IndexedDB 请求失败"));
  });
}

function settled(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error ?? new Error("IndexedDB 事务失败"));
    tx.onabort = () => reject(tx.error ?? new Error("IndexedDB 事务被中止"));
  });
}

function openDatabase(factory: IDBFactory, name: string): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    let req: IDBOpenDBRequest;
    try {
      req = factory.open(name, DB_VERSION);
    } catch (error) {
      reject(error instanceof Error ? error : new Error("无法打开 bead-v1"));
      return;
    }
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE.state)) db.createObjectStore(STORE.state);
      if (!db.objectStoreNames.contains(STORE.patterns)) {
        db.createObjectStore(STORE.patterns, { keyPath: "projectId" });
      }
      if (!db.objectStoreNames.contains(STORE.progress)) {
        db.createObjectStore(STORE.progress, { keyPath: "projectId" });
      }
      if (!db.objectStoreNames.contains(STORE.meta)) db.createObjectStore(STORE.meta);
      req.transaction?.objectStore(STORE.meta).put({ schemaVersion: DB_VERSION }, META_SCHEMA_KEY);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("无法打开 bead-v1"));
    req.onblocked = () => reject(new Error("bead-v1 正被其他标签页升级"));
  });
}

function readLegacy(storage: Storage | undefined): string | null {
  if (storage === undefined) return null;
  try {
    return storage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/**
 * §5.4 plus the round3 §2.1 reconciliation: `bead.state` grew a fourth key
 * (`progress`) after the contract was written, and that one does not belong in
 * the `state` store. The three whole arrays keep their array semantics there;
 * the cursors are split one row per project into `progress`, which is the
 * shape every later read expects.
 *
 * One-way and idempotent by construction: the legacy key is deleted once the
 * transaction commits, and a second open finds nothing to do.
 */
async function migrateLegacyState(db: IDBDatabase, storage: Storage | undefined): Promise<void> {
  const raw = readLegacy(storage);
  if (raw === null) return;

  const state = parsePersistedState(raw);
  const tx = db.transaction([STORE.state, STORE.progress, STORE.meta], "readwrite");
  const stateStore = tx.objectStore(STORE.state);
  stateStore.put(state.projects, STATE_KEY.projects);
  stateStore.put(state.favorites, STATE_KEY.favorites);
  // D-UP-15: `parsePersistedState` has already stamped every pre-BD20 entry
  // `gallery`, so the migration is the moment the namespace becomes real.
  stateStore.put(state.inventory, STATE_KEY.inventory);
  const progressStore = tx.objectStore(STORE.progress);
  for (const cursor of state.progress) progressStore.put(cursor);
  tx.objectStore(STORE.meta).put({ schemaVersion: DB_VERSION }, META_SCHEMA_KEY);
  await settled(tx);

  storage?.removeItem(STORAGE_KEY);
}

interface MemoryView {
  state: PersistedState;
  patterns: Map<string, PatternDoc>;
}

export interface BeadV1Options {
  /** Migration source. Also the thing whose key gets deleted afterwards. */
  legacyStorage?: Storage | undefined;
  /** Overridable so tests can run each case against its own database. */
  databaseName?: string;
}

export function createBeadV1Repository(
  factory: IDBFactory,
  { legacyStorage, databaseName = DB_NAME }: BeadV1Options = {},
): Repository {
  let persistenceFailed = false;
  const listeners = new Set<() => void>();
  const memory: MemoryView = { state: { ...EMPTY_STATE }, patterns: new Map() };

  const markFailed = (): void => {
    if (persistenceFailed) return;
    persistenceFailed = true;
    for (const listener of [...listeners]) listener();
  };

  let opening: Promise<IDBDatabase | null> | null = null;

  /**
   * One lazy open for the session. A failed open is not retried: the second
   * attempt fails for the same reason and the memory view is already the whole
   * truth by then, so retrying would only churn.
   */
  const database = (): Promise<IDBDatabase | null> => {
    opening ??= openDatabase(factory, databaseName)
      .then(async (db) => {
        try {
          await migrateLegacyState(db, legacyStorage);
        } catch (error) {
          // The legacy key is still on disk and the new database is empty:
          // serving that empty database would look exactly like data loss.
          memory.state = parsePersistedState(readLegacy(legacyStorage));
          markFailed();
          db.close();
          throw error;
        }
        return db;
      })
      .catch(() => {
        markFailed();
        return null;
      });
    return opening;
  };

  // Writes are serialised so a read-modify-write cannot interleave with the
  // effect that produced the state it is rebasing on: `store.tsx` fires its
  // four save effects in order but does not await them, and `saveProgress`
  // prunes against whatever `projects` it can see.
  let tail: Promise<unknown> = Promise.resolve();
  const serialize = <T,>(task: () => Promise<T>): Promise<T> => {
    const next = tail.then(task, task);
    tail = next.catch(() => undefined);
    return next;
  };

  async function readArray<T>(key: string, parse: (value: unknown) => T[]): Promise<T[] | null> {
    const db = await database();
    if (db === null) return null;
    try {
      const tx = db.transaction(STORE.state, "readonly");
      return parse(await request(tx.objectStore(STORE.state).get(key)));
    } catch {
      markFailed();
      return null;
    }
  }

  async function writeArray(key: string, value: unknown): Promise<void> {
    const db = await database();
    if (db === null) return;
    try {
      const tx = db.transaction(STORE.state, "readwrite");
      tx.objectStore(STORE.state).put(value, key);
      await settled(tx);
    } catch {
      markFailed();
    }
  }

  async function currentProjects(): Promise<Project[]> {
    return (await readArray(STATE_KEY.projects, parseProjects)) ?? memory.state.projects;
  }

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
      const projects = await readArray(STATE_KEY.projects, parseProjects);
      if (projects === null) return memory.state.projects;
      memory.state = { ...memory.state, projects };
      return projects;
    },
    async saveProjects(projects) {
      memory.state = { ...memory.state, projects: [...projects] };
      await serialize(() => writeArray(STATE_KEY.projects, projects));
    },

    async loadFavorites() {
      const favorites = await readArray<PatternId>(STATE_KEY.favorites, parseFavorites);
      if (favorites === null) return memory.state.favorites;
      memory.state = { ...memory.state, favorites };
      return favorites;
    },
    async saveFavorites(favorites) {
      memory.state = { ...memory.state, favorites: [...favorites] };
      await serialize(() => writeArray(STATE_KEY.favorites, favorites));
    },

    async loadInventory() {
      const inventory = await readArray<InventoryEntry>(STATE_KEY.inventory, parseInventory);
      if (inventory === null) return memory.state.inventory;
      memory.state = { ...memory.state, inventory };
      return inventory;
    },
    async saveInventory(inventory) {
      memory.state = { ...memory.state, inventory: [...inventory] };
      await serialize(() => writeArray(STATE_KEY.inventory, inventory));
    },

    /**
     * §2.2: the array signature is the whole point of not touching `store.tsx`.
     * The array is a view over one row per project, assembled here.
     */
    async loadProgress() {
      const db = await database();
      if (db === null) return memory.state.progress;
      try {
        const tx = db.transaction(STORE.progress, "readonly");
        const rows = await request(tx.objectStore(STORE.progress).getAll());
        const progress: ProgressCursor[] = [];
        for (const row of rows) {
          const cursor = toProgressCursor(row);
          if (cursor !== null) progress.push(cursor);
        }
        memory.state = { ...memory.state, progress };
        return progress;
      } catch {
        markFailed();
        return memory.state.progress;
      }
    },
    async saveProgress(cursors) {
      return serialize(async () => {
        const projects = await currentProjects();
        const kept = sanitizeProgress(cursors, projects);
        memory.state = { ...memory.state, progress: kept };
        const db = await database();
        if (db === null) return;
        try {
          const tx = db.transaction(STORE.progress, "readwrite");
          const store = tx.objectStore(STORE.progress);
          const keep = new Set<string>(kept.map((cursor) => cursor.projectId));
          for (const key of await request(store.getAllKeys())) {
            if (!keep.has(String(key))) store.delete(key);
          }
          for (const cursor of kept) store.put(cursor);
          await settled(tx);
        } catch {
          markFailed();
        }
      });
    },

    async loadPatternDoc(id) {
      const db = await database();
      if (db === null) return memory.patterns.get(id) ?? null;
      try {
        const tx = db.transaction(STORE.patterns, "readonly");
        return toPatternDoc(await request(tx.objectStore(STORE.patterns).get(id)));
      } catch {
        markFailed();
        return memory.patterns.get(id) ?? null;
      }
    },

    /**
     * D-UP-10 awaits this one and mints nothing when it rejects, so unlike
     * every other write it must not swallow the failure: the banner goes up
     * (DATA-1) *and* the caller hears about it.
     */
    async savePatternDoc(doc) {
      const checked = toPatternDoc(doc);
      if (checked === null) throw new Error("豆图文档形状不合法，拒绝写入");
      return serialize(async () => {
        memory.patterns.set(checked.projectId, checked);
        const db = await database();
        if (db === null) {
          throw new Error("本地存储不可用，豆图没有保存");
        }
        try {
          const tx = db.transaction(STORE.patterns, "readwrite");
          tx.objectStore(STORE.patterns).put(checked);
          await settled(tx);
        } catch (error) {
          markFailed();
          throw error instanceof Error ? error : new Error("豆图写入失败");
        }
      });
    },

    /**
     * §5.3's one cross-store invariant, and the only reclamation path a
     * megabyte-scale pattern document has. The project record itself lives in
     * the `projects` array the reducer owns, so removing it is that reducer's
     * job — this is the part no page could do for itself.
     */
    async deleteProject(id) {
      return serialize(async () => {
        memory.patterns.delete(id);
        memory.state = {
          ...memory.state,
          progress: memory.state.progress.filter((cursor) => cursor.projectId !== id),
        };
        const db = await database();
        if (db === null) return;
        try {
          const tx = db.transaction([STORE.patterns, STORE.progress], "readwrite");
          tx.objectStore(STORE.patterns).delete(id);
          tx.objectStore(STORE.progress).delete(id);
          await settled(tx);
        } catch {
          markFailed();
        }
      });
    },
  };
}

/** Enumerates one store, for the tests that assert what did *not* reach disk. */
export async function readStoreEntries(
  factory: IDBFactory,
  storeName: string,
  databaseName: string = DB_NAME,
): Promise<{ key: string; value: unknown }[]> {
  const db = await openDatabase(factory, databaseName);
  try {
    const tx = db.transaction(storeName, "readonly");
    const store = tx.objectStore(storeName);
    const keys = await request(store.getAllKeys());
    const values = await request(store.getAll());
    return keys.map((key, index) => ({ key: String(key), value: values[index] }));
  } finally {
    db.close();
  }
}
