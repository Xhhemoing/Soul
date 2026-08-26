import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useSyncExternalStore,
  type ReactNode,
} from "react";

import { createRepository, type Repository } from "./repository.ts";
import { createProjectFromPattern } from "./projects.ts";
import { clampBeads, normalizeCode, paletteCodeKey } from "./inventory.ts";
import type { PatternId, ProjectId } from "./ids.ts";
import {
  EMPTY_STATE,
  type BackdropKind,
  type InventoryEntry,
  type PaletteNamespaceId,
  type Pattern,
  type PatternDoc,
  type PersistedState,
  type ProgressCursor,
  type Project,
  type ProjectStatus,
} from "./types.ts";

/** `updatedAt` is stamped by the action, never by the caller. */
export type ProgressCursorInput = Omit<ProgressCursor, "updatedAt">;

interface StoreState extends PersistedState {
  hydrated: boolean;
}

interface PersistenceView {
  /** DATA-1: true once the repository has stopped persisting. Not reducer state. */
  persistenceFailed: boolean;
}

type Action =
  | { kind: "hydrated"; state: PersistedState }
  | { kind: "addProject"; project: Project }
  | { kind: "setProjectStatus"; id: ProjectId; status: ProjectStatus }
  | { kind: "setBackdrop"; id: ProjectId; backdrop: BackdropKind; color?: string }
  | { kind: "toggleFavorite"; id: PatternId }
  | { kind: "addInventoryEntry"; entry: InventoryEntry }
  | { kind: "setInventoryBeads"; paletteId: PaletteNamespaceId; code: string; beads: number }
  | { kind: "removeInventoryEntry"; paletteId: PaletteNamespaceId; code: string }
  | { kind: "upsertProgress"; cursor: ProgressCursor };

function reduce(state: StoreState, action: Action): StoreState {
  switch (action.kind) {
    case "hydrated":
      return { ...action.state, hydrated: true };
    case "addProject":
      return { ...state, projects: [action.project, ...state.projects] };
    case "setProjectStatus":
      return {
        ...state,
        projects: state.projects.map((project) =>
          project.id === action.id ? { ...project, status: action.status } : project,
        ),
      };
    case "setBackdrop":
      return {
        ...state,
        projects: state.projects.map((project) =>
          project.id === action.id
            ? {
                ...project,
                backdrop: action.backdrop,
                backdropColor: action.color ?? project.backdropColor,
              }
            : project,
        ),
      };
    case "toggleFavorite":
      return {
        ...state,
        favorites: state.favorites.includes(action.id)
          ? state.favorites.filter((id) => id !== action.id)
          : [...state.favorites, action.id],
      };
    // D-INV-6 with BD20's key: `(paletteId, code)` is the identity, so adding a
    // pair that is already there is a no-op rather than a silent merge — the
    // form reports it inline and the user edits the existing row instead. The
    // same code in the other namespace is a different bead and goes in.
    case "addInventoryEntry": {
      const entry = {
        ...action.entry,
        code: normalizeCode(action.entry.code),
        beads: clampBeads(action.entry.beads),
      };
      const key = paletteCodeKey(entry);
      if (state.inventory.some((existing) => paletteCodeKey(existing) === key)) return state;
      return { ...state, inventory: [...state.inventory, entry] };
    }
    case "setInventoryBeads": {
      const key = paletteCodeKey({ paletteId: action.paletteId, code: normalizeCode(action.code) });
      const beads = clampBeads(action.beads);
      return {
        ...state,
        inventory: state.inventory.map((entry) =>
          paletteCodeKey(entry) === key ? { ...entry, beads } : entry,
        ),
      };
    }
    case "removeInventoryEntry": {
      const key = paletteCodeKey({ paletteId: action.paletteId, code: normalizeCode(action.code) });
      return {
        ...state,
        inventory: state.inventory.filter((entry) => paletteCodeKey(entry) !== key),
      };
    }
    case "upsertProgress": {
      const others = state.progress.filter(
        (cursor) => cursor.projectId !== action.cursor.projectId,
      );
      return { ...state, progress: [...others, action.cursor] };
    }
  }
}

export interface StoreActions {
  /** Returns the minted project so the caller can navigate straight to it. */
  instantiatePattern(pattern: Pattern, status: Extract<ProjectStatus, "todo" | "active">): Project;
  /**
   * D-UP-10: the second half of the conversion save. The caller has already
   * awaited `savePatternDoc` for the id this project carries, so a project only
   * ever appears once its document is on disk.
   */
  addProject(project: Project): void;
  setProjectStatus(id: ProjectId, status: ProjectStatus): void;
  setProjectBackdrop(id: ProjectId, backdrop: BackdropKind, color?: string): void;
  toggleFavorite(id: PatternId): void;
  /** D-INV-5: three narrow inventory actions, all of them driven by `/inventory`. */
  addInventoryEntry(entry: InventoryEntry): void;
  setInventoryBeads(paletteId: PaletteNamespaceId, code: string, beads: number): void;
  removeInventoryEntry(paletteId: PaletteNamespaceId, code: string): void;
  /** BD19: one cursor per project, overwritten in place. Never called per timer tick. */
  upsertProgress(cursor: ProgressCursorInput): void;
  /**
   * The `patterns` store, reached through the same provider every other page
   * already has. Grids are too big to sit in reducer state, so these stay
   * promises and their callers own the loading state (D-UP-14 / §4.3).
   */
  loadPatternDoc(id: ProjectId): Promise<PatternDoc | null>;
  savePatternDoc(doc: PatternDoc): Promise<void>;
}

