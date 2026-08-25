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
  | { kind: "setInventory"; entries: InventoryEntry[] }
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
    case "setInventory":
      return { ...state, inventory: action.entries };
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
  setInventory(entries: InventoryEntry[]): void;
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

  const setInventory = useCallback<StoreActions["setInventory"]>((entries) => {
    dispatch({ kind: "setInventory", entries });
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
      setInventory,
      upsertProgress,
    }),
    [
      state,
      persistenceFailed,
      instantiatePattern,
      setProjectStatus,
      setProjectBackdrop,
      toggleFavorite,
      setInventory,
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
