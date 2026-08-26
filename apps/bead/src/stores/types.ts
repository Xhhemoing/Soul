import type { ImageKind } from "../algo/classify.ts";
import type { SplitMode } from "../algo/steps.ts";
import type { CreatorId, PatternId, ProjectId } from "./ids.ts";

/**
 * BD20: colour codes are only unique inside a palette. The gallery fixtures and
 * `generic-5mm` really do both have a `G07`, and they are different beads
 * (苔绿 #4c7a44 vs Silver #b7bfc6), so every code that reaches inventory, a
 * requirement row or the purchase text travels with the namespace it came from.
 *
 * Closed union in v0: brand palettes are BD9 and not this work package.
 */
export const PALETTE_NAMESPACES = ["gallery", "generic-5mm"] as const;

export type PaletteNamespaceId = (typeof PALETTE_NAMESPACES)[number];

export const GALLERY_PALETTE: PaletteNamespaceId = "gallery";
export const GENERIC_5MM_PALETTE: PaletteNamespaceId = "generic-5mm";

/** The label that goes next to a code wherever one is shown. */
export const PALETTE_NAMESPACE_LABEL: Record<PaletteNamespaceId, string> = {
  gallery: "画廊",
  "generic-5mm": "通用5mm",
};

export function isPaletteNamespaceId(value: unknown): value is PaletteNamespaceId {
  return typeof value === "string" && (PALETTE_NAMESPACES as readonly string[]).includes(value);
}

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
  /** BD20: `(paletteId, code)` is the identity key, not `code` alone. */
  paletteId: PaletteNamespaceId;
  code: string;
  name: string;
  hex: string;
  beads: number;
}

/**
 * BD19: the only assemble progress that reaches storage. The key set is exactly
 * these five — `Step[]`, `Grid` and anything else derived stay out. The grid
 * itself now has a home of its own (`PatternDoc` below, in the `patterns`
 * store), and the shape here is unchanged by that move.
 */
export interface ProgressCursor {
  projectId: ProjectId;
  mode: SplitMode;
  /** 0-based; consumers clamp it into `[0, steps.length]`. */
  stepIndex: number;
  elapsedMs: number;
  updatedAt: number;
}

/**
 * round2-data §5.2. `doneBits` is the per-cell completion bitmap the contract
 * reserves for a splitter-independent progress model; nothing writes it in v0
 * (D-UP-13), so it is optional and readers must cope with its absence.
 */
export interface ProgressDoc extends ProgressCursor {
  doneBits?: Uint8Array;
}

/** Where a converted grid came from. Display-only, never an input to the pipeline. */
export interface PatternProvenance {
  kind: ImageKind;
  ditherApplied: boolean;
}

/**
 * round2-data §5.2: the one document that carries a grid to disk.
 *
 * `cells` is row-major `Int16Array` with `-1` for an empty cell — structured
 * clone stores it as a typed array rather than 3136 JSON numbers, and 56×56
 * lands at about 6.3KB. `paletteId` is `generic-5mm` for every v0 document;
 * it is written down anyway so a later palette cannot be inferred from silence.
 */
export interface PatternDoc {
  projectId: ProjectId;
  paletteId: PaletteNamespaceId;
  width: number;
  height: number;
  cells: Int16Array;
  provenance?: PatternProvenance;
}

export interface PersistedState {
  projects: Project[];
  favorites: PatternId[];
  inventory: InventoryEntry[];
  progress: ProgressCursor[];
}

export const EMPTY_STATE: PersistedState = {
  projects: [],
  favorites: [],
  inventory: [],
  progress: [],
};