export type StoreValue = StoreState & PersistenceView & StoreActions;

const StoreContext = createContext<StoreValue | null>(null);

export function StoreProvider({
  children,
  repository,
}: {
  children: ReactNode;
  repository?: Repository;
}) {
  const repo = useMemo(() => repository ?? createRepository(), [repository]);
  const [state, dispatch] = useReducer(reduce, { ...EMPTY_STATE, hydrated: false });
  const hydratedRef = useRef(false);
  const persistenceFailed = useSyncExternalStore(
    repo.subscribeToPersistence,
    repo.isPersistenceFailed,
  );

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const [projects, favorites, inventory, progress] = await Promise.all([
        repo.loadProjects(),
        repo.loadFavorites(),
        repo.loadInventory(),
        repo.loadProgress(),
      ]);
      if (cancelled) return;
      hydratedRef.current = true;
      dispatch({ kind: "hydrated", state: { projects, favorites, inventory, progress } });
    })();
    return () => {
      cancelled = true;
    };
  }, [repo]);

  // Writing back only after hydration keeps a slow load from clobbering the
  // stored state with the empty initial value.
  useEffect(() => {
    if (!state.hydrated) return;
    void repo.saveProjects(state.projects);
  }, [repo, state.hydrated, state.projects]);

  useEffect(() => {
    if (!state.hydrated) return;
    void repo.saveFavorites(state.favorites);
  }, [repo, state.hydrated, state.favorites]);

  useEffect(() => {
    if (!state.hydrated) return;
    void repo.saveInventory(state.inventory);
  }, [repo, state.hydrated, state.inventory]);

  useEffect(() => {
    if (!state.hydrated) return;
    void repo.saveProgress(state.progress);
  }, [repo, state.hydrated, state.progress]);

  const instantiatePattern = useCallback<StoreActions["instantiatePattern"]>((pattern, status) => {
    const project = createProjectFromPattern(pattern, status);
    dispatch({ kind: "addProject", project });
    return project;
  }, []);

  const addProject = useCallback<StoreActions["addProject"]>((project) => {
    dispatch({ kind: "addProject", project });
  }, []);

  const setProjectStatus = useCallback<StoreActions["setProjectStatus"]>((id, status) => {
    dispatch({ kind: "setProjectStatus", id, status });
  }, []);

  const setProjectBackdrop = useCallback<StoreActions["setProjectBackdrop"]>(
    (id, backdrop, color) => {
      dispatch(color === undefined ? { kind: "setBackdrop", id, backdrop } : { kind: "setBackdrop", id, backdrop, color });
    },
    [],
  );

  const toggleFavorite = useCallback<StoreActions["toggleFavorite"]>((id) => {
    dispatch({ kind: "toggleFavorite", id });
  }, []);

  const addInventoryEntry = useCallback<StoreActions["addInventoryEntry"]>((entry) => {
    dispatch({ kind: "addInventoryEntry", entry });
  }, []);

  const setInventoryBeads = useCallback<StoreActions["setInventoryBeads"]>(
    (paletteId, code, beads) => {
      dispatch({ kind: "setInventoryBeads", paletteId, code, beads });
    },
    [],
  );

  const removeInventoryEntry = useCallback<StoreActions["removeInventoryEntry"]>(
    (paletteId, code) => {
      dispatch({ kind: "removeInventoryEntry", paletteId, code });
    },
    [],
  );

  const upsertProgress = useCallback<StoreActions["upsertProgress"]>((cursor) => {
    dispatch({ kind: "upsertProgress", cursor: { ...cursor, updatedAt: Date.now() } });
  }, []);

  const loadPatternDoc = useCallback<StoreActions["loadPatternDoc"]>(
    (id) => repo.loadPatternDoc(id),
    [repo],
  );

  const savePatternDoc = useCallback<StoreActions["savePatternDoc"]>(
    (doc) => repo.savePatternDoc(doc),
    [repo],
  );

  const value = useMemo<StoreValue>(
    () => ({
      ...state,
      persistenceFailed,
      instantiatePattern,
      addProject,
      setProjectStatus,
      setProjectBackdrop,
      toggleFavorite,
      addInventoryEntry,
      setInventoryBeads,
      removeInventoryEntry,
      upsertProgress,
      loadPatternDoc,
      savePatternDoc,
    }),
    [
      state,
      persistenceFailed,
      instantiatePattern,
      addProject,
      setProjectStatus,
      setProjectBackdrop,
      toggleFavorite,
      addInventoryEntry,
      setInventoryBeads,
      removeInventoryEntry,
      upsertProgress,
      loadPatternDoc,
      savePatternDoc,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore(): StoreValue {
  const value = useContext(StoreContext);
  if (value === null) throw new Error("useStore 必须在 <StoreProvider> 内使用");
  return value;
}
