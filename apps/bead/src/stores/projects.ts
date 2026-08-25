import { mintProjectId, type PatternId, type ProjectId } from "./ids.ts";
import type { Pattern, Project, ProjectStatus } from "./types.ts";

// Derived numbers (counts, section splits) are computed here on every read.
// Section 5 of the IA review forbids persisting derived data.

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
    backdropColor: "#101014",
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
