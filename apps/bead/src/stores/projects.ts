import { mintProjectId, type PatternId, type ProjectId } from "./ids.ts";
import type { Pattern, Project, ProjectStatus } from "./types.ts";

// Derived numbers (counts, section splits) are computed here on every read.
// Section 5 of the IA review forbids persisting derived data.

export const DEFAULT_BACKDROP_COLOR = "#101014";

export function createProjectFromPattern(
  pattern: Pattern,
  status: Extract<ProjectStatus, "todo" | "active">,
  now: number = Date.now(),
): Project {
  return {
    id: mintProjectId(now),
    title: pattern.title,
    sourcePatternId: pattern.id,
    status,
    createdAt: now,
    backdrop: "black",
    backdropColor: DEFAULT_BACKDROP_COLOR,
  };
}

/**
 * D-UP-11: a converted upload has no gallery pattern behind it, so
 * `sourcePatternId` is null and the `PatternDoc` in the `patterns` store is the
 * whole source of its grid. No `kind` field joins the record to say so — null
 * source plus a document that loads is already the discriminator, and a second
 * authority for the same fact is a second thing to keep true.
 *
 * The id is minted by the caller because the document has to be written under
 * it *before* the project exists (D-UP-10).
 */
export function createProjectFromConversion(
  id: ProjectId,
  title: string,
  status: Extract<ProjectStatus, "todo" | "active">,
  now: number = Date.now(),
): Project {
  return {
    id,
    title,
    sourcePatternId: null,
    status,
    createdAt: now,
    backdrop: "black",
    backdropColor: DEFAULT_BACKDROP_COLOR,
  };
}

export function selectProject(projects: readonly Project[], id: string): Project | undefined {
  return projects.find((project) => project.id === (id as ProjectId));
}

/** Workspace 正在拼 / PersonalStrip 进度卡 both read this ordering. */
export function selectInProgress(projects: readonly Project[]): Project[] {
  return projects
    .filter((project) => project.status === "active" || project.status === "todo")
    .sort((a, b) => b.createdAt - a.createdAt);
}

export function selectActiveProject(projects: readonly Project[]): Project | undefined {
  return selectInProgress(projects)[0];
}

export function selectDrafts(projects: readonly Project[]): Project[] {
  return projects
    .filter((project) => project.status === "draft")
    .sort((a, b) => b.createdAt - a.createdAt);
}

export function selectFinished(projects: readonly Project[]): Project[] {
  return projects
    .filter((project) => project.status === "done")
    .sort((a, b) => b.createdAt - a.createdAt);
}

export function selectTodoCount(projects: readonly Project[]): number {
  return projects.filter((project) => project.status === "todo").length;
}

export function isFavorite(favorites: readonly PatternId[], id: PatternId): boolean {
  return favorites.includes(id);
}
