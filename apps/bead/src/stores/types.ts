import type { CreatorId, PatternId, ProjectId } from "./ids.ts";

export type BoardKind = "square-28" | "square-56" | "hex" | "round";

export interface PaletteEntry {
  /** Brand colour code. Always rendered next to the swatch — colour is never the only signal. */
  code: string;
  name: string;
  hex: string;
  beads: number;
}

export interface Pattern {
  id: PatternId;
  title: string;
  creatorId: CreatorId;
  tags: string[];
  /** 1–5. */
  difficulty: number;
  beadCount: number;
  estimatedMinutes: number;
  board: BoardKind;
  palette: PaletteEntry[];
}

export interface Creator {
  id: CreatorId;
  name: string;
  bio: string;
  /** 被拼打卡数 */
  checkIns: number;
}

/**
 * `todo` and `active` both show up under Workspace 正在拼; `todo` is what
 * 「加入待拼」 mints, `active` is what 「转入工作台」 mints.
 */
export type ProjectStatus = "todo" | "active" | "draft" | "done";

/**
 * D-UI-6: the assemble backdrop is a per-project canvas colour stored on the
 * project record. It is not the app theme and must never read from it.
 */
export type BackdropKind = "black" | "white" | "custom";

export interface Project {
  id: ProjectId;
  title: string;
  /** Null for blank projects created outside a gallery pattern. */
  sourcePatternId: PatternId | null;
  status: ProjectStatus;
  createdAt: number;
  backdrop: BackdropKind;
  /** Only meaningful when `backdrop === "custom"`. */
  backdropColor: string;
}

export interface InventoryEntry {
  code: string;
  name: string;
  hex: string;
  beads: number;
}

export interface PersistedState {
  projects: Project[];
  favorites: PatternId[];
  inventory: InventoryEntry[];
}

export const EMPTY_STATE: PersistedState = {
  projects: [],
  favorites: [],
  inventory: [],
};
