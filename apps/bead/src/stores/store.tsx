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
import { clampBeads, normalizeCode } from "./inventory.ts";
import type { PatternId, ProjectId } from "./ids.ts";
import {
  EMPTY_STATE,
  type BackdropKind,
  type InventoryEntry,
  type Pattern,
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
  | { kind: "setInventoryBeads"; code: string; beads: number }
  | { kind: "removeInventoryEntry"; code: string }
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
    // D-INV-6: the code is the identity key, so adding a code that is already
    // there is a no-op rather than a silent merge — the form reports it inline
    // and the user edits the existing row instead.
    case "addInventoryEntry": {
      const entry = {
        ...action.entry,
        code: normalizeCode(action.entry.code),
        beads: clampBeads(action.entry.beads),
      };
      if (state.inventory.some((existing) => existing.code === entry.code)) return state;
      return { ...state, inventory: [...state.inventory, entry] };
    }
    case "setInventoryBeads": {
      const code = normalizeCode(action.code);
      const beads = clampBeads(action.beads);
      return {
        ...state,
        inventory: state.inventory.map((entry) =>
          entry.code === code ? { ...entry, beads } : entry,
        ),
      };
    }
    case "removeInventoryEntry": {
      const code = normalizeCode(action.code);
      return { ...state, inventory: state.inventory.filter((entry) => entry.code !== code) };
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
  setProjectStatus(id: ProjectId, status: ProjectStatus): void;
  setProjectBackdrop(id: ProjectId, backdrop: BackdropKind, color?: string): void;
  toggleFavorite(id: PatternId): void;
  /** D-INV-5: three narrow inventory actions, all of them driven by `/inventory`. */
  addInventoryEntry(entry: InventoryEntry): void;
  setInventoryBeads(code: string, beads: number): void;
  removeInventoryEntry(code: string): void;
  /** BD19: one cursor per project, overwritten in place. Never called per timer tick. */
  upsertProgress(cursor: ProgressCursorInput): void;
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

  const setInventoryBeads = useCallback<StoreActions["setInventoryBeads"]>((code, beads) => {
    dispatch({ kind: "setInventoryBeads", code, beads });
  }, []);

  const removeInventoryEntry = useCallback<StoreActions["removeInventoryEntry"]>((code) => {
    dispatch({ kind: "removeInventoryEntry", code });
  }, []);

  const upsertProgress = useCallback<StoreActions["upsertProgress"]>((cursor) => {
    dispatch({ kind: "upsertProgress", cursor: { ...cursor, updatedAt: Date.now() } });
  }, []);

  const value = useMemo<StoreValue>(
    () => ({
      ...state,
      persistenceFailed,
      instantiatePattern,
      setProjectStatus,
      setProjectBackdrop,
      toggleFavorite,
      addInventoryEntry,
      setInventoryBeads,
      removeInventoryEntry,
      upsertProgress,
    }),
    [
      state,
      persistenceFailed,
      instantiatePattern,
      setProjectStatus,
      setProjectBackdrop,
      toggleFavorite,
      addInventoryEntry,
      setInventoryBeads,
      removeInventoryEntry,
      upsertProgress,
    ],
  );

  return <StoreContext.Provider value={value}>{children}</StoreContext.Provider>;
}

export function useStore(): StoreValue {
  const value = useContext(StoreContext);
  if (value === null) throw new Error("useStore 必须在 <StoreProvider> 内使用");
  return value;
}
